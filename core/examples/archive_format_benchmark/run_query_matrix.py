#!/usr/bin/env python3
"""Run synthetic query experiments serially and record child resources."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

parser = argparse.ArgumentParser()
parser.add_argument("--binary", type=Path, required=True)
parser.add_argument("--cwd", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--repetitions", type=int, default=7)
parser.add_argument("--cases", nargs="*")
args = parser.parse_args()
if sys.platform != "darwin":
    raise SystemExit("This runner records ru_maxrss as bytes and currently supports macOS only")
if args.repetitions <= 0:
    raise SystemExit("--repetitions must be positive")
args.binary = args.binary.resolve()
args.cwd = args.cwd.resolve()
args.output = args.output.resolve()
if not args.binary.is_file() or not os.access(args.binary, os.X_OK):
    raise SystemExit("--binary must name an existing executable file")
if not args.cwd.is_dir():
    raise SystemExit("--cwd must name an existing directory")
commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=args.cwd, text=True).strip()
status = subprocess.check_output(["git", "status", "--porcelain"], cwd=args.cwd, text=True)
if status:
    raise SystemExit("Refusing measurements from a dirty source worktree")
cases = [
    ("24h-stable", 1, "stable", 30), ("24h-churn", 1, "churn", 30),
    ("30d-stable", 30, "stable", 30), ("30d-churn", 30, "churn", 30),
    ("1y-stable", 365, "stable", 30), ("1y-churn", 365, "churn", 30),
    ("30d-churn-1m", 30, "churn", 1),
]
if args.cases:
    unknown = set(args.cases) - {case[0] for case in cases}
    if unknown:
        raise SystemExit(f"Unknown cases: {sorted(unknown)}")
    cases = [case for case in cases if case[0] in args.cases]
args.output.mkdir(parents=True, exist_ok=False)
result = {
    "format": "hardviz-archive-query-matrix-v1",
    "source_commit": commit,
    "base_benchmark_commit": "713add956faaca2a8d43dec8cf5c47cd43d30ebf",
    "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
    "started_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "method": "Serial direct-child os.wait4; macOS ru_maxrss is bytes; no concurrent builds or benchmarks.",
    "cases": [],
}
for name, days, workload, lifetime in cases:
    case_dir = args.output / name
    command = [str(args.binary), "--output", str(case_dir), "--days", str(days),
               "--query-experiment", "--process-workload", workload,
               "--process-lifetime-minutes", str(lifetime),
               "--repetitions", str(args.repetitions), "--group-cap", "1000000"]
    print(f"START {name}", flush=True)
    started = time.perf_counter()
    with (args.output / f"{name}.stdout.log").open("w") as out, (args.output / f"{name}.stderr.log").open("w") as err:
        child = subprocess.Popen(command, cwd=args.cwd, stdout=out, stderr=err)
        _, wait_status, usage = os.wait4(child.pid, 0)
        child.returncode = os.waitstatus_to_exitcode(wait_status)
    record = {
        "name": name, "arguments": command[1:], "exit_code": child.returncode,
        "wall_seconds": time.perf_counter() - started,
        "user_seconds": usage.ru_utime, "system_seconds": usage.ru_stime,
        "peak_rss_bytes": usage.ru_maxrss, "swaps": usage.ru_nswap,
    }
    report_path = case_dir / "report.json"
    if report_path.exists():
        record["report"] = json.loads(report_path.read_text())
    result["cases"].append(record)
    (args.output / "matrix.json").write_text(json.dumps(result, indent=2) + chr(10))
    print(f"END {name}: exit={child.returncode} wall={record['wall_seconds']:.3f}s peak_rss={usage.ru_maxrss}", flush=True)
    if child.returncode:
        raise SystemExit(f"Benchmark failed: {name}; inspect its stderr log")
