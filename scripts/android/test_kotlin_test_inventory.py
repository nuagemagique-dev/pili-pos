#!/usr/bin/env python3
"""Self-test of scripts/android/kotlin_test_inventory.py (no Gradle, no network).

Usage: python3 scripts/android/test_kotlin_test_inventory.py

Builds temporary git repositories with Kotlin test files and runs the inventory on
them through --root. Prints one line per case and a summary; exits 1 on a failure.
"""

import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "kotlin_test_inventory.py")

BASIC = """package dev.example.core

import org.junit.Test

class AlphaTest {
    @Test
    fun one() {}

    @Test fun two() = Unit

    fun helper() {}
}
"""


def run(root, *args):
    r = subprocess.run([sys.executable, "-I", SCRIPT, "--root", root, *args], capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def git(root, *args):
    subprocess.run(
        ["git", "-C", root, "-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", *args],
        check=True, capture_output=True,
    )


class Repo:
    def __init__(self, test):
        self.dir = tempfile.mkdtemp(prefix="ktests.")
        test.addCleanup(self.cleanup)
        git(self.dir, "init", "-q", "-b", "main")

    def cleanup(self):
        subprocess.run(["rm", "-rf", self.dir])

    def write(self, rel, text):
        path = os.path.join(self.dir, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)

    def commit(self, tag=None):
        git(self.dir, "add", "-A")
        git(self.dir, "commit", "-q", "--allow-empty", "-m", "c")
        if tag:
            git(self.dir, "tag", tag)


class InventoryTests(unittest.TestCase):
    def test_listing_is_sorted_and_complete(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/dev/example/core/AlphaTest.kt", BASIC)
        code, out, _ = run(r.dir, "--list")
        self.assertEqual(code, 0)
        self.assertEqual(out.splitlines(), [
            "m/test:dev.example.core.AlphaTest#one",
            "m/test:dev.example.core.AlphaTest#two",
        ])

    def test_annotation_forms(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/Forms.kt", """package p

class Forms {
    @Test
    @Ignore("later")
    fun otherAnnotationBetween() {}

    @Ignore @Test fun inlineBefore() {}

    @org.junit.Test
    internal fun qualified() {}

    @Test(timeout = 5)
    private suspend fun withArgsAndModifiers() {}

    @Test
    @Throws(IllegalStateException::class)
    fun throwsOne() {}

    @TestOnly
    fun notATest() {}

    // @Test
    fun commentedOut() {}

    val s = "@Test fun inString() {}"

    @Test
    val notAFunction = 1

    fun plain() {}
}
""")
        code, out, _ = run(r.dir, "--list")
        self.assertEqual(code, 0)
        names = sorted(l.split("#")[1] for l in out.splitlines())
        self.assertEqual(names, sorted([
            "otherAnnotationBetween", "inlineBefore", "qualified", "withArgsAndModifiers", "throwsOne",
        ]))

    def test_backtick_names_are_kept_and_nested_classes_use_the_first_top_level_class(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/Nested.kt", """package a.b

import org.junit.Test

class Outer {
    class Inner {
        @Test
        fun `a name with spaces (and parens)`() {}
    }

    @Test fun `x { brace`() {}
}

class Second {
    @Test fun late() {}
}
""")
        code, out, _ = run(r.dir, "--list")
        self.assertEqual(code, 0)
        self.assertEqual(out.splitlines(), [
            "m/test:a.b.Outer#`a name with spaces (and parens)`",
            "m/test:a.b.Outer#`x { brace`",
            "m/test:a.b.Outer#late",  # the file's first top-level class owns every test
        ])

    def test_android_test_and_custom_sets(self):
        r = Repo(self)
        r.write("android/m/src/androidTest/kotlin/D.kt", "package d\nclass D {\n @Test fun a() {}\n}\n")
        r.write("android/m/src/integrationTest/kotlin/I.kt", "package i\nclass I {\n @Test fun b() {}\n}\n")
        r.write("android/m/src/main/kotlin/M.kt", "package m\nclass M {\n @Test fun ignored() {}\n}\n")
        r.write("android/m/src/testFixtures/kotlin/F.kt", "package f\nclass F {\n @Test fun ignoredToo() {}\n}\n")
        code, out, _ = run(r.dir, "--list")
        self.assertEqual(code, 0)
        self.assertEqual(out.splitlines(), ["m/androidTest:d.D#a", "m/integrationTest:i.I#b"])

    def test_unchanged_tree_passes(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/AlphaTest.kt", BASIC)
        r.commit("base")
        code, out, _ = run(r.dir, "--base", "base")
        self.assertEqual(code, 0)
        self.assertEqual(out, "KTESTS | total | PASS | base 2, head 2, removed 0\n")

    def test_added_tests_pass(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/AlphaTest.kt", BASIC)
        r.commit("base")
        r.write("android/n/src/test/kotlin/B.kt", "package b\nclass B {\n @Test fun z() {}\n}\n")
        code, out, _ = run(r.dir, "--base", "base")
        self.assertEqual(code, 0)
        self.assertEqual(out, "KTESTS | total | PASS | base 2, head 3, removed 0\n")

    def test_removed_test_fails(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/AlphaTest.kt", BASIC)
        r.commit("base")
        r.write("android/m/src/test/kotlin/AlphaTest.kt", BASIC.replace("@Test fun two() = Unit", "fun two() = Unit"))
        code, out, err = run(r.dir, "--base", "base")
        self.assertEqual(code, 1)
        self.assertEqual(out, "KTESTS | total | FAIL | base 2, head 1, removed 1\n")
        self.assertIn("m/test:dev.example.core.AlphaTest#two", err)

    def test_deleted_file_fails_and_a_rename_rule_maps_it(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/AlphaTest.kt", BASIC)
        r.commit("base")
        os.remove(os.path.join(r.dir, "android/m/src/test/kotlin/AlphaTest.kt"))
        r.write("android/app/src/test/kotlin/AlphaTest.kt", BASIC)
        code, out, _ = run(r.dir, "--base", "base")
        self.assertEqual(code, 1)
        self.assertEqual(out, "KTESTS | total | FAIL | base 2, head 2, removed 2\n")
        r.write("scripts/android/kotlin_test_moves.tsv",
                "# moved to :app\nm/test:\tapp/test:\n")
        code, out, err = run(r.dir, "--base", "base")
        self.assertEqual(code, 0, err)
        self.assertEqual(out, "KTESTS | total | PASS | base 2, head 2, removed 0\n")

    def test_stale_rule_fails(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/AlphaTest.kt", BASIC)
        r.commit("base")
        r.write("scripts/android/kotlin_test_moves.tsv", "gone/test:\tapp/test:\n")
        code, out, err = run(r.dir, "--base", "base")
        self.assertEqual(code, 1)
        self.assertTrue(out.startswith("KTESTS | total | FAIL | base 2, head 2, removed 0"), out)
        self.assertIn("stale rule", err)

    def test_rule_whose_target_is_missing_still_fails(self):
        r = Repo(self)
        r.write("android/m/src/test/kotlin/AlphaTest.kt", BASIC)
        r.commit("base")
        os.remove(os.path.join(r.dir, "android/m/src/test/kotlin/AlphaTest.kt"))
        r.write("scripts/android/kotlin_test_moves.tsv", "m/test:\tapp/test:\n")
        code, out, err = run(r.dir, "--base", "base")
        self.assertEqual(code, 1)
        self.assertIn("removed 2", out)
        self.assertIn("not found", err)

    def test_base_without_the_android_directory(self):
        r = Repo(self)
        r.write("README.md", "x\n")
        r.commit("base")
        r.write("android/m/src/test/kotlin/AlphaTest.kt", BASIC)
        code, out, _ = run(r.dir, "--base", "base")
        self.assertEqual(code, 0)
        self.assertEqual(out, "KTESTS | total | PASS | base 0, head 2, removed 0\n")

    def test_usage_and_git_errors_exit_2(self):
        r = Repo(self)
        r.write("README.md", "x\n")
        r.commit("base")
        self.assertEqual(run(r.dir)[0], 2)
        self.assertEqual(run(r.dir, "--bogus")[0], 2)
        self.assertEqual(run(r.dir, "--base")[0], 2)
        self.assertEqual(run(r.dir, "--base", "no-such-ref")[0], 2)
        r.write("scripts/android/kotlin_test_moves.tsv", "no tab here\n")
        self.assertEqual(run(r.dir, "--base", "base")[0], 2)

    def test_the_real_repository_lists_tests(self):
        root = os.path.abspath(os.path.join(HERE, "..", ".."))
        code, out, _ = run(root, "--list")
        self.assertEqual(code, 0)
        self.assertGreater(len(out.splitlines()), 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
