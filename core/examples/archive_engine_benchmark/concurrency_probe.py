#!/usr/bin/env python3
"""Focused append/read and process-crash probes for archive engine comparison.

This experiment creates only synthetic databases below the requested output
directory. It does not exercise production migration, recovery selection, or
power-failure durability.
"""

from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import resource
import subprocess
import sys
import threading
import time
from typing import Any, Callable

import pysqlite3 as sqlite3

try:
    import duckdb
except ImportError:
    duckdb = None


MEMORY_LIMIT = "128MB"
DUCKDB_THREADS = 2
EXPECTED_SQLITE_VERSION = "3.46.0"
SQLITE_TIMEOUT_SECONDS = 30.0
READY_TIMEOUT_SECONDS = 10.0
WORKER_STOP_TIMEOUT_SECONDS = 5.0
CRASH_COMMITTED_ID = 9_000_000_000_000_001
CRASH_IN_FLIGHT_ID = 9_000_000_000_000_002


def run_concurrency(
    output: Path, rows: int = 200_000, repetitions: int = 30
) -> dict[str, Any]:
    """Run synthetic SQLite/DuckDB concurrency and process-crash probes."""
    if rows <= 0:
        raise ValueError("rows must be positive")
    if repetitions <= 0:
        raise ValueError("repetitions must be positive")
    if sqlite3.sqlite_version != EXPECTED_SQLITE_VERSION:
        raise RuntimeError(
            "concurrency comparison requires pysqlite3 linked to SQLite "
            f"{EXPECTED_SQLITE_VERSION}, got {sqlite3.sqlite_version}"
        )

    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    run_dir = output / f"concurrency-probe-{os.getpid()}-{time.time_ns()}"
    run_dir.mkdir()
    process_before = _process_usage()
    started = time.perf_counter()

    sqlite_result = _guard_engine(
        "sqlite",
        lambda: _run_engine("sqlite", run_dir, rows, repetitions),
    )
    duckdb_result = _guard_engine(
        "duckdb",
        lambda: _run_engine("duckdb", run_dir, rows, repetitions),
    )

    process_after = _process_usage()
    return {
        "format": "hardware-archive-engine-concurrency-v1",
        "scope": "controlled synthetic fixture; source and application databases untouched",
        "run_directory": str(run_dir),
        "configuration": {
            "rows": rows,
            "append_transactions": repetitions,
            "duckdb_threads": DUCKDB_THREADS,
            "duckdb_memory_limit": MEMORY_LIMIT,
            "sqlite_journal_mode": "WAL",
            "sqlite_synchronous": "NORMAL",
            "sqlite_version": sqlite3.sqlite_version,
            "duckdb_version": None if duckdb is None else duckdb.__version__,
        },
        "engines": {
            "sqlite_wal_normal": sqlite_result,
            "duckdb_native": duckdb_result,
            "parquet": {
                "status": "not_tested",
                "transactional_append_tested": False,
                "reason": (
                    "Parquet files are immutable storage artifacts; an in-place "
                    "concurrent append transaction was not modeled."
                ),
                "required_for_comparison": (
                    "A generation publisher must write and verify a new file, "
                    "durably publish an authoritative manifest, preserve pinned "
                    "readers on the old generation, and test interruption before "
                    "and after manifest publication."
                ),
                "production_recovery_guarantee": False,
            },
        },
        "whole_process_resources": {
            "wall_ms": _ms(time.perf_counter() - started),
            "cpu_user_ms": _ms(process_after["user_s"] - process_before["user_s"]),
            "cpu_system_ms": _ms(
                process_after["system_s"] - process_before["system_s"]
            ),
            "max_rss_raw_before": process_before["max_rss_raw"],
            "max_rss_raw_after": process_after["max_rss_raw"],
            "max_rss_unit": process_after["max_rss_unit"],
            "qualifier": (
                "CPU and peak RSS are process-wide counters across fixture setup, "
                "both engines, threads, and crash orchestration; they are not "
                "isolated engine counters and exclude caller JSON serialization."
            ),
        },
        "durability_scope": (
            "Abrupt process termination is tested. OS crash, power loss, storage "
            "cache behavior, and a hard one-minute durability bound are not tested."
        ),
    }


def _guard_engine(name: str, action: Callable[[], dict[str, Any]]) -> dict[str, Any]:
    try:
        return action()
    except Exception as error:  # The report must retain the causal failure.
        return {
            "status": "error",
            "engine": name,
            "error_type": type(error).__name__,
            "error": str(error),
        }


def _run_engine(
    engine: str, run_dir: Path, rows: int, repetitions: int
) -> dict[str, Any]:
    if engine == "duckdb" and duckdb is None:
        raise RuntimeError("duckdb is not installed in this Python interpreter")

    suffix = "sqlite3" if engine == "sqlite" else "duckdb"
    database_path = run_dir / f"{engine}-concurrency.{suffix}"
    temp_dir = run_dir / f"{engine}-temp"
    temp_dir.mkdir()
    _create_fixture(engine, database_path, temp_dir, rows)
    resources_before = _process_usage()

    state: dict[str, Any] = {
        "reader_intervals_ns": [],
        "transaction_intervals_ns": [],
        "transaction_ms": [],
        "errors": [],
        "reader_before": None,
        "reader_during": None,
        "reader_groups": None,
        "reader_aggregate_stable": True,
        "reader_first_result": None,
        "artifacts_during_overlap": None,
    }
    reader_ready = threading.Barrier(2)
    query_active = threading.Event()
    writer_done = threading.Event()

    reader = threading.Thread(
        target=_reader,
        args=(
            engine,
            database_path,
            temp_dir,
            reader_ready,
            query_active,
            writer_done,
            state,
        ),
        name=f"{engine}-analytic-reader",
    )
    writer = threading.Thread(
        target=_writer,
        args=(
            engine,
            database_path,
            temp_dir,
            rows,
            repetitions,
            reader_ready,
            query_active,
            writer_done,
            state,
        ),
        name=f"{engine}-append-writer",
    )
    reader.start()
    writer.start()
    reader.join(timeout=SQLITE_TIMEOUT_SECONDS + 30.0)
    writer.join(timeout=SQLITE_TIMEOUT_SECONDS + 30.0)
    if reader.is_alive() or writer.is_alive():
        state["errors"].append("reader or writer thread exceeded bounded join timeout")
        writer_done.set()
        reader.join(timeout=1.0)
        writer.join(timeout=1.0)

    transaction_intervals = state["transaction_intervals_ns"]
    reader_intervals = state["reader_intervals_ns"]
    overlapping_transactions = sum(
        any(_overlaps(transaction, query) for query in reader_intervals)
        for transaction in transaction_intervals
    )
    resources_after = _process_usage()
    reopened = _verify_reopened(engine, database_path, temp_dir, rows, repetitions)
    crash = _run_crash_probes(engine, run_dir)
    sizes = _artifact_sizes(
        engine,
        database_path,
        temp_dir,
        "after concurrency connections close and reopen validation",
    )

    success_count = len(state["transaction_ms"])
    errors = list(state["errors"])
    if overlapping_transactions != success_count:
        errors.append(
            f"{success_count - overlapping_transactions} committed append transactions "
            "did not overlap an analytic GROUP BY interval"
        )
    if success_count != repetitions:
        errors.append(
            f"only {success_count} of {repetitions} append transactions committed"
        )

    reader_snapshot_consistent = (
        state["reader_before"] == rows
        and state["reader_during"] == rows
        and state["reader_aggregate_stable"]
        and reopened["row_count"] == rows + repetitions
    )
    if not reader_snapshot_consistent:
        errors.append(
            "reader transaction did not remain on the before snapshot or reopen "
            "did not observe the after snapshot"
        )
    if not reopened["appended_ids_exact"]:
        errors.append("reopen found missing or duplicate appended ids")
    if not crash["all_passed"]:
        errors.append("one or more process-crash probes failed")

    return {
        "status": "passed" if not errors else "error",
        "engine": engine,
        "engine_version": (
            sqlite3.sqlite_version if engine == "sqlite" else duckdb.__version__
        ),
        "errors": errors,
        "fixture_rows": rows,
        "analytic_query": (
            "SELECT source, COUNT(*), SUM(value), SUM(value * ((id % 17) + 1)) "
            "FROM samples GROUP BY source"
        ),
        "analytic_query_count": len(reader_intervals),
        "analytic_group_count": state["reader_groups"],
        "append_transactions_requested": repetitions,
        "append_transactions_succeeded": success_count,
        "append_shape": (
            "synthetic one-row append transaction; this does not establish timing "
            "or durability for the production minute append"
        ),
        "transaction_timing_ms": _timing_summary(state["transaction_ms"]),
        "overlap": {
            "transactions_overlapping_reader_query": overlapping_transactions,
            "all_transactions_overlapped": overlapping_transactions == repetitions,
            "evidence": (
                "Overlap is computed from monotonic start/end intervals around "
                "each analytic execute/fetch and each append transaction from "
                "before BEGIN through completed COMMIT. Commit duration is not "
                "instrumented separately."
            ),
        },
        "snapshot": {
            "reader_count_before_writes": state["reader_before"],
            "reader_count_after_writes_in_same_transaction": state["reader_during"],
            "analytic_result_stable_in_reader_transaction": state[
                "reader_aggregate_stable"
            ],
            "count_after_reopen": reopened["row_count"],
            "consistent_before_or_after": reader_snapshot_consistent,
        },
        "reopen_validation": reopened,
        "process_crash": crash,
        "artifacts_bytes_during_overlap": state["artifacts_during_overlap"],
        "artifacts_bytes_after_reopen": sizes,
        "process_resources_during_concurrency": {
            "cpu_user_ms": _ms(
                resources_after["user_s"] - resources_before["user_s"]
            ),
            "cpu_system_ms": _ms(
                resources_after["system_s"] - resources_before["system_s"]
            ),
            "max_rss_raw_before": resources_before["max_rss_raw"],
            "max_rss_raw_after": resources_after["max_rss_raw"],
            "max_rss_unit": resources_after["max_rss_unit"],
            "qualifier": (
                "Process-wide counters include both database threads and Python "
                "coordination during this engine interval."
            ),
        },
    }


def _reader(
    engine: str,
    path: Path,
    temp_dir: Path,
    ready: threading.Barrier,
    query_active: threading.Event,
    writer_done: threading.Event,
    state: dict[str, Any],
) -> None:
    connection = None
    try:
        connection = _connect(engine, path, temp_dir)
        _begin(connection)
        state["reader_before"] = _scalar(connection, "SELECT COUNT(*) FROM samples")
        ready.wait(timeout=READY_TIMEOUT_SECONDS)
        while not writer_done.is_set() or len(state["reader_intervals_ns"]) < 2:
            query_active.set()
            started = time.perf_counter_ns()
            result = connection.execute(
                "SELECT source, COUNT(*), SUM(value), "
                "SUM(value * ((id % 17) + 1)) "
                "FROM samples GROUP BY source"
            ).fetchall()
            finished = time.perf_counter_ns()
            query_active.clear()
            state["reader_intervals_ns"].append((started, finished))
            state["reader_groups"] = len(result)
            if state["reader_first_result"] is None:
                state["reader_first_result"] = result
            elif result != state["reader_first_result"]:
                state["reader_aggregate_stable"] = False
        state["reader_during"] = _scalar(connection, "SELECT COUNT(*) FROM samples")
        connection.commit()
    except Exception as error:
        query_active.set()
        state["errors"].append(f"reader {type(error).__name__}: {error}")
        try:
            ready.abort()
        except Exception:
            pass
    finally:
        query_active.clear()
        if connection is not None:
            connection.close()


def _writer(
    engine: str,
    path: Path,
    temp_dir: Path,
    rows: int,
    repetitions: int,
    ready: threading.Barrier,
    query_active: threading.Event,
    writer_done: threading.Event,
    state: dict[str, Any],
) -> None:
    connection = None
    try:
        connection = _connect(engine, path, temp_dir)
        ready.wait(timeout=READY_TIMEOUT_SECONDS)
        for offset in range(repetitions):
            if not query_active.wait(timeout=READY_TIMEOUT_SECONDS):
                raise RuntimeError("analytic reader never became active")
            started = time.perf_counter_ns()
            _begin(connection)
            connection.execute(
                "INSERT INTO samples (id, source, value) VALUES (?, ?, ?)",
                (rows + offset + 1, f"append-{offset % 4}", offset + 0.5),
            )
            connection.commit()
            finished = time.perf_counter_ns()
            state["transaction_intervals_ns"].append((started, finished))
            state["transaction_ms"].append((finished - started) / 1_000_000.0)
        state["artifacts_during_overlap"] = _artifact_sizes(
            engine,
            path,
            temp_dir,
            "writer finished while reader snapshot remained open",
        )
    except Exception as error:
        if connection is not None:
            try:
                connection.rollback()
            except Exception:
                pass
        state["errors"].append(f"writer {type(error).__name__}: {error}")
    finally:
        writer_done.set()
        if connection is not None:
            connection.close()


def _create_fixture(
    engine: str, path: Path, temp_dir: Path, rows: int
) -> None:
    connection = _connect(engine, path, temp_dir)
    try:
        connection.execute(
            "CREATE TABLE samples ("
            "id BIGINT PRIMARY KEY, source TEXT NOT NULL, value DOUBLE NOT NULL)"
        )
        if engine == "duckdb":
            _begin(connection)
            connection.execute(
                "INSERT INTO samples "
                "SELECT i, 'source-' || CAST(i % 64 AS VARCHAR), "
                "CAST((i * 17) % 10000 AS DOUBLE) / 100.0 "
                "FROM range(1, ?) AS generated(i)",
                (rows + 1,),
            )
        else:
            batch_size = 4_096
            for start in range(1, rows + 1, batch_size):
                end = min(rows + 1, start + batch_size)
                _begin(connection)
                connection.executemany(
                    "INSERT INTO samples (id, source, value) VALUES (?, ?, ?)",
                    (
                        (
                            identifier,
                            f"source-{identifier % 64}",
                            ((identifier * 17) % 10_000) / 100.0,
                        )
                        for identifier in range(start, end)
                    ),
                )
                connection.commit()
        connection.commit()
    finally:
        connection.close()


def _verify_reopened(
    engine: str, path: Path, temp_dir: Path, rows: int, repetitions: int
) -> dict[str, Any]:
    connection = _connect(engine, path, temp_dir)
    try:
        row_count = _scalar(connection, "SELECT COUNT(*) FROM samples")
        appended = connection.execute(
            "SELECT id FROM samples WHERE id > ? ORDER BY id", (rows,)
        ).fetchall()
        appended_ids = [int(row[0]) for row in appended]
        expected = list(range(rows + 1, rows + repetitions + 1))
        return {
            "row_count": row_count,
            "appended_row_count": len(appended_ids),
            "appended_distinct_id_count": len(set(appended_ids)),
            "appended_ids_exact": appended_ids == expected,
            "expected_first_id": expected[0],
            "expected_last_id": expected[-1],
        }
    finally:
        connection.close()


def _connect(engine: str, path: Path, temp_dir: Path):
    if engine == "sqlite":
        connection = sqlite3.connect(
            path, timeout=SQLITE_TIMEOUT_SECONDS, isolation_level=None
        )
        connection.execute("PRAGMA journal_mode = WAL")
        connection.execute("PRAGMA synchronous = NORMAL")
        connection.execute(f"PRAGMA busy_timeout = {int(SQLITE_TIMEOUT_SECONDS * 1000)}")
        connection.execute("PRAGMA temp_store = FILE")
        return connection
    if duckdb is None:
        raise RuntimeError("duckdb is not installed")
    connection = duckdb.connect(str(path))
    connection.execute(f"SET threads = {DUCKDB_THREADS}")
    connection.execute(f"SET memory_limit = '{MEMORY_LIMIT}'")
    escaped_temp = str(temp_dir).replace("'", "''")
    connection.execute(f"SET temp_directory = '{escaped_temp}'")
    return connection


def _begin(connection) -> None:
    connection.execute("BEGIN TRANSACTION")


def _scalar(connection, sql: str) -> int:
    row = connection.execute(sql).fetchone()
    if row is None:
        raise RuntimeError(f"scalar query returned no row: {sql}")
    return int(row[0])


def _overlaps(left: tuple[int, int], right: tuple[int, int]) -> bool:
    return left[0] < right[1] and right[0] < left[1]


def _timing_summary(values: list[float]) -> dict[str, Any]:
    ordered = sorted(values)
    if not ordered:
        return {
            "samples": 0,
            "all_ms": [],
            "first_ms": None,
            "median_ms": None,
            "p95_ms": None,
            "p99_ms": None,
            "max_ms": None,
        }
    return {
        "samples": len(ordered),
        "all_ms": values,
        "first_ms": values[0],
        "median_ms": _percentile(ordered, 50),
        "p95_ms": _percentile(ordered, 95),
        "p99_ms": _percentile(ordered, 99),
        "max_ms": ordered[-1],
    }


def _percentile(ordered: list[float], percentile: int) -> float:
    index = max(0, math.ceil(len(ordered) * percentile / 100) - 1)
    return ordered[index]


def _artifact_sizes(
    engine: str,
    database_path: Path,
    temp_dir: Path,
    measurement_point: str,
) -> dict[str, Any]:
    def size(path: Path) -> int:
        try:
            return path.stat().st_size
        except FileNotFoundError:
            return 0

    temp_files = {
        str(path.relative_to(temp_dir)): size(path)
        for path in sorted(temp_dir.rglob("*"))
        if path.is_file()
    }
    wal_path = (
        Path(f"{database_path}-wal")
        if engine == "sqlite"
        else Path(f"{database_path}.wal")
    )
    return {
        "database": size(database_path),
        "wal": size(wal_path),
        "shm": size(Path(f"{database_path}-shm")) if engine == "sqlite" else 0,
        "temp_total": sum(temp_files.values()),
        "temp_files": temp_files,
        "measurement_point": measurement_point,
    }


def _process_usage() -> dict[str, Any]:
    usage = resource.getrusage(resource.RUSAGE_SELF)
    return {
        "user_s": usage.ru_utime,
        "system_s": usage.ru_stime,
        "max_rss_raw": usage.ru_maxrss,
        "max_rss_unit": "bytes" if sys.platform == "darwin" else "KiB",
    }


def _run_crash_probes(engine: str, run_dir: Path) -> dict[str, Any]:
    suffix = "sqlite3" if engine == "sqlite" else "duckdb"
    path = run_dir / f"{engine}-process-crash.{suffix}"
    temp_dir = run_dir / f"{engine}-crash-temp"
    temp_dir.mkdir()
    connection = _connect(engine, path, temp_dir)
    try:
        connection.execute(
            "CREATE TABLE crash_rows (id BIGINT PRIMARY KEY, phase TEXT NOT NULL)"
        )
        connection.execute(
            "INSERT INTO crash_rows (id, phase) VALUES (1, 'baseline')"
        )
        connection.commit()
    finally:
        connection.close()

    committed = _kill_worker(engine, path, temp_dir, "committed")
    committed_visible = _crash_id_count(
        engine, path, temp_dir, CRASH_COMMITTED_ID
    ) == 1
    in_flight = _kill_worker(engine, path, temp_dir, "in_flight")
    in_flight_count = _crash_id_count(
        engine, path, temp_dir, CRASH_IN_FLIGHT_ID
    )
    return {
        "all_passed": (
            committed["killed_after_ready"]
            and committed_visible
            and in_flight["killed_after_ready"]
            and in_flight_count == 0
        ),
        "committed_append": {
            **committed,
            "visible_exactly_once_after_reopen": committed_visible,
        },
        "in_flight_append": {
            **in_flight,
            "visible_rows_after_reopen": in_flight_count,
            "partially_visible": in_flight_count != 0,
        },
        "scope": (
            "Abrupt process termination only; this does not establish power-failure "
            "durability or production generation recovery."
        ),
    }


def _kill_worker(
    engine: str, path: Path, temp_dir: Path, mode: str
) -> dict[str, Any]:
    ready = temp_dir / f"{mode}-{time.time_ns()}.ready"
    command = [
        sys.executable,
        str(Path(__file__).resolve()),
        "--crash-worker",
        engine,
        str(path),
        str(temp_dir),
        mode,
        str(ready),
    ]
    process = subprocess.Popen(
        command,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    deadline = time.monotonic() + READY_TIMEOUT_SECONDS
    while not ready.exists() and process.poll() is None and time.monotonic() < deadline:
        time.sleep(0.01)
    killed_after_ready = ready.exists() and process.poll() is None
    if process.poll() is None:
        process.kill()
    try:
        _, stderr = process.communicate(timeout=WORKER_STOP_TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        process.kill()
        _, stderr = process.communicate(timeout=WORKER_STOP_TIMEOUT_SECONDS)
    return {
        "mode": mode,
        "ready_observed": ready.exists(),
        "killed_after_ready": killed_after_ready,
        "return_code": process.returncode,
        "stderr": stderr.strip(),
    }


def _crash_id_count(
    engine: str, path: Path, temp_dir: Path, identifier: int
) -> int:
    connection = _connect(engine, path, temp_dir)
    try:
        return _scalar(
            connection,
            f"SELECT COUNT(*) FROM crash_rows WHERE id = {int(identifier)}",
        )
    finally:
        connection.close()


def _crash_worker(
    engine: str, path: Path, temp_dir: Path, mode: str, ready: Path
) -> None:
    connection = _connect(engine, path, temp_dir)
    identifier = (
        CRASH_COMMITTED_ID if mode == "committed" else CRASH_IN_FLIGHT_ID
    )
    _begin(connection)
    connection.execute(
        "INSERT INTO crash_rows (id, phase) VALUES (?, ?)", (identifier, mode)
    )
    if mode == "committed":
        connection.commit()
    elif mode != "in_flight":
        raise ValueError(f"unknown crash-worker mode: {mode}")
    with ready.open("w", encoding="utf-8") as marker:
        marker.write("ready\n")
        marker.flush()
        os.fsync(marker.fileno())
    while True:
        time.sleep(1.0)


def _ms(seconds: float) -> float:
    return seconds * 1_000.0


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run focused archive engine concurrency/crash probes."
    )
    parser.add_argument("--output", type=Path)
    parser.add_argument("--rows", type=int, default=200_000)
    parser.add_argument("--repetitions", type=int, default=30)
    parser.add_argument(
        "--crash-worker",
        nargs=5,
        metavar=("ENGINE", "DATABASE", "TEMP_DIR", "MODE", "READY"),
        help=argparse.SUPPRESS,
    )
    return parser.parse_args()


def main() -> None:
    args = _parse_args()
    if args.crash_worker:
        engine, database, temp_dir, mode, ready = args.crash_worker
        _crash_worker(
            engine, Path(database), Path(temp_dir), mode, Path(ready)
        )
        return
    if args.output is None:
        raise SystemExit("--output is required")
    print(
        json.dumps(
            run_concurrency(args.output, args.rows, args.repetitions),
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
