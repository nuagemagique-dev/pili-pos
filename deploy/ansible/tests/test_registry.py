"""Unit tests of the registry mock (registry.py): what it serves and what it refuses.

Run: python3 deploy/ansible/tests/test_registry.py   (also run by in-container.sh)
"""
import hashlib
import os
import socket
import sys
import tempfile
import threading
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import registry  # noqa: E402

REPO = "nuagemagique-dev/pili-pos"
AUTH = "Authorization: Bearer anon\r\n"


class RegistryMockTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        root = cls.tmp.name
        os.mkdir(os.path.join(root, "blobs"))
        with open(os.path.join(root, "declared.json"), "wb") as f:
            f.write(b'{"layers": []}')
        cls.digest = hashlib.sha256(b"blob").hexdigest()
        with open(os.path.join(root, "blobs", cls.digest), "wb") as f:
            f.write(b"blob")
        # A file next to the served root that a traversal would reach.
        cls.outside = os.path.join(os.path.dirname(root), "outside-%d.json" % os.getpid())
        with open(cls.outside, "w") as f:
            f.write("secret")
        cls.server = registry.make_server(root, REPO, 0)
        cls.port = cls.server.server_address[1]
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        os.remove(cls.outside)
        cls.tmp.cleanup()

    def raw(self, target, headers=""):
        """Send a raw request line (no client-side URL cleaning); return (status, head, body)."""
        with socket.create_connection(("127.0.0.1", self.port), timeout=5) as s:
            s.sendall(("GET %s HTTP/1.0\r\n%s\r\n" % (target, headers)).encode("latin-1"))
            data = b""
            while True:
                chunk = s.recv(65536)
                if not chunk:
                    break
                data += chunk
        head, _, body = data.partition(b"\r\n\r\n")
        status = int(head.split(b" ", 2)[1])
        return status, head.decode("latin-1"), body

    def test_serves_the_happy_path(self):
        self.assertEqual(self.raw("/token")[0], 200)
        status, _, body = self.raw("/v2/%s/manifests/declared" % REPO, AUTH)
        self.assertEqual((status, body), (200, b'{"layers": []}'))
        status, head, _ = self.raw("/v2/%s/blobs/sha256:%s" % (REPO, self.digest), AUTH)
        self.assertEqual(status, 307)
        self.assertIn("Location: http://localhost:%d/storage/%s\r\n" % (self.port, self.digest), head + "\r\n")
        self.assertEqual(self.raw("/storage/%s" % self.digest)[1:][1], b"blob")

    def test_auth_rules(self):
        self.assertEqual(self.raw("/v2/%s/manifests/declared" % REPO)[0], 401)
        self.assertEqual(self.raw("/storage/%s" % self.digest, AUTH)[0], 400)

    def test_traversal_is_refused(self):
        name = os.path.basename(self.outside)[: -len(".json")]
        for target in (
            "/v2/%s/manifests/../%s" % (REPO, name),
            "/v2/%s/manifests/..%%2F%s" % (REPO, name),
            "/v2/%s/manifests/.." % REPO,
            "/v2/%s/blobs/sha256:../../%s" % (REPO, name),
            "/storage/../%s.json" % name,
            "/storage/..%%2F..%%2Fetc%%2Fpasswd",
        ):
            status, _, body = self.raw(target, AUTH if target.startswith("/v2/") else "")
            self.assertIn(status, (400, 404), target)
            self.assertNotIn(b"secret", body, target)

    def test_other_repository_and_bad_references(self):
        for target in (
            "/v2/other/repo/manifests/declared",
            "/v2/%s/manifests/" % REPO,
            "/v2/%s/manifests/%s" % (REPO, "a" * 129),
            "/v2/%s/blobs/sha512:%s" % (REPO, self.digest),
            "/v2/%s/blobs/sha256:%s" % (REPO, self.digest.upper()),
            "/v2/%s/blobs/sha256:%s" % (REPO, "0" * 64),
            "/v2/%s/tags/list" % REPO,
        ):
            self.assertEqual(self.raw(target, AUTH)[0], 404, target)

    def test_crlf_in_a_reference_never_reaches_a_header(self):
        # Encoded CR LF stays in the path (the server never decodes it): refused.
        status, head, _ = self.raw("/v2/%s/blobs/sha256:%s%%0d%%0aX-Injected:%%201" % (REPO, self.digest), AUTH)
        self.assertEqual(status, 404)
        self.assertNotIn("X-Injected", head)
        status, head, _ = self.raw("/v2/%s/manifests/declared%%0d%%0aX-Injected:%%201" % REPO, AUTH)
        self.assertEqual(status, 404)
        self.assertNotIn("X-Injected", head)

    def test_a_long_repeated_path_answers_fast(self):
        # Just under http.server's 64 KiB request line limit (above it: 414 before
        # the handler), so the handler itself parses it.
        target = "/v2/" + "a/v2/" * 12000 + "manifests/declared"
        self.assertLess(len(target), 65000)
        start = time.monotonic()
        status, _, _ = self.raw(target, AUTH)
        self.assertEqual(status, 404)
        self.assertLess(time.monotonic() - start, 2.0)

    def test_validators(self):
        self.assertTrue(registry.valid_repo(REPO))
        for bad in ("", "Upper/x", "a//b", "a..b", "-a", "a-", "a/../b", "a b"):
            self.assertFalse(registry.valid_repo(bad), bad)
        self.assertTrue(registry.valid_tag("declared"))
        self.assertTrue(registry.valid_tag("linux-v1.2.0"))
        for bad in ("", ".hidden", "-x", "a/b", "a\r\nb", "x" * 129, ".."):
            self.assertFalse(registry.valid_tag(bad), bad)
        self.assertTrue(registry.valid_digest_hex(self.digest))
        self.assertFalse(registry.valid_digest_hex(self.digest[:-1] + "g"))
        with self.assertRaises(ValueError):
            registry.make_handler(self.tmp.name, "../etc")


if __name__ == "__main__":
    unittest.main(verbosity=2)
