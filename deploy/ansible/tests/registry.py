"""Minimal stand-in for ghcr.io's anonymous pull API (OCI distribution).

GET /token?scope=...                  -> {"token": "anon"}
GET /v2/<repo>/manifests/<tag>        -> /srv/registry/<tag>.json (Bearer anon required)
GET /v2/<repo>/blobs/sha256:<hex>     -> 307 to http://localhost:8000/storage/<hex>
                                         (Bearer anon required; other host name)
GET /storage/<hex>                    -> the blob; 400 if an Authorization
                                         header is sent (like the pre-signed
                                         storage URLs behind ghcr.io)
"""
import json
import os
import re
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = sys.argv[1]


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def send(self, code, body=b"", ctype="application/json", headers=None):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        for k, v in (headers or {}).items():
            self.send_header(k, v)
        self.end_headers()
        self.wfile.write(body)

    def authed(self):
        return self.headers.get("Authorization") == "Bearer anon"

    def do_GET(self):
        path = self.path.split("?")[0]
        if path == "/token":
            return self.send(200, json.dumps({"token": "anon"}).encode())
        m = re.fullmatch(r"/v2/(.+)/manifests/([\w.-]+)", path)
        if m:
            if not self.authed():
                return self.send(401)
            f = os.path.join(ROOT, m.group(2) + ".json")
            if not os.path.exists(f):
                return self.send(404)
            return self.send(200, open(f, "rb").read(), "application/vnd.oci.image.manifest.v1+json")
        m = re.fullmatch(r"/v2/(.+)/blobs/sha256:([0-9a-f]{64})", path)
        if m:
            if not self.authed():
                return self.send(401)
            # Another host name, as ghcr.io's redirect to its storage:
            # curl must drop the Authorization header there.
            return self.send(307, headers={"Location": "http://localhost:8000/storage/" + m.group(2)})
        m = re.fullmatch(r"/storage/([0-9a-f]{64})", path)
        if m:
            if self.headers.get("Authorization"):
                return self.send(400, b"Only one auth mechanism allowed")
            f = os.path.join(ROOT, "blobs", m.group(1))
            if not os.path.exists(f):
                return self.send(404)
            return self.send(200, open(f, "rb").read(), "application/octet-stream")
        self.send(404)


ThreadingHTTPServer(("127.0.0.1", 8000), Handler).serve_forever()
