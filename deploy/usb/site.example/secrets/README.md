# Secrets of the site

Put the files the site needs here. Each one is optional.

| File | Installed as | What it is |
|------|--------------|------------|
| `fdm/token` | `/etc/pili-pos/fdm/token` | Bearer token of the FDM |
| `fdm/ca.pem` | `/etc/pili-pos/fdm/ca.pem` | CA of the FDM's server certificate (HTTPS) |
| `fdm/client.pem` | `/etc/pili-pos/fdm/client.pem` | mTLS client certificate (port 443) |
| `fdm/client.key` | `/etc/pili-pos/fdm/client.key` | mTLS client key |
| `hubrise.secret` | `/etc/pili-pos/secrets/hubrise.secret` | HubRise OAuth client secret |
| `tem.key` | `/etc/pili-pos/secrets/tem.key` | Scaleway TEM API key (digital ticket) |
| `github.token` | `/etc/pili-pos/secrets/github.token` | Only for a private releases repository (contents:read) |

On the terminal, the files are copied to the encrypted `/var` as `root:pos`
with mode `0640`. Then they are deleted from the terminal's ESP.

They stay on the USB key until you run `deploy/usb/make-key.sh --wipe-secrets`.

This README is not copied to the terminal.
