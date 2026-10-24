#!/usr/bin/env bash
# Runs deploy/ansible/site.yml against a throwaway Debian 13 container
# (the terminal's distribution, its ansible-core and minisign), with a
# fake pili-pos, a local "releases" HTTP server and a throwaway minisign
# key. No network, no systemd, no NetworkManager: the network role is
# exercised in DHCP mode only (nothing to do), the rest for real.
#
# Usage: deploy/ansible/tests/run.sh
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
exec docker run --rm --platform linux/amd64 -e VERBOSE \
    -v "$REPO_ROOT/deploy:/src/deploy:ro" \
    debian:trixie bash /src/deploy/ansible/tests/in-container.sh
