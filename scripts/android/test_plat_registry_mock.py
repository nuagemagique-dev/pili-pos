#!/usr/bin/env python3
"""Self-test of plat_registry_mock.py. Run: python3 -I scripts/android/test_plat_registry_mock.py"""
import hashlib
import http.client
import importlib.util
import json
import os
import shutil
import tempfile
import threading
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SPEC = importlib.util.spec_from_file_location("plat_registry_mock", os.path.join(HERE, "plat_registry_mock.py"))
MOCK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MOCK)

REPO = "brunomoyse/moyun-pos"
APK = b"PK\x03\x04 not really an apk " * 100


def sha(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def get(port, path, headers=None):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=10)
    try:
        conn.request("GET", path, headers=headers or {})
        resp = conn.getresponse()
        return resp.status, dict(resp.getheaders()), resp.read()
    finally:
        conn.close()


class MockTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp(prefix="plat-registry-test.")
        self.addCleanup(shutil.rmtree, self.dir, True)
        self.files = {
            "declared.json": b'{"format":1}\n',
            "declared.json.minisig": b"untrusted comment: x\nRW\n",
            "pili-pos-android-1.0.1.apk": APK,
        }
        for name, data in self.files.items():
            with open(os.path.join(self.dir, name), "wb") as fh:
                fh.write(data)
        self.servers = []

    def start(self, redirect=False, log_path=None):
        main, cdn, log = MOCK.make_servers(
            self.dir, 0, redirect_port=0 if redirect else None, log_path=log_path, echo=False
        )
        for server in (main, cdn):
            if server is not None:
                threading.Thread(target=server.serve_forever, daemon=True).start()
                self.servers.append(server)
        self.addCleanup(self.stop)
        return main.server_address[1], (cdn.server_address[1] if cdn else None), log

    def stop(self):
        for server in self.servers:
            server.shutdown()
            server.server_close()

    def test_token(self):
        port, _, _ = self.start()
        status, _, body = get(port, "/token?scope=repository:%s:pull&service=ghcr.io" % REPO)
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(body), {"token": "t"})

    def test_manifest_needs_the_bearer(self):
        port, _, log = self.start()
        path = "/v2/%s/manifests/declared-android" % REPO
        status, headers, _ = get(port, path)
        self.assertEqual(status, 401)
        self.assertIn("WWW-Authenticate", headers)
        status, _, _ = get(port, path, {"Authorization": "Bearer wrong"})
        self.assertEqual(status, 401)
        status, headers, _ = get(port, path, {"Authorization": "Bearer t"})
        self.assertEqual(status, 200)
        self.assertEqual(headers["Content-Type"], "application/vnd.oci.image.manifest.v1+json")
        self.assertEqual(log.lines[0], "REQ main GET %s auth=no -> 401" % path)
        self.assertEqual(log.lines[1], "REQ main GET %s auth=yes -> 401" % path)
        self.assertEqual(log.lines[2], "REQ main GET %s auth=yes -> 200" % path)

    def test_manifest_layers_carry_titles_digests_and_sizes(self):
        port, _, _ = self.start()
        status, _, body = get(
            port, "/v2/%s/manifests/declared-android" % REPO, {"Authorization": "Bearer t"}
        )
        self.assertEqual(status, 200)
        manifest = json.loads(body)
        self.assertEqual(manifest["schemaVersion"], 2)
        titles = [layer["annotations"]["org.opencontainers.image.title"] for layer in manifest["layers"]]
        self.assertEqual(titles, sorted(self.files))
        for layer in manifest["layers"]:
            data = self.files[layer["annotations"]["org.opencontainers.image.title"]]
            self.assertEqual(layer["digest"], sha(data))
            self.assertEqual(layer["size"], len(data))

    def test_blob_is_served_with_the_bearer(self):
        port, _, _ = self.start()
        path = "/v2/%s/blobs/%s" % (REPO, sha(APK))
        self.assertEqual(get(port, path)[0], 401)
        status, _, body = get(port, path, {"Authorization": "Bearer t"})
        self.assertEqual(status, 200)
        self.assertEqual(body, APK)

    def test_unknown_paths_and_blobs_are_404(self):
        port, _, _ = self.start()
        bearer = {"Authorization": "Bearer t"}
        self.assertEqual(get(port, "/nope")[0], 404)
        self.assertEqual(get(port, "/v2/%s/blobs/sha256:%s" % (REPO, "0" * 64), bearer)[0], 404)
        self.assertEqual(get(port, "/v2/%s/blobs/md5:abc" % REPO, bearer)[0], 404)

    def test_redirect_goes_to_the_cdn_listener_without_a_bearer(self):
        port, cdn_port, log = self.start(redirect=True)
        digest = sha(APK)
        status, headers, body = get(
            port, "/v2/%s/blobs/%s" % (REPO, digest), {"Authorization": "Bearer t"}
        )
        self.assertEqual(status, 307)
        self.assertEqual(body, b"")
        location = headers["Location"]
        self.assertEqual(location, "http://127.0.0.1:%d/cdn/%s" % (cdn_port, digest))
        # the client follows without the bearer: the CDN serves the blob and logs auth=no
        status, _, body = get(cdn_port, "/cdn/%s" % digest)
        self.assertEqual(status, 200)
        self.assertEqual(body, APK)
        self.assertIn("REQ cdn GET /cdn/%s auth=no -> 200" % digest, log.lines)
        # a client that wrongly forwards the bearer is visible in the log
        get(cdn_port, "/cdn/%s" % digest, {"Authorization": "Bearer t"})
        self.assertIn("REQ cdn GET /cdn/%s auth=yes -> 200" % digest, log.lines)

    def test_location_port_can_differ_from_the_listening_port(self):
        main, cdn, _ = MOCK.make_servers(self.dir, 0, redirect_port=0, redirect_location_port=19393, echo=False)
        for server in (main, cdn):
            threading.Thread(target=server.serve_forever, daemon=True).start()
            self.servers.append(server)
        self.addCleanup(self.stop)
        digest = sha(APK)
        status, headers, _ = get(
            main.server_address[1], "/v2/%s/blobs/%s" % (REPO, digest), {"Authorization": "Bearer t"}
        )
        self.assertEqual(status, 307)
        self.assertEqual(headers["Location"], "http://127.0.0.1:19393/cdn/%s" % digest)

    def test_the_directory_is_read_at_every_request(self):
        port, _, _ = self.start()
        path = "/v2/%s/manifests/declared-android" % REPO
        bearer = {"Authorization": "Bearer t"}
        first = json.loads(get(port, path, bearer)[2])
        new_apk = b"another apk" * 50
        with open(os.path.join(self.dir, "pili-pos-android-1.0.1.apk"), "wb") as fh:
            fh.write(new_apk)
        second = json.loads(get(port, path, bearer)[2])
        digests = {l["annotations"]["org.opencontainers.image.title"]: l["digest"] for l in second["layers"]}
        self.assertEqual(digests["pili-pos-android-1.0.1.apk"], sha(new_apk))
        self.assertNotEqual(first, second)
        status, _, body = get(port, "/v2/%s/blobs/%s" % (REPO, sha(new_apk)), bearer)
        self.assertEqual((status, body), (200, new_apk))
        # the old blob is gone with its file
        self.assertEqual(get(port, "/v2/%s/blobs/%s" % (REPO, sha(APK)), bearer)[0], 404)

    def test_log_file_gets_one_line_per_request(self):
        log_path = os.path.join(self.dir, "..", "plat-registry-test-%d.log" % os.getpid())
        log_path = os.path.abspath(log_path)
        self.addCleanup(lambda: os.path.exists(log_path) and os.remove(log_path))
        port, _, _ = self.start(log_path=log_path)
        get(port, "/token")
        get(port, "/v2/%s/manifests/declared-android" % REPO)
        with open(log_path, encoding="utf-8") as fh:
            lines = fh.read().splitlines()
        self.assertEqual(
            lines,
            [
                "REQ main GET /token auth=no -> 200",
                "REQ main GET /v2/%s/manifests/declared-android auth=no -> 401" % REPO,
            ],
        )

    def test_missing_directory_gives_an_empty_manifest(self):
        shutil.rmtree(self.dir)
        port, _, _ = self.start()
        status, _, body = get(port, "/v2/%s/manifests/declared-android" % REPO, {"Authorization": "Bearer t"})
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(body)["layers"], [])

    def test_main_rejects_a_missing_directory_argument(self):
        self.assertEqual(MOCK.main(["--dir", os.path.join(self.dir, "absent"), "--port", "0"]), 2)


if __name__ == "__main__":
    unittest.main(verbosity=1)
