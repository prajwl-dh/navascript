"""Harness logic for the compatibility corpus.

This module is the testable seam behind the ``scripts/compat.py`` CLI: case
discovery, tool execution and classification. Report rendering lives in
``compat_report``; neither module writes files or parses command-line flags.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from compat_report import render_report

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "tests" / "compatibility" / "corpus"
DEFAULT_REPORT = ROOT / "docs" / "COMPATIBILITY_REPORT.md"

EXPECTATIONS = ("match", "differ", "skip")


@dataclass(frozen=True)
class Case:
    """One corpus case, validated at load time."""

    id: str
    title: str
    category: str
    expect: str
    reason: str
    directory: Path
    # Non-match cases pin the reference output and, where relevant, the exact
    # NavaScript output or failure mode. Without these a crash would count as
    # the "documented" divergence.
    reference_output: str | None = None
    navascript_output: str | None = None
    navascript_fails: bool | None = None

    @property
    def ts_files(self) -> list[Path]:
        return sorted(self.directory.glob("*.ts"))


class CorpusError(Exception):
    """The corpus is malformed; fail loudly rather than reporting a false pass."""


def normalize(text: str) -> str:
    """Canonicalise program output for comparison.

    Trailing whitespace (including the final newline) is insignificant for this
    corpus; interior whitespace and line breaks are not.
    """

    return text.replace("\r\n", "\n").replace("\r", "\n").rstrip()


@dataclass(frozen=True)
class Result:
    """What actually happened when a case was executed."""

    stdout: str
    code: int
    error: str = ""

    @property
    def ran(self) -> bool:
        """Whether the tool executed at all (as opposed to being unavailable)."""

        return not self.error


def load_cases(corpus: Path = CORPUS) -> list[Case]:
    """Discover and validate every case, in deterministic id order."""

    if not corpus.is_dir():
        raise CorpusError(f"corpus directory not found: {corpus}")
    cases: list[Case] = []
    seen: set[str] = set()
    for directory in sorted(corpus.iterdir(), key=lambda path: path.name):
        if not directory.is_dir():
            continue
        manifest = directory / "case.json"
        if not manifest.is_file():
            raise CorpusError(f"{directory.name}: missing case.json")
        try:
            data = json.loads(manifest.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            raise CorpusError(f"{directory.name}: invalid case.json: {exc}") from exc
        if not isinstance(data, dict):
            raise CorpusError(f"{directory.name}: case.json must be an object")
        for field in ("id", "title", "category", "expect"):
            if not isinstance(data.get(field), str) or not data[field].strip():
                raise CorpusError(f"{directory.name}: missing or empty '{field}'")
        case_id = data["id"]
        if case_id in seen:
            raise CorpusError(f"duplicate case id: {case_id}")
        seen.add(case_id)
        expect = data["expect"]
        if expect not in EXPECTATIONS:
            raise CorpusError(f"{case_id}: expect must be one of {EXPECTATIONS}")
        reason = data.get("reason", "")
        if expect != "match" and not isinstance(reason, str):
            raise CorpusError(f"{case_id}: reason must be a string")
        if expect != "match" and not reason.strip():
            raise CorpusError(f"{case_id}: a non-match case needs a reason")
        for name in ("program.ts", "program.ns"):
            if not (directory / name).is_file():
                raise CorpusError(f"{case_id}: missing {name}")

        reference_output = data.get("reference_output")
        navascript_output = data.get("navascript_output")
        navascript_fails = data.get("navascript_fails")
        for name, value in (
            ("reference_output", reference_output),
            ("navascript_output", navascript_output),
        ):
            if value is not None and not isinstance(value, str):
                raise CorpusError(f"{case_id}: {name} must be a string")
        if navascript_fails is not None and not isinstance(navascript_fails, bool):
            raise CorpusError(f"{case_id}: navascript_fails must be a boolean")
        if expect == "differ" and reference_output is None:
            raise CorpusError(f"{case_id}: a differ case must pin reference_output")

        cases.append(
            Case(
                id=case_id,
                title=data["title"].strip(),
                category=data["category"].strip(),
                expect=expect,
                reason=reason.strip(),
                directory=directory,
                reference_output=reference_output,
                navascript_output=navascript_output,
                navascript_fails=navascript_fails,
            )
        )
    if not cases:
        raise CorpusError(f"no cases found in {corpus}")
    return cases


def run(command: list[str], cwd: Path | None = None, timeout: int = 120) -> Result:
    """Run one tool, capturing its output instead of aborting the corpus run."""

    if shutil.which(command[0]) is None and not Path(command[0]).is_file():
        return Result("", -1, f"tool not found: {command[0]}")
    try:
        completed = subprocess.run(
            [str(part) for part in command],
            cwd=cwd,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return Result("", -1, f"{command[0]} failed to run: {exc}")
    return Result(normalize(completed.stdout.decode("utf-8", errors="replace")), completed.returncode)


def run_reference(case: Case, workdir: Path) -> Result:
    """Compile with tsc, then execute the emitted JavaScript with Node."""

    js_dir = workdir / "reference"
    js_dir.mkdir(parents=True, exist_ok=True)
    tsc = run(
        [
            "tsc",
            "--target",
            "ES2020",
            "--module",
            "commonjs",
            "--rootDir",
            case.directory,
            "--outDir",
            js_dir,
            *case.ts_files,
        ]
    )
    if not tsc.ran:
        return tsc
    # tsc reports type errors through its exit status but still emits JavaScript
    # by default. The reference is the *runtime* behaviour of that JavaScript;
    # only a case with no emitted file is genuinely blocked. The `type_error`
    # case records this deliberate choice.
    emitted = js_dir / "program.js"
    if not emitted.is_file():
        return tsc
    return run(["node", emitted])


def run_navascript(case: Case, workdir: Path, binary: Path) -> Result:
    """Build the NavaScript program and execute the native binary."""

    binary_path = binary if binary.is_absolute() else ROOT / binary
    if not binary_path.is_file():
        return Result("", -1, f"compiler not built: {binary_path}")
    output = workdir / "navascript-program"
    build = run([binary_path, "build", case.directory / "program.ns", "-o", output])
    if not build.ran or build.code != 0:
        return build
    return run([output])


def classify(case: Case, reference: Result, actual: Result) -> str:
    """Turn two observations into a raw observation class.

    A missing tool is ``blocked`` (never a pass), a skipped case is ``skip``,
    identical stdout and exit status is ``match``, and anything else is
    ``differ``. A non-match expectation is only a pass when the divergence was
    actually observed, so the report measures behaviour instead of asserting it.
    """

    if case.expect == "skip":
        return "skip"
    if not reference.ran or not actual.ran:
        return "blocked"
    if (reference.stdout, reference.code) == (actual.stdout, actual.code):
        return "match"
    return "differ"


def matches_details(case: Case, reference: Result, actual: Result) -> bool:
    """Check that a divergence is the one the manifest documents.

    Known-difference cases pin what Node prints and, where the divergence is a
    diagnosed construct, that the NavaScript build fails; where it changes an
    output, the exact expected output can be pinned too. An arbitrary crash or
    wrong output therefore cannot satisfy a documented divergence.
    """

    if case.expect != "differ":
        return True
    if case.reference_output is not None and reference.stdout != normalize(case.reference_output):
        return False
    if case.navascript_output is not None and actual.stdout != normalize(case.navascript_output):
        return False
    if case.navascript_fails is not None and case.navascript_fails != (actual.code != 0):
        return False
    return True


def evaluate(case: Case, reference: Result, actual: Result) -> str:
    """Translate an observation class into the case outcome."""

    observed = classify(case, reference, actual)
    if observed in ("skip", "blocked"):
        return observed
    if observed != case.expect:
        return "fail"
    return "pass" if matches_details(case, reference, actual) else "fail"


def build_report(corpus: Path, binary: Path) -> tuple[str, dict[str, str]]:
    """Run every case and return the rendered report plus raw outcomes."""

    cases = load_cases(corpus)
    results: dict[str, str] = {}
    with tempfile.TemporaryDirectory(prefix="navascript-compat-") as tmp:
        workdir = Path(tmp)
        for case in cases:
            if case.expect == "skip":
                results[case.id] = "skip"
                continue
            reference = run_reference(case, workdir / case.id)
            actual = run_navascript(case, workdir / case.id, binary)
            results[case.id] = evaluate(case, reference, actual)
    return render_report(cases, results), results
