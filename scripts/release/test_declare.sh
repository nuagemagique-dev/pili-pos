#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# pili-pos - self-test of scripts/release/declare.sh (Linux kiosk)
#
# Usage: scripts/release/test_declare.sh
#
# Everything happens in one mktemp directory outside this repository,
# deleted on exit: a copy of declare.sh in a scratch tree (so its
# deploy/release/declared.pub is the scratch one), a throwaway minisign
# key pair without password, a fake release tarball, and STUB oras and
# gh that serve the tarball and record their arguments. The real oras
# and gh are never reached and no key of the release is involved.
# Prints "TESTDECL | <case> | PASS|FAIL | <detail>" and a final
# "TESTDECL | total | PASS|FAIL | <n> cases". Exit 0 when every case passes.
#
# Needs: minisign, python3.
# ─────────────────────────────────────────────────────────────────
set -uo pipefail
export LC_ALL=C

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

total=0
failed=0
result() { # result NAME PASS|FAIL DETAIL
    total=$((total + 1))
    [[ "$2" == PASS ]] || failed=$((failed + 1))
    printf 'TESTDECL | %s | %s | %s\n' "$1" "$2" "$3"
}
expect() { # expect NAME STATUS(0=true) DETAIL_ON_FAIL
    if [[ "$2" -eq 0 ]]; then result "$1" PASS "ok"; else result "$1" FAIL "$3"; fi
}
bail() {
    printf 'TESTDECL | setup | FAIL | %s\n' "$1"
    printf 'TESTDECL | total | FAIL | setup failed\n'
    exit 1
}
command -v minisign >/dev/null || bail "minisign not found"
command -v python3 >/dev/null || bail "python3 not found"

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT
REPO="$T/repo"
mkdir -p "$REPO/scripts/release" "$REPO/deploy/release" "$T/keys" "$T/served" "$T/pushed" "$T/stubs"
cp "$HERE/declare.sh" "$REPO/scripts/release/declare.sh"
DECLARE="$REPO/scripts/release/declare.sh"
SECRET="$T/keys/declared.key"
PUB="$REPO/deploy/release/declared.pub"
minisign -G -W -p "$T/keys/declared.pub" -s "$SECRET" >/dev/null 2>&1 || bail "minisign -G failed"

# ── the released tarball, as release.yml pushes it ────────────────
mkdir -p "$T/pkg"
printf '#!/bin/sh\necho "pili-pos 1.2.0"\n' >"$T/pkg/pili-pos"
printf 'x' >"$T/pkg/install.sh"
TARBALL="pili-pos-linux-v1.2.0-x86_64.tar.gz"
tar -C "$T/pkg" -czf "$T/served/$TARBALL" pili-pos install.sh
GOOD_SHA1="$(python3 -c 'import hashlib,sys;print(hashlib.sha1(open(sys.argv[1],"rb").read()).hexdigest())' "$T/pkg/pili-pos")"
GOOD_SHA256="$(python3 -c 'import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$T/served/$TARBALL")"

# ── stub oras and gh ──────────────────────────────────────────────
STUBLOG="$T/stub.log"
: >"$STUBLOG"
cat >"$T/stubs/oras" <<STUB
#!/usr/bin/env bash
echo "oras \$*" >>"$STUBLOG"
case "\$1" in
    login) cat >/dev/null ;;
    pull)  # oras pull <ref> -o <dir>: serve the tarball of that tag
        tag="\${2##*:}"
        [[ -f "$T/served/pili-pos-\$tag-x86_64.tar.gz" ]] || { echo "not found: \$2" >&2; exit 1; }
        cp "$T/served/pili-pos-\$tag-x86_64.tar.gz" "\$4/" ;;
    push)  # keep a copy of every pushed file (they live in a temp dir)
        for a in "\$@"; do f="\${a%%:*}"; [[ -f "\$f" ]] && cp "\$f" "$T/pushed/"; done ;;
esac
exit 0
STUB
cat >"$T/stubs/gh" <<STUB
#!/usr/bin/env bash
echo "gh \$*" >>"$STUBLOG"
case "\$1" in
    auth) echo stub-token ;;
    api)  echo stub-user ;;
esac
exit 0
STUB
chmod +x "$T/stubs/oras" "$T/stubs/gh"
for tool in oras gh; do
    [[ "$(PATH="$T/stubs:$PATH" command -v "$tool")" == "$T/stubs/$tool" ]] || bail "the $tool stub does not shadow the real one"
done
run() { # run ARGS... -> RC, output in $T/out.txt
    PATH="$T/stubs:$PATH" PILI_MINISIGN_KEY="$SECRET" bash "$DECLARE" "$@" >"$T/out.txt" 2>&1
    RC=$?
}

# 1. --init-key with the shared secret key present: only the public half is written
run --init-key
ok=1; [[ $RC -eq 0 && -f "$PUB" ]] && cmp -s <(tail -n1 "$PUB") <(tail -n1 "$T/keys/declared.pub") && ok=0
expect "init-key-recreates-public-key" "$ok" "exit $RC: $(tail -n2 "$T/out.txt" | tr '\n' ' ')"

# 2. --init-key again: refused, nothing overwritten
run --init-key
ok=1; [[ $RC -ne 0 ]] && grep -q "already exists" "$T/out.txt" && ok=0
expect "init-key-refuses-when-both-exist" "$ok" "exit $RC"

# 3. a bad version is a usage error, no tool called
: >"$STUBLOG"
run 1.2.0 --sha1 "$GOOD_SHA1"
ok=1; [[ $RC -ne 0 && ! -s "$STUBLOG" ]] && grep -q "usage" "$T/out.txt" && ok=0
expect "bad-version-refused" "$ok" "exit $RC, stub log: $(tr '\n' ';' <"$STUBLOG")"

# 4. an Android tag is not a Linux release
run android-v1.2.0 --sha1 "$GOOD_SHA1"
ok=1; [[ $RC -ne 0 && ! -s "$STUBLOG" ]] && ok=0
expect "android-tag-refused" "$ok" "exit $RC"

# 5. the wrong SHA-1: nothing pushed, no release touched
run linux-v1.2.0 --sha1 0000000000000000000000000000000000000000
ok=1; [[ $RC -ne 0 ]] && ! grep -q "oras push\|gh release" "$STUBLOG" && grep -q "nothing published" "$T/out.txt" && ok=0
expect "wrong-sha1-publishes-nothing" "$ok" "exit $RC, stub log: $(tr '\n' ';' <"$STUBLOG")"

# 6. a real (stubbed) run, given as vX.Y.Z: taken for linux-vX.Y.Z
: >"$STUBLOG"
run v1.2.0 --sha1 "$GOOD_SHA1"
ok=1; [[ $RC -eq 0 ]] && ok=0
expect "declare-run-passes" "$ok" "exit $RC: $(tail -n3 "$T/out.txt" | tr '\n' ' ')"
grep -q "^oras pull ghcr.io/nuagemagique-dev/pili-pos:linux-v1.2.0 " "$STUBLOG"
expect "pulls-the-release-tag" $? "stub log: $(tr '\n' ';' <"$STUBLOG")"
grep -q "^oras push ghcr.io/nuagemagique-dev/pili-pos:declared --artifact-type application/vnd.pili-pos.declared.v1 $TARBALL:application/gzip declared.json:application/json declared.json.minisig:text/plain" "$STUBLOG"
expect "pushes-tag-declared-with-three-files" $? "stub log: $(tr '\n' ';' <"$STUBLOG")"
grep -q "^gh release edit linux-v1.2.0 -R nuagemagique-dev/pili-pos --prerelease=false --latest" "$STUBLOG"
expect "turns-the-prerelease-into-latest" $? "stub log: $(tr '\n' ';' <"$STUBLOG")"

# 7. the descriptor: the shape deploy/ansible/roles/pos_binary checks
python3 - "$T/pushed/declared.json" "$GOOD_SHA1" "$GOOD_SHA256" "$TARBALL" <<'PY'
import json, re, sys
d = json.load(open(sys.argv[1]))
assert d["format"] == 1 and d["platform"] == "linux", d
assert d["version"] == "1.2.0" and d["sha1"] == sys.argv[2] and d["sha256"] == sys.argv[3], d
assert d["asset"] == sys.argv[4], d
assert re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", d["declared_at"]), d
PY
expect "descriptor-shape" $? "$(cat "$T/pushed/declared.json" 2>/dev/null)"
minisign -V -q -p "$PUB" -m "$T/pushed/declared.json" -x "$T/pushed/declared.json.minisig" >/dev/null 2>&1
expect "signature-verifies-with-the-committed-public-key" $? "minisign -V failed"

if [[ "$failed" -eq 0 ]]; then
    printf 'TESTDECL | total | PASS | %d cases\n' "$total"
    exit 0
fi
printf 'TESTDECL | total | FAIL | %d of %d cases failed\n' "$failed" "$total"
exit 1
