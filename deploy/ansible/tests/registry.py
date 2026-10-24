"""Minimal stand-in for ghcr.io's anonymous pull API (OCI distribution).

GET /token?scope=...                  -> {"token": "anon"}
GET /v2/<repo>/manifests/<tag>        -> ROOT/<tag>.json (Bearer anon required)
GET /v2/<repo>/blobs/sha256:<hex>     -> 307 to http://localhost:<port>/storage/<hex>
                                         (Bearer anon required; other host name)
GET /storage/<hex>                    -> the blob; 400 if an Authorization
                                         header is sent (like the pre-signed
                                         storage URLs behind ghcr.io)

Usage: registry.py ROOT [REPO]   (REPO defaults to the role's pili_registry_repo;
port PILI_REGISTRY_PORT, default 8000)

Every request value is checked against a strict allow-list without regular
expressions (linear time), the file it selects must resolve inside ROOT, and
no request value reaches a response header unchecked. Tests: test_registry.py.
"""
import json
import os
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

DEFAULT_REPO = "nuagemagique-dev/pili-pos"
HEX = frozenset("0123456789abcdef")
LOWER_ALNUM = frozenset("abcdefghijklmnopqrstuvwxyz0123456789")
TAG_FIRST = frozenset("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_")
TAG_REST = TAG_FIRST | frozenset(".-")


def valid_repo(name):
    """OCI repository name: lowercase alphanumeric components joined by one
    of . _ - inside a path component and by / between components."""
    if not name or len(name) > 255:
        return False
    for part in name.split("/"):
        if not part or part[0] not in LOWER_ALNUM or part[-1] not in LOWER_ALNUM:
            return False
        prev_sep = False
        for c in part:
            if c in LOWER_ALNUM:
                prev_sep = False
            elif c in "._-" and not prev_sep:
                prev_sep = True
            else:
                return False
    return True


def valid_tag(tag):
    """OCI tag: [A-Za-z0-9_][A-Za-z0-9._-]{0,127}."""
    return 0 < len(tag) <= 128 and tag[0] in TAG_FIRST and set(tag[1:]) <= TAG_REST


def valid_digest_hex(s):
    return len(s) == 64 and set(s) <= HEX


def inside(root, *parts):
    """The resolved path of root/parts if it stays inside root, else None."""
    base = os.path.realpath(root)
    path = os.path.realpath(os.path.join(base, *parts))
    if os.path.commonpath([base, path]) != base or not os.path.isfile(path):
        return None
    return path


def make_handler(root, repo):
    if not valid_repo(repo):
        raise ValueError("invalid repository name: %r" % repo)
    prefix = "/v2/" + repo + "/"

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def send(self, code, body=b"", ctype="application/json", location=None):
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            if location is not None:
                # Built only from a checked digest: never CR, LF or anything else.
                if "\r" in location or "\n" in location:
                    raise ValueError("header value with CR or LF")
                self.send_header("Location", location)
            self.end_headers()
            self.wfile.write(body)

        def authed(self):
            return self.headers.get("Authorization") == "Bearer anon"

        def serve_file(self, path, ctype):
            with open(path, "rb") as f:
                self.send(200, f.read(), ctype)

        def do_GET(self):
            path = self.path.split("?", 1)[0]
            if "\r" in path or "\n" in path or "\\" in path:
                return self.send(400)
            if path == "/token":
                return self.send(200, json.dumps({"token": "anon"}).encode())

            if path.startswith("/v2/"):
                if not path.startswith(prefix):
                    return self.send(404)
                kind, sep, ref = path[len(prefix):].partition("/")
                if not sep or kind not in ("manifests", "blobs"):
                    return self.send(404)
                if not self.authed():
                    return self.send(401)
                if kind == "manifests":
                    if not valid_tag(ref):
                        return self.send(404)
                    f = inside(root, ref + ".json")
                    if f is None:
                        return self.send(404)
                    return self.serve_file(f, "application/vnd.oci.image.manifest.v1+json")
                algo, _, digest = ref.partition(":")
                if algo != "sha256" or not valid_digest_hex(digest):
                    return self.send(404)
                if inside(root, "blobs", digest) is None:
                    return self.send(404)
                # Another host name, as ghcr.io's redirect to its storage:
                # curl must drop the Authorization header there.
                port = self.server.server_address[1]
                return self.send(307, location="http://localhost:%d/storage/%s" % (port, digest))

            if path.startswith("/storage/"):
                if self.headers.get("Authorization"):
                    return self.send(400, b"Only one auth mechanism allowed")
                digest = path[len("/storage/"):]
                if not valid_digest_hex(digest):
                    return self.send(404)
                f = inside(root, "blobs", digest)
                if f is None:
                    return self.send(404)
                return self.serve_file(f, "application/octet-stream")
            self.send(404)

    return Handler


def make_server(root, repo=DEFAULT_REPO, port=8000):
    return ThreadingHTTPServer(("127.0.0.1", port), make_handler(root, repo))


if __name__ == "__main__":
    make_server(
        sys.argv[1],
        sys.argv[2] if len(sys.argv) > 2 else DEFAULT_REPO,
        int(os.environ.get("PILI_REGISTRY_PORT", "8000")),
    ).serve_forever()
