#!/usr/bin/env python3
# ─────────────────────────────────────────────────────────────────
# pili-pos - per-entry comparison of two APKs (P3.4, PD83; port of the S5 tool)
#
# Usage: python3 -I scripts/android/apk_entry_diff.py A.apk B.apk
#
# Prints the entry manifest of each APK: the sha256 over the sorted list of
# (entry name, sha256 of the uncompressed content). That digest ignores the zip
# container layout (timestamps, compression, order, extra fields) and the APK
# Signing Block, so it is the content digest of an APK.
#     MANIFEST | A | <sha256> | <entries>
#     MANIFEST | B | <sha256> | <entries>
# Then, per entry whose content differs or that exists on one side only:
#     DIFF | <name> | sha256 A <..> B <..> | size A <..> B <..> | csize A <..> B <..> |
#     crc A <..> B <..> | date A <..> B <..>
#     ONLY | <A|B> | <name>
# Entries with equal content but different container metadata (compressed size, method,
# timestamp, extra field) are listed as
#     META | <name> | <fields that differ>
# as is a different entry order (META | (order) | ...). META lines never change the
# exit status.
# A last line says "RESULT | EQUAL" or "RESULT | DIFFERENT | <n> entries".
# Exit 0 when the contents are equal, 1 when they differ, 2 on a usage error or when
# a file is not a readable zip. Python 3 standard library only.
# ─────────────────────────────────────────────────────────────────
import hashlib
import sys
import zipfile

META_FIELDS = ("csize", "method", "date_time", "extra")


def load(path):
    entries = {}
    order = []
    with zipfile.ZipFile(path) as z:
        for info in z.infolist():
            if info.filename in entries:
                raise ValueError("%s: duplicate entry name %s" % (path, info.filename))
            data = z.read(info.filename)
            entries[info.filename] = {
                "sha256": hashlib.sha256(data).hexdigest(),
                "size": info.file_size,
                "csize": info.compress_size,
                "method": info.compress_type,
                "date_time": info.date_time,
                "crc": info.CRC,
                "extra": info.extra,
            }
            order.append(info.filename)
    return entries, order


def manifest(entries):
    h = hashlib.sha256()
    for name in sorted(entries):
        h.update(name.encode("utf-8") + b"\0" + entries[name]["sha256"].encode() + b"\n")
    return h.hexdigest()


def diff(a, a_order, b, b_order, out):
    """Writes the report to out; returns the number of content differences."""
    out.write("MANIFEST | A | %s | %d\n" % (manifest(a), len(a)))
    out.write("MANIFEST | B | %s | %d\n" % (manifest(b), len(b)))
    content = 0
    meta = []
    for name in sorted(set(a) | set(b)):
        ea, eb = a.get(name), b.get(name)
        if ea is None or eb is None:
            out.write("ONLY | %s | %s\n" % ("B" if ea is None else "A", name))
            content += 1
            continue
        if ea["sha256"] != eb["sha256"]:
            content += 1
            out.write(
                "DIFF | %s | sha256 A %s B %s | size A %d B %d | csize A %d B %d | crc A %08x B %08x | date A %s B %s\n"
                % (
                    name,
                    ea["sha256"], eb["sha256"],
                    ea["size"], eb["size"],
                    ea["csize"], eb["csize"],
                    ea["crc"], eb["crc"],
                    "%04d-%02d-%02dT%02d:%02d:%02d" % ea["date_time"],
                    "%04d-%02d-%02dT%02d:%02d:%02d" % eb["date_time"],
                )
            )
            continue
        fields = [k for k in META_FIELDS if ea[k] != eb[k]]
        if fields:
            meta.append("META | %s | %s\n" % (name, ",".join(fields)))
    for line in meta:
        out.write(line)
    common_a = [n for n in a_order if n in b]
    common_b = [n for n in b_order if n in a]
    if common_a != common_b:
        out.write("META | (order) | the entry order differs\n")
    if content == 0:
        out.write("RESULT | EQUAL\n")
    else:
        out.write("RESULT | DIFFERENT | %d entries\n" % content)
    return content


def main(argv):
    if len(argv) != 3 or argv[1] in ("-h", "--help"):
        sys.stderr.write("usage: apk_entry_diff.py A.apk B.apk\n")
        return 2
    try:
        a, a_order = load(argv[1])
        b, b_order = load(argv[2])
    except (OSError, zipfile.BadZipFile, ValueError) as e:
        sys.stderr.write("apk_entry_diff: %s\n" % e)
        return 2
    return 1 if diff(a, a_order, b, b_order, sys.stdout) else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
