#!/usr/bin/env python3
"""Run the bounded #2052 archive engine matrix in isolated serial children."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import importlib
import json
import os
from pathlib import Path
import platform
import re
import resource
import subprocess
import sys
import time
from typing import Any, Sequence

import duckdb
import pysqlite3 as sqlite3


CASES = (
    "24h-stable",
    "24h-churn",
    "30d-stable",
    "30d-churn",
    "1y-stable",
    "1y-churn",
    "30d-churn-1m",
)
STRATEGIES = ("sqlite", "duckdb", "parquet")
REQUIRED_SQLITE_VERSION = "3.46.0"
REQUIRED_DUCKDB_VERSION = "1.5.5"
MEMORY_LIMIT_RE = re.compile(r"^[1-9][0-9]*(?:\.[0-9]+)?(?:KB|MB|GB|TB)$", re.IGNORECASE)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source-root",
        type=Path,
        required=True,
        help="Directory containing matrix.json and the seven immutable source cases",
    )
    parser.add_argument("--output", type=Path, required=True, help="New result directory")
    parser.add_argument("--repetitions", type=int, default=7)
    parser.add_argument("--threads", type=int, default=2)
    parser.add_argument("--memory-limit", default="128MB")
    args = parser.parse_args(argv)
    if args.repetitions <= 0 or args.threads <= 0:
        parser.error("--repetitions and --threads must be positive")
    if not MEMORY_LIMIT_RE.fullmatch(args.memory_limit):
        parser.error("--memory-limit must be a positive DuckDB size such as 128MB")
    args.source_root = args.source_root.resolve()
    args.output = args.output.resolve()
    if not args.source_root.is_dir():
        parser.error("--source-root must be an existing directory")
    source_matrix = args.source_root / "matrix.json"
    if not source_matrix.is_file():
        parser.error("--source-root must contain matrix.json")
    missing = [
        str(path)
        for case in CASES
        for path in (
            args.source_root / case / "relational.sqlite3",
            args.source_root / case / "report.json",
        )
        if not path.is_file()
    ]
    if missing:
        parser.error("missing required case inputs: " + ", ".join(missing))
    if args.output.exists():
        parser.error("--output must not exist")
    try:
        args.output.relative_to(args.source_root)
    except ValueError:
        pass
    else:
        parser.error("--output must be outside --source-root")
    return args


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    assert_versions()
    script_path = Path(__file__).resolve()
    engine_path = script_path.with_name("engine_benchmark.py")
    if not engine_path.is_file():
        raise SystemExit(f"missing engine benchmark {engine_path}")

    args.output.mkdir(parents=True, exist_ok=False)
    (args.output / "cases").mkdir()
    (args.output / "logs").mkdir()
    report = initial_report(args, script_path, engine_path)
    write_progress(args.output, report)

    for case_index, case_name in enumerate(CASES):
        case = report["cases"][case_name]
        source = args.source_root / case_name / "relational.sqlite3"
        range_report = args.source_root / case_name / "report.json"
        case_output = args.output / "cases" / case_name
        case_logs = args.output / "logs" / case_name
        case_logs.mkdir()

        prepare_command = engine_command(
            engine_path, args, source, range_report, case_output, "prepare", None
        )
        case["prepare"] = run_phase(
            prepare_command, case_logs / "prepare.stdout", case_logs / "prepare.stderr"
        )
        attach_artifact(case["prepare"], case_output / "prepared.json")
        update_case_passed(case)
        write_progress(args.output, report)
        print_phase(case_name, "prepare", case["prepare"])

        order = STRATEGIES[case_index % len(STRATEGIES) :] + STRATEGIES[: case_index % len(STRATEGIES)]
        case["query_order"] = list(order)
        if not phase_passed(case["prepare"]):
            for strategy in order:
                case["queries"][strategy] = {
                    "state": "skipped",
                    "reason": "prepare phase failed",
                }
                update_case_passed(case)
                write_progress(args.output, report)
                print(f"{case_name} query-{strategy}: state=skipped", flush=True)
            continue

        for strategy in order:
            query_command = engine_command(
                engine_path, args, source, range_report, case_output, "query", strategy
            )
            phase = run_phase(
                query_command,
                case_logs / f"query-{strategy}.stdout",
                case_logs / f"query-{strategy}.stderr",
            )
            attach_artifact(phase, case_output / f"query-{strategy}.json")
            case["queries"][strategy] = phase
            update_case_passed(case)
            write_progress(args.output, report)
            print_phase(case_name, f"query-{strategy}", phase)

    report["completed"] = True
    report["completed_at_utc"] = utc_now()
    report["all_passed"] = all(case["all_passed"] for case in report["cases"].values())
    report["failure_count"] = sum(
        not phase_passed(phase)
        for case in report["cases"].values()
        for phase in [case.get("prepare"), *case["queries"].values()]
        if phase is not None
    )
    write_progress(args.output, report)
    print(
        f"matrix completed: all_passed={str(report['all_passed']).lower()} "
        f"failures={report['failure_count']}",
        flush=True,
    )
    return 0 if report["all_passed"] else 1


def initial_report(
    args: argparse.Namespace, script_path: Path, engine_path: Path
) -> dict[str, Any]:
    return {
        "format": "hardviz-archive-engine-matrix-v1",
        "completed": False,
        "all_passed": False,
        "started_at_utc": utc_now(),
        "config": {
            "case_order": list(CASES),
            "strategy_order_base": list(STRATEGIES),
            "execution": "strictly serial child processes",
            "interpreter": sys.executable,
            "repetitions": args.repetitions,
            "threads": args.threads,
            "memory_limit": args.memory_limit,
            "source_root": str(args.source_root),
            "output": str(args.output),
        },
        "environment": environment_report(args, script_path, engine_path),
        "measurement_contract": {
            "child_isolation": (
                "Prepare and every strategy query run in separate serial child processes. "
                "Each child peak is process-wide high-water RSS, not incremental query memory."
            ),
            "duckdb_memory_limit": "DuckDB-managed memory limit; not a whole-process RSS cap.",
            "cache_state": "Repeated reads may be OS/cache-primed; this is not controlled cold-cache evidence.",
            "wait_accounting": "wall clock plus wait4 child user CPU, system CPU, and ru_maxrss",
        },
        "cases": {
            name: {
                "index": index,
                "source": str(args.source_root / name / "relational.sqlite3"),
                "range_report": str(args.source_root / name / "report.json"),
                "prepare": None,
                "query_order": [],
                "queries": {},
                "all_passed": False,
            }
            for index, name in enumerate(CASES)
        },
    }


def engine_command(
    engine_path: Path,
    args: argparse.Namespace,
    source: Path,
    range_report: Path,
    output: Path,
    mode: str,
    strategy: str | None,
) -> list[str]:
    command = [
        sys.executable,
        str(engine_path),
        "--source",
        str(source),
        "--range-report",
        str(range_report),
        "--output",
        str(output),
        "--repetitions",
        str(args.repetitions),
        "--threads",
        str(args.threads),
        "--memory-limit",
        args.memory_limit,
        "--mode",
        mode,
    ]
    if strategy is not None:
        command.extend(("--strategy", strategy))
    return command


def run_phase(command: list[str], stdout_path: Path, stderr_path: Path) -> dict[str, Any]:
    started_at = utc_now()
    wall_started = time.perf_counter_ns()
    phase: dict[str, Any] = {
        "state": "running",
        "command": command,
        "started_at_utc": started_at,
        "stdout": str(stdout_path),
        "stderr": str(stderr_path),
    }
    process: subprocess.Popen[bytes] | None = None
    try:
        with stdout_path.open("wb") as stdout_handle, stderr_path.open("wb") as stderr_handle:
            child_environment = os.environ.copy()
            child_environment["PYTHONDONTWRITEBYTECODE"] = "1"
            process = subprocess.Popen(
                command,
                stdout=stdout_handle,
                stderr=stderr_handle,
                env=child_environment,
            )
            if hasattr(os, "wait4"):
                pid, status, usage = os.wait4(process.pid, 0)
                if pid != process.pid:
                    raise RuntimeError(f"wait4 returned unexpected pid {pid}")
                exit_code = os.waitstatus_to_exitcode(status)
                process.returncode = exit_code
                phase["wait_status_raw"] = status
                phase["usage"] = child_usage_report(usage)
            else:
                exit_code = process.wait()
                phase["usage"] = {
                    "available": False,
                    "reason": "os.wait4 is unavailable on this platform",
                }
        phase["exit_code"] = exit_code
        phase["state"] = "completed" if exit_code == 0 else "failed"
    except (KeyboardInterrupt, SystemExit):
        stop_owned_child(process)
        raise
    except Exception as error:
        stop_owned_child(process)
        phase["state"] = "failed"
        phase["launch_or_wait_error"] = f"{type(error).__name__}: {error}"
    phase["completed_at_utc"] = utc_now()
    phase["wall_ms"] = (time.perf_counter_ns() - wall_started) / 1_000_000
    phase["stdout_file"] = file_report(stdout_path)
    phase["stderr_file"] = file_report(stderr_path)
    if phase["state"] == "failed" and stderr_path.is_file():
        phase["stderr_tail"] = read_tail(stderr_path, 16 * 1024)
    return phase


def stop_owned_child(process: subprocess.Popen[bytes] | None) -> None:
    if process is None or process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def read_tail(path: Path, maximum_bytes: int) -> str:
    with path.open("rb") as handle:
        handle.seek(0, os.SEEK_END)
        length = handle.tell()
        handle.seek(max(0, length - maximum_bytes))
        return handle.read().decode("utf-8", errors="replace")


def child_usage_report(usage: resource.struct_rusage) -> dict[str, Any]:
    raw_rss = usage.ru_maxrss
    if sys.platform == "darwin":
        rss_bytes = int(raw_rss)
        rss_source_unit = "bytes"
    else:
        rss_bytes = int(raw_rss) * 1024
        rss_source_unit = "KiB converted to bytes"
    return {
        "available": True,
        "user_cpu_ms": usage.ru_utime * 1000,
        "system_cpu_ms": usage.ru_stime * 1000,
        "maximum_resident_set_size_bytes": rss_bytes,
        "ru_maxrss_source_unit": rss_source_unit,
        "rss_scope": "whole child process lifetime high-water mark",
    }


def attach_artifact(phase: dict[str, Any], artifact_path: Path) -> None:
    phase["artifact_path"] = str(artifact_path)
    if not artifact_path.is_file():
        phase["artifact_error"] = "expected JSON artifact is missing"
        return
    phase["artifact_file"] = file_report(artifact_path)
    try:
        with artifact_path.open() as handle:
            phase["artifact"] = json.load(handle)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        phase["artifact_error"] = f"{type(error).__name__}: {error}"


def phase_passed(phase: dict[str, Any] | None) -> bool:
    return bool(
        phase
        and phase.get("state") == "completed"
        and phase.get("exit_code") == 0
        and "artifact" in phase
        and "artifact_error" not in phase
    )


def update_case_passed(case: dict[str, Any]) -> None:
    case["all_passed"] = phase_passed(case.get("prepare")) and len(case["queries"]) == len(
        STRATEGIES
    ) and all(phase_passed(case["queries"].get(strategy)) for strategy in STRATEGIES)


def print_phase(case_name: str, phase_name: str, phase: dict[str, Any]) -> None:
    exit_text = phase.get("exit_code", "unavailable")
    print(
        f"{case_name} {phase_name}: state={phase['state']} exit={exit_text} "
        f"wall_ms={phase['wall_ms']:.3f}",
        flush=True,
    )


def environment_report(
    args: argparse.Namespace, script_path: Path, engine_path: Path
) -> dict[str, Any]:
    repo_root = script_path.parents[3]
    source_matrix = args.source_root / "matrix.json"
    sqlite_connection = sqlite3.connect(":memory:")
    try:
        sqlite_source_id = sqlite_connection.execute("SELECT sqlite_source_id()").fetchone()[0]
        sqlite_compile_options = [
            row[0] for row in sqlite_connection.execute("PRAGMA compile_options").fetchall()
        ]
    finally:
        sqlite_connection.close()
    return {
        "python": sys.version,
        "python_executable": sys.executable,
        "duckdb_version": duckdb.__version__,
        "pysqlite3_sqlite_version": sqlite3.sqlite_version,
        "sqlite_source_id": sqlite_source_id,
        "sqlite_compile_options": sqlite_compile_options,
        "os": {
            "platform": platform.platform(),
            "uname": list(platform.uname()),
            "mac_ver": list(platform.mac_ver()),
        },
        "cpu": cpu_report(),
        "physical_memory_bytes": physical_memory_bytes(),
        "git": git_report(repo_root),
        "source_root_matrix": file_report(source_matrix),
        "python_files": {
            str(path.relative_to(repo_root)): file_report(path)
            for path in sorted(script_path.parent.glob("*.py"))
        },
        "native_modules": {
            "_duckdb": module_file_report("_duckdb"),
            "pysqlite3._sqlite3": module_file_report("pysqlite3._sqlite3"),
        },
        "engine_script": file_report(engine_path),
    }


def assert_versions() -> None:
    if sqlite3.sqlite_version != REQUIRED_SQLITE_VERSION:
        raise SystemExit(
            f"pysqlite3 SQLite {REQUIRED_SQLITE_VERSION} is required; found {sqlite3.sqlite_version}"
        )
    if duckdb.__version__ != REQUIRED_DUCKDB_VERSION:
        raise SystemExit(
            f"DuckDB {REQUIRED_DUCKDB_VERSION} is required; found {duckdb.__version__}"
        )


def cpu_report() -> dict[str, Any]:
    report: dict[str, Any] = {
        "logical_count": os.cpu_count(),
        "platform_processor": platform.processor(),
        "machine": platform.machine(),
    }
    if sys.platform == "darwin":
        result = subprocess.run(
            ["sysctl", "-n", "machdep.cpu.brand_string"],
            capture_output=True,
            text=True,
            check=False,
        )
        report["brand"] = result.stdout.strip() if result.returncode == 0 else None
        if result.returncode != 0:
            report["brand_error"] = result.stderr.strip()
    elif Path("/proc/cpuinfo").is_file():
        for line in Path("/proc/cpuinfo").read_text(errors="replace").splitlines():
            if line.startswith("model name"):
                report["brand"] = line.partition(":")[2].strip()
                break
    return report


def physical_memory_bytes() -> int | None:
    if sys.platform == "darwin":
        result = subprocess.run(
            ["sysctl", "-n", "hw.memsize"], capture_output=True, text=True, check=False
        )
        if result.returncode == 0:
            return int(result.stdout.strip())
        return None
    page_size = os.sysconf("SC_PAGE_SIZE") if "SC_PAGE_SIZE" in os.sysconf_names else None
    pages = os.sysconf("SC_PHYS_PAGES") if "SC_PHYS_PAGES" in os.sysconf_names else None
    return int(page_size * pages) if page_size is not None and pages is not None else None


def git_report(repo_root: Path) -> dict[str, Any]:
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=repo_root, capture_output=True, text=True, check=False
    )
    status = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    return {
        "head": head.stdout.strip() if head.returncode == 0 else None,
        "head_error": head.stderr.strip() if head.returncode != 0 else None,
        "dirty": bool(status.stdout) if status.returncode == 0 else None,
        "status_porcelain": status.stdout.splitlines() if status.returncode == 0 else None,
        "status_error": status.stderr.strip() if status.returncode != 0 else None,
    }


def module_file_report(module_name: str) -> dict[str, Any]:
    module = importlib.import_module(module_name)
    path_value = getattr(module, "__file__", None)
    if path_value is None:
        return {"path": None, "error": "module has no __file__"}
    return file_report(Path(path_value).resolve())


def file_report(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {"path": str(path), "exists": False}
    return {
        "path": str(path),
        "exists": True,
        "size_bytes": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while block := handle.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def write_progress(output: Path, report: dict[str, Any]) -> None:
    destination = output / "matrix.json"
    temporary = output / ".matrix.json.tmp"
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    os.replace(temporary, destination)


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


if __name__ == "__main__":
    raise SystemExit(main())
