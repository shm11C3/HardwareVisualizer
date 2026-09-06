#!/usr/bin/env python3
"""Measure current idle memory of standalone archive-engine probe processes.

The Rust probes prepare fixtures outside measured children, emit one JSON line
at each idle boundary, flush, and block for one stdin line. This driver samples
Darwin's current resident size and physical footprint only while that handshake
is blocked. It never uses ru_maxrss as an idle-memory value.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import math
import os
import selectors
import shutil
import statistics
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

PROTOCOL_VERSION = 1
ENGINE_NAMES = {
    "baseline": "baseline",
    "sqlite": "sqlite_sqlx",
    "duckdb": "duckdb_bundled",
}
EXPECTED_STAGES = {
    "baseline": {"before_open", "after_close"},
    "empty": {"before_open", "open_empty", "post_query", "after_close"},
    "seeded": {"before_open", "open_seeded", "post_query", "after_close"},
}
RUSAGE_INFO_V0 = 0


class RUsageInfoV0(ctypes.Structure):
    _fields_ = [
        ("ri_uuid", ctypes.c_uint8 * 16),
        ("ri_user_time", ctypes.c_uint64),
        ("ri_system_time", ctypes.c_uint64),
        ("ri_pkg_idle_wkups", ctypes.c_uint64),
        ("ri_interrupt_wkups", ctypes.c_uint64),
        ("ri_pageins", ctypes.c_uint64),
        ("ri_wired_size", ctypes.c_uint64),
        ("ri_resident_size", ctypes.c_uint64),
        ("ri_phys_footprint", ctypes.c_uint64),
        ("ri_proc_start_abstime", ctypes.c_uint64),
        ("ri_proc_exit_abstime", ctypes.c_uint64),
    ]


@dataclass(frozen=True)
class ProbeBinary:
    key: str
    path: Path

    @property
    def reported_engine(self) -> str:
        return ENGINE_NAMES[self.key]


class CurrentMemoryReader:
    """Read current Darwin task memory fields documented by the local SDK."""

    def __init__(self) -> None:
        if sys.platform != "darwin":
            raise RuntimeError("current-memory probe currently requires macOS")
        self._libproc = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
        self._libproc.proc_pid_rusage.argtypes = [
            ctypes.c_int,
            ctypes.c_int,
            ctypes.c_void_p,
        ]
        self._libproc.proc_pid_rusage.restype = ctypes.c_int

    def read(self, pid: int) -> dict[str, int]:
        info = RUsageInfoV0()
        result = self._libproc.proc_pid_rusage(
            pid, RUSAGE_INFO_V0, ctypes.byref(info)
        )
        if result != 0:
            error_number = ctypes.get_errno()
            raise OSError(error_number, os.strerror(error_number), pid)
        return {
            "resident_size_bytes": int(info.ri_resident_size),
            "physical_footprint_bytes": int(info.ri_phys_footprint),
        }


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _run_prepare(binary: ProbeBinary, database: Path, seed_rows: int) -> dict[str, Any]:
    command = [
        str(binary.path),
        "--prepare",
        "--database",
        str(database),
        "--seed-rows",
        str(seed_rows),
    ]
    started = time.monotonic()
    completed = subprocess.run(
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=180,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"fixture preparation failed for {binary.key}: exit={completed.returncode}; "
            f"stderr={completed.stderr.strip()}"
        )
    auxiliary_paths = (
        Path(f"{database}-wal"),
        Path(f"{database}-shm"),
        Path(f"{database}.wal"),
    )
    auxiliary = {
        str(candidate): candidate.stat().st_size
        for candidate in auxiliary_paths
        if candidate.exists()
    }
    nonempty_wal = [
        candidate
        for candidate in (Path(f"{database}-wal"), Path(f"{database}.wal"))
        if candidate.exists() and candidate.stat().st_size > 0
    ]
    if nonempty_wal:
        raise RuntimeError(
            "prepared fixture retained nonempty WAL state: "
            + ", ".join(str(candidate) for candidate in nonempty_wal)
        )
    return {
        "engine": binary.reported_engine,
        "seed_rows": seed_rows,
        "ambient_rows": math.ceil(seed_rows / 10),
        "database": str(database),
        "database_bytes": database.stat().st_size,
        "database_sha256": _sha256(database),
        "auxiliary_files_after_prepare": auxiliary,
        "copy_policy": (
            "Copy the checkpointed main database only. A zero-byte WAL carries no "
            "committed pages; SQLite may retain a non-authoritative SHM index."
        ),
        "elapsed_ms_unmeasured_setup": (time.monotonic() - started) * 1000.0,
        "stdout": completed.stdout.strip(),
    }


def _readline_with_timeout(
    process: subprocess.Popen[str], selector: selectors.BaseSelector, timeout: float
) -> str | None:
    events = selector.select(timeout)
    if not events:
        raise TimeoutError(f"probe {process.pid} produced no stage within {timeout}s")
    line = process.stdout.readline() if process.stdout is not None else ""
    return line if line else None


def _sample_stage(
    reader: CurrentMemoryReader,
    pid: int,
    sample_count: int,
    interval_seconds: float,
    settle_seconds: float,
) -> list[dict[str, Any]]:
    time.sleep(settle_seconds)
    samples = []
    for index in range(sample_count):
        if index:
            time.sleep(interval_seconds)
        measured_at = time.time_ns()
        values = reader.read(pid)
        samples.append(
            {
                "sample_index": index,
                "measured_at_unix_ns": measured_at,
                **values,
            }
        )
    return samples


def _stage_summary(samples: list[dict[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {"sample_count": len(samples)}
    for field in ("resident_size_bytes", "physical_footprint_bytes"):
        values = [sample[field] for sample in samples]
        result[field] = {
            "minimum": min(values),
            "median": statistics.median(values),
            "maximum": max(values),
        }
    return result


def _measured_command(
    binary: ProbeBinary, database: Path | None, seed_rows: int
) -> list[str]:
    command = [str(binary.path)]
    if binary.key != "baseline":
        if database is None:
            raise ValueError("database is required for an engine probe")
        command.extend(
            ["--database", str(database), "--seed-rows", str(seed_rows)]
        )
    return command


def _run_measured_child(
    binary: ProbeBinary,
    mode: str,
    database: Path | None,
    seed_rows: int,
    expected_result_rows: int | None,
    repetition: int,
    reader: CurrentMemoryReader,
    sample_count: int,
    interval_seconds: float,
    settle_seconds: float,
    stage_timeout_seconds: float,
) -> dict[str, Any]:
    command = _measured_command(binary, database, seed_rows)
    process = subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    selector = selectors.DefaultSelector()
    if process.stdout is None or process.stdin is None:
        process.kill()
        raise RuntimeError("probe pipes were not created")
    selector.register(process.stdout, selectors.EVENT_READ)
    stages: list[dict[str, Any]] = []
    try:
        while True:
            line = _readline_with_timeout(process, selector, stage_timeout_seconds)
            if line is None:
                break
            try:
                state = json.loads(line)
            except json.JSONDecodeError as error:
                raise RuntimeError(
                    f"probe {process.pid} emitted non-JSON stdout: {line.rstrip()}"
                ) from error
            required = {
                "protocol_version",
                "engine",
                "stage",
                "pid",
                "seed_rows",
                "ambient_rows",
                "result_rows",
                "query_digest",
                "configuration",
            }
            missing = sorted(required - state.keys())
            if missing:
                raise RuntimeError(f"probe stage omitted fields: {missing}")
            if state["protocol_version"] != PROTOCOL_VERSION:
                raise RuntimeError(
                    f"protocol mismatch: {state['protocol_version']} != {PROTOCOL_VERSION}"
                )
            if state["engine"] != binary.reported_engine:
                raise RuntimeError(
                    f"engine mismatch: {state['engine']} != {binary.reported_engine}"
                )
            if state["pid"] != process.pid:
                raise RuntimeError(f"PID mismatch: {state['pid']} != {process.pid}")
            if state["seed_rows"] != seed_rows:
                raise RuntimeError(
                    f"seed-row mismatch: {state['seed_rows']} != {seed_rows}"
                )
            expected_ambient = 0 if binary.key == "baseline" else math.ceil(seed_rows / 10)
            if state["ambient_rows"] != expected_ambient:
                raise RuntimeError(
                    f"ambient-row mismatch: {state['ambient_rows']} != {expected_ambient}"
                )
            if state["stage"] == "post_query":
                if expected_result_rows is None:
                    raise RuntimeError("post_query was not expected for this case")
                if state["result_rows"] != expected_result_rows:
                    raise RuntimeError(
                        f"query-result count mismatch: {state[result_rows]} "
                        f"!= {expected_result_rows}"
                    )
            if any(stage["state"]["stage"] == state["stage"] for stage in stages):
                raise RuntimeError(f"duplicate stage: {state['stage']}")
            samples = _sample_stage(
                reader,
                process.pid,
                sample_count,
                interval_seconds,
                settle_seconds,
            )
            stages.append(
                {
                    "state": state,
                    "samples": samples,
                    "summary": _stage_summary(samples),
                }
            )
            process.stdin.write("\n")
            process.stdin.flush()
        return_code = process.wait(timeout=stage_timeout_seconds)
        stderr = process.stderr.read() if process.stderr is not None else ""
        if return_code != 0:
            raise RuntimeError(
                f"probe {process.pid} exited {return_code}: {stderr.strip()}"
            )
        observed_stages = {stage["state"]["stage"] for stage in stages}
        expected_stages = EXPECTED_STAGES[mode]
        if observed_stages != expected_stages:
            raise RuntimeError(
                f"stage mismatch for {binary.key}/{mode}: "
                f"observed={sorted(observed_stages)}, expected={sorted(expected_stages)}"
            )
        return {
            "engine": binary.reported_engine,
            "mode": mode,
            "repetition": repetition,
            "pid": process.pid,
            "command": command,
            "database": str(database) if database else None,
            "stages": stages,
            "stderr": stderr.strip(),
        }
    except BaseException:
        if process.poll() is None:
            process.kill()
            process.wait()
        raise
    finally:
        selector.close()
        if process.stdin is not None:
            process.stdin.close()
        if process.stdout is not None:
            process.stdout.close()
        if process.stderr is not None:
            process.stderr.close()


def _copy_fixture(template: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(template, destination)


def _case_summary(runs: list[dict[str, Any]]) -> dict[str, Any]:
    by_stage: dict[str, list[dict[str, Any]]] = {}
    for run in runs:
        for stage in run["stages"]:
            by_stage.setdefault(stage["state"]["stage"], []).append(stage)
    stage_results: dict[str, Any] = {}
    for stage_name, stages in sorted(by_stage.items()):
        result: dict[str, Any] = {"independent_processes": len(stages)}
        for field in ("resident_size_bytes", "physical_footprint_bytes"):
            process_medians = [stage["summary"][field]["median"] for stage in stages]
            result[field] = {
                "minimum_process_median": min(process_medians),
                "median_process_median": statistics.median(process_medians),
                "maximum_process_median": max(process_medians),
            }
        stage_results[stage_name] = result

    deltas: dict[str, dict[str, list[float]]] = {}
    for run in runs:
        stages = {stage["state"]["stage"]: stage for stage in run["stages"]}
        before = stages["before_open"]
        for stage_name, stage in stages.items():
            if stage_name == "before_open":
                continue
            target = deltas.setdefault(
                stage_name,
                {"resident_size_bytes": [], "physical_footprint_bytes": []},
            )
            for field in target:
                target[field].append(
                    stage["summary"][field]["median"]
                    - before["summary"][field]["median"]
                )
    delta_summary = {
        stage_name: {
            field: {
                "minimum": min(values),
                "median": statistics.median(values),
                "maximum": max(values),
            }
            for field, values in fields.items()
        }
        for stage_name, fields in deltas.items()
    }
    return {"stages": stage_results, "within_process_delta_from_before_open": delta_summary}


def _query_evidence(cases: dict[str, list[dict[str, Any]]]) -> dict[str, Any]:
    evidence: dict[str, Any] = {}
    for mode in ("empty", "seeded"):
        values: dict[str, list[dict[str, Any]]] = {}
        for engine in ("sqlite", "duckdb"):
            key = f"{engine}_{mode}"
            values[ENGINE_NAMES[engine]] = [
                {
                    "result_rows": stage["state"]["result_rows"],
                    "query_digest": stage["state"]["query_digest"],
                }
                for run in cases[key]
                for stage in run["stages"]
                if stage["state"]["stage"] == "post_query"
            ]
        canonical = {
            (item["result_rows"], item["query_digest"])
            for engine_values in values.values()
            for item in engine_values
        }
        evidence[mode] = {
            "observations": values,
            "cross_engine_and_repetition_match": len(canonical) == 1,
        }
    return evidence


def run_idle_memory_probe(
    output: Path,
    baseline_binary: Path,
    sqlite_binary: Path,
    duckdb_binary: Path,
    seed_rows: int = 100_000,
    expected_seeded_result_rows: int = 2_926,
    repetitions: int = 3,
    sample_count: int = 6,
    interval_seconds: float = 1.0,
    settle_seconds: float = 1.0,
    stage_timeout_seconds: float = 60.0,
) -> dict[str, Any]:
    """Prepare deterministic fixtures, then measure fresh idle probe processes."""
    if seed_rows < 65_000:
        raise ValueError("seed_rows must be at least 65,000 to cover the query range")
    if expected_seeded_result_rows <= 0:
        raise ValueError("expected seeded query result must be nonzero")
    if repetitions < 3:
        raise ValueError("at least three independent processes are required")
    if sample_count < 2:
        raise ValueError("at least two current-memory samples are required")
    if interval_seconds <= 0 or settle_seconds < 0:
        raise ValueError("invalid sampling interval or settle duration")

    binaries = {
        "baseline": ProbeBinary("baseline", baseline_binary.expanduser().resolve()),
        "sqlite": ProbeBinary("sqlite", sqlite_binary.expanduser().resolve()),
        "duckdb": ProbeBinary("duckdb", duckdb_binary.expanduser().resolve()),
    }
    for binary in binaries.values():
        if not binary.path.is_file() or not os.access(binary.path, os.X_OK):
            raise FileNotFoundError(f"probe binary is not executable: {binary.path}")

    output = output.expanduser().resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    artifact_directory = output.parent / f"idle-memory-{uuid.uuid4().hex}"
    templates = artifact_directory / "templates"
    instances = artifact_directory / "instances"
    templates.mkdir(parents=True)
    instances.mkdir()

    preparation: dict[str, dict[str, Any]] = {}
    template_paths: dict[tuple[str, str], Path] = {}
    for engine, extension in (("sqlite", "sqlite3"), ("duckdb", "duckdb")):
        for mode, rows in (("empty", 0), ("seeded", seed_rows)):
            template = templates / f"{engine}-{mode}.{extension}"
            template_paths[(engine, mode)] = template
            preparation[f"{engine}_{mode}"] = _run_prepare(
                binaries[engine], template, rows
            )

    reader = CurrentMemoryReader()
    cases: dict[str, list[dict[str, Any]]] = {"baseline": []}
    for repetition in range(repetitions):
        cases["baseline"].append(
            _run_measured_child(
                binaries["baseline"],
                "baseline",
                None,
                0,
                None,
                repetition,
                reader,
                sample_count,
                interval_seconds,
                settle_seconds,
                stage_timeout_seconds,
            )
        )
    for engine in ("sqlite", "duckdb"):
        for mode in ("empty", "seeded"):
            case_key = f"{engine}_{mode}"
            cases[case_key] = []
            rows = 0 if mode == "empty" else seed_rows
            for repetition in range(repetitions):
                extension = "sqlite3" if engine == "sqlite" else "duckdb"
                database = instances / case_key / f"run-{repetition}.{extension}"
                _copy_fixture(template_paths[(engine, mode)], database)
                cases[case_key].append(
                    _run_measured_child(
                        binaries[engine],
                        mode,
                        database,
                        rows,
                        0 if mode == "empty" else expected_seeded_result_rows,
                        repetition,
                        reader,
                        sample_count,
                        interval_seconds,
                        settle_seconds,
                        stage_timeout_seconds,
                    )
                )

    summaries = {key: _case_summary(runs) for key, runs in cases.items()}
    query_evidence = _query_evidence(cases)
    result = {
        "schema_version": 1,
        "completed": True,
        "measurement_passed": all(
            evidence["cross_engine_and_repetition_match"]
            for evidence in query_evidence.values()
        ),
        "production_memory_budget_accepted": False,
        "protocol": {
            "version": PROTOCOL_VERSION,
            "idle_definition": (
                "The child emitted and flushed its stage JSON, disposed the query result "
                "where applicable, then blocked reading exactly one stdin line."
            ),
            "fresh_processes_per_engine_mode": repetitions,
            "samples_per_stage": sample_count,
            "sample_interval_seconds": interval_seconds,
            "sampling_window_seconds": (sample_count - 1) * interval_seconds,
            "settle_seconds_before_first_sample": settle_seconds,
            "sequential_execution": True,
            "expected_query_result_rows": {
                "empty": 0,
                "seeded": expected_seeded_result_rows,
            },
        },
        "memory_source": {
            "api": "proc_pid_rusage(pid, RUSAGE_INFO_V0)",
            "fields": ["ri_resident_size", "ri_phys_footprint"],
            "units": "bytes",
            "semantics": "current snapshots at each sample; never ru_maxrss",
            "local_sdk_evidence": (
                "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/"
                "sys/resource.h and libproc.h"
            ),
        },
        "binaries": {
            key: {"path": str(binary.path), "reported_engine": binary.reported_engine}
            for key, binary in binaries.items()
        },
        "preparation": preparation,
        "artifact_directory": str(artifact_directory),
        "summaries": summaries,
        "query_evidence": query_evidence,
        "runs": cases,
        "limitations": [
            "These are standalone release-probe processes, not the HardwareVisualizer app.",
            "The three binaries share the probe runtime and protocol code but link different engine dependencies.",
            "Current resident size and physical footprint are sampled over time; samples within one process are not independent repetitions.",
            "Only differences from before_open within the same process are reported as deltas; absolute cross-process values retain host noise.",
            "Engine configuration and pool state come from the child stage JSON; the driver does not infer them.",
            "No DuckDB internal memory diagnostic query is issued because that query would alter the idle point; child configuration is still recorded.",
            "Fixture setup, application lifecycle, UI, migration, sustained retention, and OS/power-loss behavior are outside this measurement.",
        ],
    }
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--baseline-binary", required=True, type=Path)
    parser.add_argument("--sqlite-binary", required=True, type=Path)
    parser.add_argument("--duckdb-binary", required=True, type=Path)
    parser.add_argument("--seed-rows", type=int, default=100_000)
    parser.add_argument("--expected-seeded-result-rows", type=int, default=2_926)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--samples", type=int, default=6)
    parser.add_argument("--interval-seconds", type=float, default=1.0)
    parser.add_argument("--settle-seconds", type=float, default=1.0)
    parser.add_argument("--stage-timeout-seconds", type=float, default=60.0)
    arguments = parser.parse_args()
    result = run_idle_memory_probe(
        arguments.output,
        arguments.baseline_binary,
        arguments.sqlite_binary,
        arguments.duckdb_binary,
        arguments.seed_rows,
        arguments.expected_seeded_result_rows,
        arguments.repetitions,
        arguments.samples,
        arguments.interval_seconds,
        arguments.settle_seconds,
        arguments.stage_timeout_seconds,
    )
    print(
        json.dumps(
            {
                "completed": result["completed"],
                "measurement_passed": result["measurement_passed"],
                "output": str(arguments.output.expanduser().resolve()),
            },
            sort_keys=True,
        )
    )
    return 0 if result["measurement_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
