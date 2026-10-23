#!/usr/bin/env bash
# Last step of every job on the self-hosted runner: empties the host-mounted
# checkout, temp and home folders. The job container runs as root, so only it
# can delete what the job wrote; the container itself (with /opt and /tmp,
# where the build output lives) is removed by the runner when the job ends.
set -uo pipefail
for d in "${GITHUB_WORKSPACE:-}" "${RUNNER_TEMP:-}" "${HOME:-}"; do
    case "$d" in
        "" | / | /root) continue ;;
    esac
    [ -d "$d" ] && find "$d" -mindepth 1 -delete 2>/dev/null
done
exit 0
