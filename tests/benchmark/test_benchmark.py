"""Unit tests for the Phase 8 benchmark harness (not the timings themselves).

Run with: python3 -m unittest discover -s tests/benchmark -p 'test_*.py'
"""

from __future__ import annotations

import importlib
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
benchmark = importlib.import_module("benchmark")


def measurement(name: str = "sample", base: int = 20, opt: int = 10) -> benchmark.Measurement:
    return benchmark.Measurement(name, base, opt, 10.0, 5.0, True)


class BenchmarkTests(unittest.TestCase):
    def test_instruction_count_counts_indented_lines_only(self) -> None:
        ir = "declare void @f()\ndefine double @g() {\nbb0:\n  %v = fadd double 1.0, 2.0\n  ret double %v\n}\n"
        self.assertEqual(benchmark.instruction_count(ir), 2)

    def test_measurement_derives_reduction_and_speedup(self) -> None:
        result = measurement()
        self.assertEqual(result.reduction, 50.0)
        self.assertEqual(result.speedup, 2.0)
        self.assertEqual(benchmark.Measurement("z", 0, 0, 1.0, 0.0, True).speedup, 0.0)

    def test_render_report_is_deterministic_and_flags_mismatches(self) -> None:
        report = benchmark.render_report([measurement("b"), measurement("a")])
        self.assertEqual(report, benchmark.render_report([measurement("b"), measurement("a")]))
        self.assertLess(report.index("| `a` |"), report.index("| `b` |"))
        self.assertIn("- Output matches: 2/2", report)
        self.assertIn("| `b` | 20 | 10 | 50.0% | 10.00 | 5.00 | 2.00x | match |", report)
        bad = benchmark.Measurement("bad", 1, 1, 1.0, 1.0, False)
        self.assertIn("| MISMATCH |", benchmark.render_report([bad]))

    def test_cli_facade_is_callable(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "benchmark.py"), "--help"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("--check", completed.stdout)


if __name__ == "__main__":
    unittest.main()
