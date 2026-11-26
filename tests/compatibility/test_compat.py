"""Unit tests for the compatibility harness (not the corpus programs).

The tests import the harness's public logic module normally and exercise the
CLI façade once. Run with:
python3 -m unittest discover -s tests/compatibility -p 'test_*.py'
"""

from __future__ import annotations

import importlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
compat = importlib.import_module("compat_core")


def case(
    case_id: str = "sample",
    expect: str = "match",
    directory: Path | None = None,
    reference_output: str | None = None,
    **details: object,
) -> compat.Case:
    return compat.Case(
        case_id,
        "Sample",
        "runtime",
        expect,
        "reason",
        directory or Path("."),
        reference_output,
        details.get("navascript_output"),
        details.get("navascript_fails"),
    )


def result(stdout: str = "", code: int = 0, error: str = "") -> compat.Result:
    return compat.Result(compat.normalize(stdout), code, error)


class HarnessTests(unittest.TestCase):
    def test_normalize_canonicalises_line_endings_and_trailing_space(self) -> None:
        self.assertEqual(compat.normalize("a\r\nb\r\n\n"), "a\nb")
        self.assertEqual(compat.normalize("a   "), "a")
        self.assertNotEqual(compat.normalize("a\n\nb"), "a\nb")

    def test_load_cases_is_sorted_and_validates_expectations(self) -> None:
        cases = compat.load_cases()
        self.assertGreater(len(cases), 20)
        self.assertEqual([c.id for c in cases], sorted(c.id for c in cases))
        self.assertTrue(all(c.expect in compat.EXPECTATIONS for c in cases))
        self.assertTrue(all(c.reason for c in cases if c.expect != "match"))
        self.assertTrue(all(c.reference_output for c in cases if c.expect == "differ"))
        self.assertTrue(
            all((c.directory / "program.ts") in c.ts_files for c in cases),
            "every case must provide a program.ts",
        )

    def test_load_cases_rejects_duplicate_ids(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in ("a", "b"):
                directory = root / name
                directory.mkdir()
                (directory / "case.json").write_text(
                    json.dumps(
                        {
                            "id": "same",
                            "title": "Duplicate",
                            "category": "runtime",
                            "expect": "match",
                        }
                    ),
                    encoding="utf-8",
                )
                (directory / "program.ts").touch()
                (directory / "program.ns").touch()
            with self.assertRaises(compat.CorpusError):
                compat.load_cases(root)

    def test_load_cases_requires_reasons_and_measured_differences(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp) / "case"
            directory.mkdir()
            (directory / "program.ts").touch()
            (directory / "program.ns").touch()
            metadata = {
                "id": "case",
                "title": "No reason",
                "category": "runtime",
                "expect": "differ",
            }
            (directory / "case.json").write_text(json.dumps(metadata), encoding="utf-8")
            with self.assertRaises(compat.CorpusError):
                compat.load_cases(Path(tmp))
            metadata["reason"] = "documented reason"
            (directory / "case.json").write_text(json.dumps(metadata), encoding="utf-8")
            with self.assertRaises(compat.CorpusError):
                compat.load_cases(Path(tmp))

    def test_classify_distinguishes_match_differ_skip_and_blocked(self) -> None:
        self.assertEqual(compat.classify(case(), result("1"), result("1")), "match")
        self.assertEqual(compat.classify(case(), result("1"), result("2")), "differ")
        self.assertEqual(
            compat.classify(case(), result("1"), result("1", -1, "missing tool")),
            "blocked",
        )
        self.assertEqual(
            compat.classify(case("s", "skip"), result("1"), result("2")), "skip"
        )

    def test_evaluate_verifies_the_documented_divergence(self) -> None:
        divergent = case("d", "differ", reference_output="1")
        self.assertEqual(compat.evaluate(divergent, result("1"), result("2")), "pass")
        self.assertEqual(compat.evaluate(divergent, result("1"), result("1")), "fail")
        wrong_reference = case("d", "differ", reference_output="9")
        self.assertEqual(compat.evaluate(wrong_reference, result("1"), result("2")), "fail")
        wrong_exit = case("d", "differ", reference_output="1", navascript_fails=True)
        self.assertEqual(compat.evaluate(wrong_exit, result("1"), result("2")), "fail")

    def test_render_report_is_deterministic_and_counts_outcomes(self) -> None:
        cases = [case("b"), case("a"), case("s", "skip")]
        results = {"a": "pass", "b": "fail", "s": "skip"}
        report = compat.render_report(cases, results)
        self.assertEqual(report, compat.render_report(cases, results))
        self.assertIn("| Failed (behaviour contradicted the expectation) | 1 |", report)
        self.assertIn("Executed-pass rate: 50.0% (1/2 executed cases).", report)
        self.assertLess(report.index("| `a` |"), report.index("| `b` |"))
        self.assertIn("- `b` — Sample", report)
        self.assertIn("Skipped cases are declared", report)

    def test_cli_facade_is_callable(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "compat.py"), "--help"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("--check", completed.stdout)


if __name__ == "__main__":
    unittest.main()
