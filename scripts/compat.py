#!/usr/bin/env python3
"""CLI façade for the reproducible TypeScript/NavaScript compatibility runner.

All discovery, execution and classification logic lives in ``compat_core``;
this file only parses arguments, calls ``build_report`` and writes or checks
the committed Markdown report.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from compat_core import CORPUS, DEFAULT_REPORT, CorpusError, build_report

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_BINARY = ROOT / "target" / "debug" / "navascript"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, default=CORPUS, help="corpus directory")
    parser.add_argument("--navascript", type=Path, default=DEFAULT_BINARY, help="compiler binary")
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT, help="report to write or check")
    parser.add_argument(
        "--check",
        action="store_true",
        help="do not write; exit non-zero if the committed report is stale",
    )
    args = parser.parse_args(argv)

    try:
        report, results = build_report(args.corpus, args.navascript)
    except CorpusError as exc:
        print(f"corpus error: {exc}", file=sys.stderr)
        return 2

    if args.check:
        if not args.report.is_file() or args.report.read_text(encoding="utf-8") != report:
            print(f"compatibility report is stale: {args.report}", file=sys.stderr)
            return 1
        print(f"compatibility report is current ({len(results)} cases)")
        return 0

    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(report, encoding="utf-8")
    failed = [case_id for case_id, outcome in results.items() if outcome == "fail"]
    blocked = [case_id for case_id, outcome in results.items() if outcome == "blocked"]
    print(f"wrote {args.report} ({len(results)} cases)")
    if failed:
        print(f"failed cases: {', '.join(failed)}", file=sys.stderr)
        return 1
    if blocked:
        print(f"blocked cases: {', '.join(blocked)}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
