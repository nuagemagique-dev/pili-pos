# Mock FDM TLS certificates (test only)

Made like module2's (manual.module2.be, "HTTPS en mTLS"): an X.509 v3
root CA, and X.509 v1 server and client certificates without any
extension or host name, RSA 2048, SHA-256, valid 100 years. Generated
once with openssl 3 and re-signed as v1 (openssl 3 only writes v3).

These keys are public test material: never use them anywhere real.

`MOCK_FDM_TLS_DIR` points the mock at this directory (see the crate
README); the POS then uses `ca.crt`, `client.crt` and `client.key`.
