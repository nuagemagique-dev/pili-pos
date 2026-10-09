#!/usr/bin/env python3
"""OCI-like registry mock for the platform end-to-end run (P6-T11).

Serves the declared Android release (`declared.json`, `declared.json.minisig` and the APK) from a
directory, the way ghcr.io serves the artifact tag `declared-android`, so that the debug app on the
emulator can run its whole update path against the host (through `adb reverse`).

  GET /token[?scope=..]                          -> {"token":"t"}
  GET /v2/<repo>/manifests/<tag>                 -> OCI image manifest (needs "Authorization: Bearer t")
  GET /v2/<repo>/blobs/sha256:<hex>              -> the blob (needs the bearer)

The manifest is rebuilt from the directory at every request, so the files may be swapped while the mock
runs. Every regular file of the directory is a layer: `org.opencontainers.image.title` is its file name,
`digest` its sha256 and `size` its size; the layers are sorted by title.

With --redirect-port the blob requests of the main port answer one 307 whose Location points at a second
listener (the "CDN"), which serves the same blob. The second listener asks for no bearer and logs whether
the request carried one: the app must not forward its token there. --redirect-location-port is the port
written in the Location (the port the device sees through adb reverse); it defaults to --redirect-port.

Every request is logged as one line, to stdout and to --log:
    REQ <listener> <GET path> auth=<yes|no> -> <status>
where <listener> is `main` or `cdn`.

Usage:
    python3 -I scripts/android/plat_registry_mock.py --dir DIR --port N [--redirect-port N]
        [--redirect-location-port N] [--log FILE]

Standard library only.
"""
import argparse
import hashlib
import http.server
import json
import os
import re
import socketserver
import sys
import threading

TITLE_ANNOTATION = "org.opencontainers.image.title"
MANIFEST_TYPE = "application/vnd.oci.image.manifest.v1+json"
BEARER = "Bearer t"
DIGEST_RE = re.compile(r"^sha256:([0-9a-f]{64})$")
MANIFEST_RE = re.compile(r"^/v2/([A-Za-z0-9._/-]+)/manifests/([A-Za-z0-9._-]+)$")
BLOB_RE = re.compile(r"^/v2/([A-Za-z0-9._/-]+)/blobs/(sha256:[0-9a-f]{64})$")
CDN_RE = re.compile(r"^/cdn/(sha256:[0-9a-f]{64})$")


class Store:
    """The files of the registry directory, with their digests (cached per mtime and size)."""

    def __init__(self, directory):
        self.directory = directory
        self._cache = {}
        self._lock = threading.Lock()

    def layers(self):
        """[(title, digest, size, path)] of the regular files, sorted by title."""
        found = []
        try:
            names = sorted(os.listdir(self.directory))
        except OSError:
            return found
        for name in names:
            path = os.path.join(self.directory, name)
            if not os.path.isfile(path):
                continue
            stat = os.stat(path)
            key = (path, stat.st_mtime_ns, stat.st_size)
            with self._lock:
                digest = self._cache.get(key)
                if digest is None:
                    digest = "sha256:" + sha256_file(path)
                    self._cache = {k: v for k, v in self._cache.items() if k[0] != path}
                    self._cache[key] = digest
            found.append((name, digest, stat.st_size, path))
        return found

    def blob(self, digest):
        for _title, d, size, path in self.layers():
            if d == digest:
                return path, size
        return None

    def manifest(self):
        layers = [
            {
                "mediaType": "application/octet-stream",
                "digest": digest,
                "size": size,
                "annotations": {TITLE_ANNOTATION: title},
            }
            for title, digest, size, _path in self.layers()
        ]
        config = b"{}"
        return {
            "schemaVersion": 2,
            "mediaType": MANIFEST_TYPE,
            "artifactType": "application/vnd.pili-pos.declared-android.v1",
            "config": {
                "mediaType": "application/vnd.oci.empty.v1+json",
                "digest": "sha256:" + hashlib.sha256(config).hexdigest(),
                "size": len(config),
            },
            "layers": layers,
        }


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


class Log:
    def __init__(self, path, echo=True):
        self.path = path
        self.echo = echo
        self.lines = []
        self._lock = threading.Lock()

    def add(self, listener, request, auth, status):
        line = "REQ %s %s auth=%s -> %d" % (listener, request, "yes" if auth else "no", status)
        with self._lock:
            self.lines.append(line)
            if self.echo:
                print(line, flush=True)
            if self.path:
                with open(self.path, "a", encoding="utf-8") as fh:
                    fh.write(line + "\n")


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "plat-registry-mock"
    # set per server class
    listener = "main"

    def log_message(self, *_args):  # the Log object prints one line per request
        pass

    def _log_request(self, status):
        # logged before the response goes out, so a client that has the response can already read the line
        self.server.log.add(self.listener, "GET " + self._path, self._auth is not None, status)

    def _send(self, status, body=b"", content_type="application/octet-stream", extra=None):
        self._log_request(status)
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        for key, value in (extra or {}).items():
            self.send_header(key, value)
        self.end_headers()
        self.wfile.write(body)

    def _send_file(self, path, size):
        self._log_request(200)
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(size))
        self.end_headers()
        with open(path, "rb") as fh:
            for chunk in iter(lambda: fh.read(1 << 16), b""):
                self.wfile.write(chunk)

    def do_GET(self):  # noqa: N802 (http.server API)
        auth = self.headers.get("Authorization")
        path = self.path.split("?", 1)[0]
        self._auth = auth
        self._path = path
        self._route(path, auth)

    def _route(self, path, auth):
        store = self.server.store
        if self.listener == "cdn":
            m = CDN_RE.match(path)
            blob = store.blob(m.group(1)) if m else None
            if blob is None:
                self._send(404, b"not found")
                return 404
            self._send_file(*blob)
            return 200
        if path == "/token":
            self._send(200, json.dumps({"token": "t"}).encode(), "application/json")
            return 200
        m = MANIFEST_RE.match(path)
        b = BLOB_RE.match(path)
        if not (m or b):
            self._send(404, b"not found")
            return 404
        if auth != BEARER:
            self._send(401, b"unauthorized", extra={"WWW-Authenticate": 'Bearer realm="/token"'})
            return 401
        if m:
            self._send(200, json.dumps(store.manifest(), sort_keys=True).encode(), MANIFEST_TYPE)
            return 200
        blob = store.blob(b.group(2))
        if blob is None:
            self._send(404, b"no such blob")
            return 404
        if self.server.redirect_location_port is not None:
            location = "http://127.0.0.1:%d/cdn/%s" % (self.server.redirect_location_port, b.group(2))
            self._send(307, b"", extra={"Location": location})
            return 307
        self._send_file(*blob)
        return 200


class CdnHandler(Handler):
    listener = "cdn"


class Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def __init__(self, address, handler, store, log, redirect_location_port=None):
        super().__init__(address, handler)
        self.store = store
        self.log = log
        self.redirect_location_port = redirect_location_port


def make_servers(directory, port=0, redirect_port=None, redirect_location_port=None, log_path=None, echo=True):
    """Return (main, cdn_or_None, log). The servers are bound, not yet serving."""
    store = Store(directory)
    log = Log(log_path, echo)
    cdn = None
    if redirect_port is not None:
        cdn = Server(("127.0.0.1", redirect_port), CdnHandler, store, log)
        if redirect_location_port is None:
            redirect_location_port = cdn.server_address[1]
    main = Server(("127.0.0.1", port), Handler, store, log, redirect_location_port if cdn else None)
    return main, cdn, log


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--dir", required=True, help="directory holding the files of the artifact")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--redirect-port", type=int, default=None)
    parser.add_argument("--redirect-location-port", type=int, default=None)
    parser.add_argument("--log", default=None, help="append the request lines to this file")
    args = parser.parse_args(argv)
    if not os.path.isdir(args.dir):
        print("plat_registry_mock: not a directory: %s" % args.dir, file=sys.stderr)
        return 2
    main_server, cdn, _log = make_servers(
        args.dir, args.port, args.redirect_port, args.redirect_location_port, args.log
    )
    if cdn is not None:
        threading.Thread(target=cdn.serve_forever, daemon=True).start()
    print("plat_registry_mock: listening on 127.0.0.1:%d" % main_server.server_address[1], flush=True)
    try:
        main_server.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
