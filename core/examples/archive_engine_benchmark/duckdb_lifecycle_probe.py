#!/usr/bin/env python3
"""Bounded native-DuckDB lifecycle probe for archive-engine qualification.

This is an experimental, synthetic probe.  It does not open an application
database and does not implement a production migration or recovery protocol.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import resource
import signal
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

import duckdb
import pysqlite3 as sqlite3


EXPECTED_DUCKDB_VERSION = "1.5.5"
EXPECTED_SQLITE_VERSION = "3.46.0"
PROCESS_ROWS_PER_MINUTE = 15
AMBIENT_SOURCES = ("cpu-package", "gpu-edge")
CRASH_PROCESS_START = 9_000_000_000_000_000
CRASH_AMBIENT_START = 9_100_000_000_000_000
BASE_TIME = datetime(2026, 1, 1, tzinfo=timezone.utc)


def _assert_versions() -> dict[str, str]:
    versions = {
        "duckdb": duckdb.__version__,
        "sqlite": sqlite3.sqlite_version,
        "sqlite_binding": "pysqlite3",
    }
    if versions["duckdb"] != EXPECTED_DUCKDB_VERSION:
        raise RuntimeError(
            f"expected DuckDB {EXPECTED_DUCKDB_VERSION}, got {versions['duckdb']}"
        )
    if versions["sqlite"] != EXPECTED_SQLITE_VERSION:
        raise RuntimeError(
            f"expected SQLite {EXPECTED_SQLITE_VERSION}, got {versions['sqlite']}"
        )
    return versions


def _timestamp(minute: int, millisecond: int = 0) -> str:
    value = BASE_TIME + timedelta(minutes=minute, milliseconds=millisecond)
    return value.isoformat(timespec="milliseconds").replace("+00:00", "Z")


def _connect(database: Path, temp_directory: Path) -> duckdb.DuckDBPyConnection:
    temp_directory.mkdir(parents=True, exist_ok=True)
    connection = duckdb.connect(str(database))
    connection.execute("SET threads = 2")
    connection.execute("SET memory_limit = '128MB'")
    escaped_temp_directory = str(temp_directory).replace("'", "''")
    connection.execute(f"SET temp_directory = '{escaped_temp_directory}'")
    return connection


def _create_schema(
    connection: duckdb.DuckDBPyConnection,
    process_sequence_start: int,
    ambient_sequence_start: int,
) -> None:
    connection.execute(
        f"CREATE SEQUENCE process_id_seq START {process_sequence_start}"
    )
    connection.execute(
        f"CREATE SEQUENCE ambient_id_seq START {ambient_sequence_start}"
    )
    connection.execute(
        """
        CREATE TABLE process_stats (
            id BIGINT PRIMARY KEY DEFAULT nextval('process_id_seq'),
            pid BIGINT NOT NULL,
            process_name VARCHAR NOT NULL,
            cpu_usage DOUBLE NOT NULL,
            memory_usage BIGINT NOT NULL,
            execution_sec BIGINT NOT NULL,
            timestamp VARCHAR NOT NULL
        )
        """
    )
    connection.execute(
        """
        CREATE TABLE ambient_archive (
            id BIGINT PRIMARY KEY DEFAULT nextval('ambient_id_seq'),
            source VARCHAR NOT NULL,
            temperature DOUBLE NOT NULL,
            humidity DOUBLE,
            timestamp VARCHAR NOT NULL
        )
        """
    )


def _process_values(minute: int) -> list[tuple[int, str, float, int, int]]:
    values = []
    for rank in range(PROCESS_ROWS_PER_MINUTE):
        pid = 1_000 + rank
        # The tuple, not pid alone, is the identity used by the production query.
        name = f"synthetic-worker-{rank:02d}"
        values.append(
            (
                pid,
                name,
                float((minute * 7 + rank * 11) % 100) + 0.25,
                32_000_000 + rank * 1_000_000 + minute * 1_024,
                minute * 60 + rank,
            )
        )
    return values


def _ambient_values(minute: int) -> list[tuple[str, float, float | None]]:
    # Missing ambient sources are represented by absent rows.
    if minute % 7 == 0:
        return []
    return [
        ("cpu-package", 42.0 + minute % 9, 0.40 if minute % 3 else None),
        ("gpu-edge", 39.5 + minute % 7, None),
    ]


def _insert_minute(
    connection: duckdb.DuckDBPyConnection,
    minute: int,
    timestamp: str,
) -> dict[str, Any]:
    process_ids: list[int] = []
    ambient_ids: list[int] = []
    for pid, name, cpu, memory, execution in _process_values(minute):
        row = connection.execute(
            """
            INSERT INTO process_stats
                (pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            """,
            [pid, name, cpu, memory, execution, timestamp],
        ).fetchone()
        process_ids.append(int(row[0]))
    for source, temperature, humidity in _ambient_values(minute):
        row = connection.execute(
            """
            INSERT INTO ambient_archive (source, temperature, humidity, timestamp)
            VALUES (?, ?, ?, ?)
            RETURNING id
            """,
            [source, temperature, humidity, timestamp],
        ).fetchone()
        ambient_ids.append(int(row[0]))
    return {
        "minute": minute,
        "timestamp": timestamp,
        "process_ids": process_ids,
        "ambient_ids": ambient_ids,
    }


def _seed_history(
    connection: duckdb.DuckDBPyConnection, process_rows: int
) -> dict[str, int]:
    ambient_rows = max(1, math.ceil(process_rows / 10))
    process_batch: list[tuple[Any, ...]] = []
    ambient_batch: list[tuple[Any, ...]] = []
    connection.execute("BEGIN TRANSACTION")
    try:
        for index in range(process_rows):
            minute = index // PROCESS_ROWS_PER_MINUTE
            pid = 1_000 + index % PROCESS_ROWS_PER_MINUTE
            process_batch.append(
                (
                    index + 1,
                    pid,
                    f"synthetic-worker-{index % PROCESS_ROWS_PER_MINUTE:02d}",
                    float((index * 13) % 100) + 0.125,
                    32_000_000 + index % 4_096 * 4_096,
                    minute * 60 + index % PROCESS_ROWS_PER_MINUTE,
                    _timestamp(-minute, index % 997),
                )
            )
            if len(process_batch) == 5_000:
                connection.executemany(
                    "INSERT INTO process_stats VALUES (?, ?, ?, ?, ?, ?, ?)",
                    process_batch,
                )
                process_batch.clear()
        if process_batch:
            connection.executemany(
                "INSERT INTO process_stats VALUES (?, ?, ?, ?, ?, ?, ?)",
                process_batch,
            )
        for index in range(ambient_rows):
            minute = index // len(AMBIENT_SOURCES)
            ambient_batch.append(
                (
                    index + 1,
                    AMBIENT_SOURCES[index % len(AMBIENT_SOURCES)],
                    38.0 + index % 17 * 0.5,
                    None if index % 3 else 0.35 + index % 5 * 0.01,
                    _timestamp(-minute, index % 991),
                )
            )
            if len(ambient_batch) == 5_000:
                connection.executemany(
                    "INSERT INTO ambient_archive VALUES (?, ?, ?, ?, ?)",
                    ambient_batch,
                )
                ambient_batch.clear()
        if ambient_batch:
            connection.executemany(
                "INSERT INTO ambient_archive VALUES (?, ?, ?, ?, ?)",
                ambient_batch,
            )
        connection.execute("COMMIT")
    except BaseException:
        connection.execute("ROLLBACK")
        raise
    return {"process_rows": process_rows, "ambient_rows": ambient_rows}


def _snapshot(connection: duckdb.DuckDBPyConnection) -> dict[str, Any]:
    process = connection.execute(
        "SELECT count(*), min(id), max(id) FROM process_stats"
    ).fetchone()
    ambient = connection.execute(
        "SELECT count(*), min(id), max(id) FROM ambient_archive"
    ).fetchone()
    return {
        "process_rows": int(process[0]),
        "process_min_id": None if process[1] is None else int(process[1]),
        "process_max_id": None if process[2] is None else int(process[2]),
        "ambient_rows": int(ambient[0]),
        "ambient_min_id": None if ambient[1] is None else int(ambient[1]),
        "ambient_max_id": None if ambient[2] is None else int(ambient[2]),
    }


def _validate_seeded_history(
    connection: duckdb.DuckDBPyConnection,
    process_rows: int,
    ambient_rows: int,
) -> dict[str, Any]:
    mismatches: list[str] = []
    cursor = connection.execute(
        """
        SELECT id, pid, process_name, timestamp
        FROM process_stats
        WHERE id <= ?
        ORDER BY id
        """,
        [process_rows],
    )
    checked_process = 0
    while batch := cursor.fetchmany(5_000):
        for identifier, pid, name, timestamp in batch:
            index = int(identifier) - 1
            expected = (
                int(identifier),
                1_000 + index % PROCESS_ROWS_PER_MINUTE,
                f"synthetic-worker-{index % PROCESS_ROWS_PER_MINUTE:02d}",
                _timestamp(
                    -(index // PROCESS_ROWS_PER_MINUTE),
                    index % 997,
                ),
            )
            actual = (int(identifier), int(pid), str(name), str(timestamp))
            if actual != expected and len(mismatches) < 10:
                mismatches.append(f"Process row {identifier}: {actual!r} != {expected!r}")
            checked_process += 1

    cursor = connection.execute(
        """
        SELECT id, source, timestamp
        FROM ambient_archive
        WHERE id <= ?
        ORDER BY id
        """,
        [ambient_rows],
    )
    checked_ambient = 0
    while batch := cursor.fetchmany(5_000):
        for identifier, source, timestamp in batch:
            index = int(identifier) - 1
            expected = (
                int(identifier),
                AMBIENT_SOURCES[index % len(AMBIENT_SOURCES)],
                _timestamp(-(index // len(AMBIENT_SOURCES)), index % 991),
            )
            actual = (int(identifier), str(source), str(timestamp))
            if actual != expected and len(mismatches) < 10:
                mismatches.append(f"Ambient row {identifier}: {actual!r} != {expected!r}")
            checked_ambient += 1

    if checked_process != process_rows:
        mismatches.append(
            f"checked {checked_process} historical Process rows, expected {process_rows}"
        )
    if checked_ambient != ambient_rows:
        mismatches.append(
            f"checked {checked_ambient} historical Ambient rows, expected {ambient_rows}"
        )
    return {
        "pass": not mismatches,
        "checked_process_rows": checked_process,
        "checked_ambient_rows": checked_ambient,
        "mismatches": mismatches,
        "fields_checked": ["id", "pid", "process_name", "timestamp", "source"],
    }


def _analytic_query(connection: duckdb.DuckDBPyConnection) -> list[tuple[Any, ...]]:
    # Repeating the relevant rows makes the synthetic query long enough for
    # overlap observation without changing the production-shaped grouping key.
    return connection.execute(
        """
        WITH amplified AS (
            SELECT p.pid, p.process_name, p.cpu_usage, p.memory_usage,
                   p.execution_sec, p.timestamp
            FROM process_stats AS p
            CROSS JOIN range(8) AS amplification(copy)
        )
        SELECT pid, process_name, avg(cpu_usage), avg(memory_usage),
               max(execution_sec), max(timestamp), count(*)
        FROM amplified
        GROUP BY pid, process_name
        ORDER BY pid, process_name
        """
    ).fetchall()


def _percentile(values: list[float], percentile: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    rank = (len(ordered) - 1) * percentile
    lower = math.floor(rank)
    upper = math.ceil(rank)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] + (ordered[upper] - ordered[lower]) * (rank - lower)


def _timing_summary(values: list[float]) -> dict[str, Any]:
    return {
        "count": len(values),
        "first_ms": values[0] if values else None,
        "median_ms": statistics.median(values) if values else None,
        "p95_ms": _percentile(values, 0.95),
        "p99_ms": _percentile(values, 0.99),
        "max_ms": max(values) if values else None,
    }


def _intervals_overlap(left: tuple[float, float], right: tuple[float, float]) -> bool:
    return left[0] < right[1] and right[0] < left[1]


def _artifact_sizes(database: Path, temp_directory: Path) -> dict[str, int]:
    sizes: dict[str, int] = {}
    for label, path in (
        ("database_bytes", database),
        ("wal_bytes", Path(f"{database}.wal")),
    ):
        sizes[label] = path.stat().st_size if path.exists() else 0
    sizes["temp_bytes"] = sum(
        path.stat().st_size
        for path in temp_directory.rglob("*")
        if path.is_file()
    )
    return sizes


def _expected_minute_rows(
    batch: dict[str, Any],
) -> tuple[list[tuple[Any, ...]], list[tuple[Any, ...]]]:
    process_rows = [
        (identifier, *values, batch["timestamp"])
        for identifier, values in zip(
            batch["process_ids"], _process_values(batch["minute"]), strict=True
        )
    ]
    ambient_rows = [
        (identifier, *values, batch["timestamp"])
        for identifier, values in zip(
            batch["ambient_ids"], _ambient_values(batch["minute"]), strict=True
        )
    ]
    return process_rows, ambient_rows


def _all_rows(
    connection: duckdb.DuckDBPyConnection,
) -> tuple[list[tuple[Any, ...]], list[tuple[Any, ...]]]:
    process_rows = connection.execute(
        """
        SELECT id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp
        FROM process_stats
        ORDER BY id
        """
    ).fetchall()
    ambient_rows = connection.execute(
        """
        SELECT id, source, temperature, humidity, timestamp
        FROM ambient_archive
        ORDER BY id
        """
    ).fetchall()
    return process_rows, ambient_rows


def _rows_digest(rows: list[tuple[Any, ...]]) -> str:
    digest = hashlib.sha256()
    for row in rows:
        digest.update(
            json.dumps(row, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        )
        digest.update(b"\n")
    return digest.hexdigest()


def _run_concurrent_lifecycle(
    directory: Path, rows: int, repetitions: int
) -> dict[str, Any]:
    database = directory / "archive.duckdb"
    temp_directory = directory / "duckdb-temp"
    setup = _connect(database, temp_directory)
    _create_schema(setup, rows + 1, max(1, math.ceil(rows / 10)) + 1)
    seeded = _seed_history(setup, rows)
    setup.execute("CHECKPOINT")
    before = _snapshot(setup)
    setup.close()

    reader = _connect(database, temp_directory)
    writer = _connect(database, temp_directory)
    reader.execute("BEGIN TRANSACTION")
    pinned_before = _snapshot(reader)
    baseline_groups = _analytic_query(reader)

    requested = [-1]
    request_lock = threading.Lock()
    query_started = [threading.Event() for _ in range(repetitions)]
    writer_done = threading.Event()
    reader_ready = threading.Event()
    query_intervals: list[tuple[float, float]] = []
    transaction_intervals: list[tuple[float, float]] = []
    transaction_ms: list[float] = []
    batches: list[dict[str, Any]] = []
    errors: list[str] = []

    def read_loop() -> None:
        reader_ready.set()
        observed_request = -1
        try:
            while not writer_done.is_set():
                with request_lock:
                    current = requested[0]
                if current <= observed_request:
                    time.sleep(0.0005)
                    continue
                observed_request = current
                start = time.monotonic()
                query_started[current].set()
                groups = _analytic_query(reader)
                end = time.monotonic()
                query_intervals.append((start, end))
                if groups != baseline_groups:
                    errors.append(
                        f"pinned reader result changed during transaction {current}"
                    )
        except BaseException as error:
            errors.append(f"reader failed: {type(error).__name__}: {error}")
            writer_done.set()

    thread = threading.Thread(target=read_loop, name="duckdb-pinned-reader")
    thread.start()
    if not reader_ready.wait(timeout=10):
        errors.append("reader did not become ready within 10 seconds")
        writer_done.set()

    if not errors:
        for repetition in range(repetitions):
            transaction_start = time.monotonic()
            try:
                writer.execute("BEGIN TRANSACTION")
                batch = _insert_minute(
                    writer,
                    repetition + 1,
                    _timestamp(repetition + 1, (repetition * 37) % 1_000),
                )
                with request_lock:
                    requested[0] = repetition
                if not query_started[repetition].wait(timeout=30):
                    writer.execute("ROLLBACK")
                    errors.append(
                        f"reader query did not start for transaction {repetition}"
                    )
                    break
                writer.execute("COMMIT")
                transaction_end = time.monotonic()
                batches.append(batch)
                transaction_intervals.append((transaction_start, transaction_end))
                transaction_ms.append((transaction_end - transaction_start) * 1_000)
            except BaseException as error:
                try:
                    writer.execute("ROLLBACK")
                except BaseException:
                    pass
                errors.append(
                    f"writer transaction {repetition} failed: "
                    f"{type(error).__name__}: {error}"
                )
                break

    writer_done.set()
    thread.join(timeout=30)
    if thread.is_alive():
        errors.append("reader thread did not finish within 30 seconds")

    pinned_after = _snapshot(reader)
    reader.execute("COMMIT")
    reader.close()
    writer.close()

    reopened = _connect(database, temp_directory)
    after = _snapshot(reopened)
    expected_process_ids = [
        identifier for batch in batches for identifier in batch["process_ids"]
    ]
    expected_ambient_ids = [
        identifier for batch in batches for identifier in batch["ambient_ids"]
    ]
    actual_process_ids = [
        int(row[0])
        for row in reopened.execute(
            "SELECT id FROM process_stats WHERE id > ? ORDER BY id", [rows]
        ).fetchall()
    ]
    seeded_ambient_rows = seeded["ambient_rows"]
    actual_ambient_ids = [
        int(row[0])
        for row in reopened.execute(
            "SELECT id FROM ambient_archive WHERE id > ? ORDER BY id",
            [seeded_ambient_rows],
        ).fetchall()
    ]
    minute_checks = []
    for batch in batches:
        process_count = int(
            reopened.execute(
                "SELECT count(*) FROM process_stats WHERE timestamp = ?",
                [batch["timestamp"]],
            ).fetchone()[0]
        )
        ambient_count = int(
            reopened.execute(
                "SELECT count(*) FROM ambient_archive WHERE timestamp = ?",
                [batch["timestamp"]],
            ).fetchone()[0]
        )
        minute_checks.append(
            {
                "timestamp": batch["timestamp"],
                "process_rows": process_count,
                "ambient_rows": ambient_count,
                "expected_process_rows": len(batch["process_ids"]),
                "expected_ambient_rows": len(batch["ambient_ids"]),
                "matches": process_count == len(batch["process_ids"])
                and ambient_count == len(batch["ambient_ids"]),
            }
        )
    seeded_history = _validate_seeded_history(
        reopened, rows, seeded_ambient_rows
    )
    sizes = _artifact_sizes(database, temp_directory)
    reopened.close()

    overlaps = []
    for transaction_interval in transaction_intervals:
        overlaps.append(
            any(
                _intervals_overlap(transaction_interval, query_interval)
                for query_interval in query_intervals
            )
        )
    validation_errors = list(errors)
    if pinned_before != pinned_after:
        validation_errors.append("pinned reader row counts changed")
    if actual_process_ids != expected_process_ids:
        validation_errors.append("appended Process IDs were missing, duplicated, or reordered")
    if actual_ambient_ids != expected_ambient_ids:
        validation_errors.append("appended Ambient IDs were missing, duplicated, or reordered")
    if not all(check["matches"] for check in minute_checks):
        validation_errors.append("a committed cross-family minute was incomplete")
    if not seeded_history["pass"]:
        validation_errors.append("seeded original IDs, timestamps, or identities changed")
    if len(transaction_intervals) != repetitions:
        validation_errors.append(
            f"only {len(transaction_intervals)} of {repetitions} transactions committed"
        )
    if len(overlaps) != repetitions or not all(overlaps):
        validation_errors.append(
            "one or more writer transactions had no measured overlap with an analytic query"
        )

    return {
        "pass": not validation_errors,
        "errors": validation_errors,
        "database": str(database),
        "seeded": seeded,
        "before": before,
        "pinned_reader_before": pinned_before,
        "pinned_reader_after": pinned_after,
        "reopened_after": after,
        "committed_minutes": len(batches),
        "requested_minutes": repetitions,
        "transaction_timing": _timing_summary(transaction_ms),
        "transaction_timing_qualifier": (
            "Each measured transaction includes the synchronization wait for its "
            "analytic reader query to start; it is not production append latency."
        ),
        "analytic_queries": len(query_intervals),
        "analytic_query_timing": _timing_summary(
            [(end - start) * 1_000 for start, end in query_intervals]
        ),
        "overlapping_transactions": sum(overlaps),
        "all_transactions_overlapped_query": len(overlaps) == repetitions
        and all(overlaps),
        "minute_atomicity": minute_checks,
        "seeded_history_exact_validation": seeded_history,
        "artifacts": sizes,
        "query": {
            "grouping_identity": ["pid", "process_name"],
            "aggregates": [
                "avg(cpu_usage)",
                "avg(memory_usage)",
                "max(execution_sec)",
                "max(timestamp)",
            ],
            "synthetic_amplification": 8,
        },
    }


def _crash_worker(
    database: Path, temp_directory: Path, mode: str, marker: Path
) -> None:
    connection = _connect(database, temp_directory)
    connection.execute("BEGIN TRANSACTION")
    batch = _insert_minute(connection, 101, _timestamp(101, 123))
    if mode == "committed":
        connection.execute("COMMIT")
    elif mode != "in-flight":
        raise ValueError(f"unknown crash mode: {mode}")
    temporary_marker = marker.with_suffix(".tmp")
    temporary_marker.write_text(json.dumps(batch), encoding="utf-8")
    os.replace(temporary_marker, marker)
    # The parent kills this process.  The in-flight transaction stays open.
    while True:
        time.sleep(1)


def _wait_for_marker(marker: Path, process: subprocess.Popen[Any]) -> dict[str, Any]:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if marker.exists():
            return json.loads(marker.read_text(encoding="utf-8"))
        if process.poll() is not None:
            raise RuntimeError(
                f"crash worker exited before marker: status {process.returncode}"
            )
        time.sleep(0.02)
    raise TimeoutError("crash worker did not publish marker within 30 seconds")


def _run_one_crash_case(directory: Path, mode: str) -> dict[str, Any]:
    database = directory / "archive.duckdb"
    temp_directory = directory / "duckdb-temp"
    marker = directory / "ready.json"
    setup = _connect(database, temp_directory)
    _create_schema(setup, CRASH_PROCESS_START, CRASH_AMBIENT_START)
    setup.execute("BEGIN TRANSACTION")
    baseline = _insert_minute(setup, 100, _timestamp(100, 111))
    setup.execute("COMMIT")
    setup.close()

    command = [
        sys.executable,
        str(Path(__file__).resolve()),
        "--crash-worker",
        str(database),
        str(temp_directory),
        mode,
        str(marker),
    ]
    process = subprocess.Popen(
        command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True
    )
    worker_batch: dict[str, Any] | None = None
    stderr = ""
    try:
        worker_batch = _wait_for_marker(marker, process)
        os.kill(process.pid, signal.SIGKILL)
        _, stderr = process.communicate(timeout=10)
    finally:
        if process.poll() is None:
            process.kill()
            _, stderr = process.communicate(timeout=10)

    reopened = _connect(database, temp_directory)
    timestamp = worker_batch["timestamp"]
    visible_process = [
        int(row[0])
        for row in reopened.execute(
            "SELECT id FROM process_stats WHERE timestamp = ? ORDER BY id",
            [timestamp],
        ).fetchall()
    ]
    visible_ambient = [
        int(row[0])
        for row in reopened.execute(
            "SELECT id FROM ambient_archive WHERE timestamp = ? ORDER BY id",
            [timestamp],
        ).fetchall()
    ]
    expected_visible = mode == "committed"
    visibility_ok = (
        visible_process == worker_batch["process_ids"]
        and visible_ambient == worker_batch["ambient_ids"]
        if expected_visible
        else not visible_process and not visible_ambient
    )
    baseline_process, baseline_ambient = _expected_minute_rows(baseline)
    worker_process, worker_ambient = _expected_minute_rows(worker_batch)
    expected_process = baseline_process + (worker_process if expected_visible else [])
    expected_ambient = baseline_ambient + (worker_ambient if expected_visible else [])
    reopened_process, reopened_ambient = _all_rows(reopened)
    complete_reopen_ok = (
        reopened_process == expected_process and reopened_ambient == expected_ambient
    )

    # Allocate after restart through sequences.  Gaps are recorded, not rejected:
    # transactional sequence behavior must not be replaced by MAX(id) + 1.
    reopened.execute("BEGIN TRANSACTION")
    post_restart = _insert_minute(reopened, 102, _timestamp(102, 234))
    reopened.execute("COMMIT")
    post_process, post_ambient = _expected_minute_rows(post_restart)
    expected_process += post_process
    expected_ambient += post_ambient
    after_restart_process, after_restart_ambient = _all_rows(reopened)
    after_restart_exact = (
        after_restart_process == expected_process
        and after_restart_ambient == expected_ambient
    )
    process_ids = [int(row[0]) for row in after_restart_process]
    ambient_ids = [int(row[0]) for row in after_restart_ambient]
    unique = len(process_ids) == len(set(process_ids)) and len(ambient_ids) == len(
        set(ambient_ids)
    )

    deleted_process_id = max(process_ids)
    deleted_ambient_id = max(ambient_ids)
    reopened.execute("BEGIN TRANSACTION")
    reopened.execute("DELETE FROM process_stats WHERE id = ?", [deleted_process_id])
    reopened.execute("DELETE FROM ambient_archive WHERE id = ?", [deleted_ambient_id])
    reopened.execute("COMMIT")
    reopened.close()

    allocation_connection = _connect(database, temp_directory)
    allocation_connection.execute("BEGIN TRANSACTION")
    after_highest_delete = _insert_minute(
        allocation_connection, 103, _timestamp(103, 345)
    )
    allocation_connection.execute("COMMIT")
    allocated_process, allocated_ambient = _expected_minute_rows(after_highest_delete)
    final_process, final_ambient = _all_rows(allocation_connection)
    expected_final_process = [
        row for row in expected_process if row[0] != deleted_process_id
    ] + allocated_process
    expected_final_ambient = [
        row for row in expected_ambient if row[0] != deleted_ambient_id
    ] + allocated_ambient
    final_exact = (
        final_process == expected_final_process and final_ambient == expected_final_ambient
    )
    highest_id_not_reused = (
        min(after_highest_delete["process_ids"]) > deleted_process_id
        and min(after_highest_delete["ambient_ids"]) > deleted_ambient_id
    )
    final_unique = len(final_process) == len({row[0] for row in final_process}) and len(
        final_ambient
    ) == len({row[0] for row in final_ambient})
    allocation_connection.close()
    baseline_last_process = max(baseline["process_ids"])
    baseline_last_ambient = (
        max(baseline["ambient_ids"])
        if baseline["ambient_ids"]
        else CRASH_AMBIENT_START - 1
    )
    return {
        "pass": (
            visibility_ok
            and complete_reopen_ok
            and after_restart_exact
            and unique
            and highest_id_not_reused
            and final_unique
            and final_exact
        ),
        "mode": mode,
        "worker_exit": process.returncode,
        "worker_stderr": stderr,
        "expected_batch_visible": expected_visible,
        "visibility_ok": visibility_ok,
        "complete_reopen_rows_and_fields_ok": complete_reopen_ok,
        "complete_reopen_process_digest": _rows_digest(reopened_process),
        "complete_reopen_ambient_digest": _rows_digest(reopened_ambient),
        "visible_process_ids": visible_process,
        "visible_ambient_ids": visible_ambient,
        "post_restart_process_ids": post_restart["process_ids"],
        "post_restart_ambient_ids": post_restart["ambient_ids"],
        "post_restart_complete_rows_and_fields_ok": after_restart_exact,
        "all_ids_unique": unique,
        "observed_process_gap_after_baseline": post_restart["process_ids"][0]
        - baseline_last_process
        - 1,
        "observed_ambient_gap_after_baseline": (
            post_restart["ambient_ids"][0] - baseline_last_ambient - 1
            if post_restart["ambient_ids"]
            else None
        ),
        "delete_highest_committed_id": {
            "deleted_process_id": deleted_process_id,
            "deleted_ambient_id": deleted_ambient_id,
            "next_process_ids": after_highest_delete["process_ids"],
            "next_ambient_ids": after_highest_delete["ambient_ids"],
            "deleted_ids_not_reused": highest_id_not_reused,
            "final_ids_unique": final_unique,
            "final_rows_and_fields_ok": final_exact,
            "final_process_digest": _rows_digest(final_process),
            "final_ambient_digest": _rows_digest(final_ambient),
        },
        "artifacts": _artifact_sizes(database, temp_directory),
    }


def _run_crash_probe(directory: Path) -> dict[str, Any]:
    committed = _run_one_crash_case(directory / "committed", "committed")
    in_flight = _run_one_crash_case(directory / "in-flight", "in-flight")
    return {
        "pass": committed["pass"] and in_flight["pass"],
        "committed": committed,
        "in_flight": in_flight,
        "qualifier": (
            "The subprocess is terminated with SIGKILL after its marker. This tests "
            "process-crash reopen behavior only, not OS or power-loss durability."
        ),
    }


def _run_retention_probe(directory: Path, rows: int) -> dict[str, Any]:
    database = directory / "archive.duckdb"
    temp_directory = directory / "duckdb-temp"
    connection = _connect(database, temp_directory)
    retention_rows = max(5_000, min(rows, 25_000))
    _create_schema(
        connection,
        retention_rows + 1,
        max(1, math.ceil(retention_rows / 10)) + 1,
    )
    seeded = _seed_history(connection, retention_rows)
    connection.execute("CHECKPOINT")
    before_counts = _snapshot(connection)
    before_process, before_ambient = _all_rows(connection)
    before_sizes = _artifact_sizes(database, temp_directory)
    cutoff = _timestamp(-(retention_rows // PROCESS_ROWS_PER_MINUTE) // 2)
    expected_process = [row for row in before_process if row[-1] >= cutoff]
    expected_ambient = [row for row in before_ambient if row[-1] >= cutoff]
    eligible_process_ids = [int(row[0]) for row in before_process if row[-1] < cutoff]
    eligible_ambient_ids = [int(row[0]) for row in before_ambient if row[-1] < cutoff]

    connection.execute("BEGIN TRANSACTION")
    connection.execute("DELETE FROM process_stats WHERE timestamp < ?", [cutoff])
    connection.execute("DELETE FROM ambient_archive WHERE timestamp < ?", [cutoff])
    connection.execute("COMMIT")
    after_delete_counts = _snapshot(connection)
    after_delete_process, after_delete_ambient = _all_rows(connection)
    exact_delete = (
        after_delete_process == expected_process
        and after_delete_ambient == expected_ambient
    )
    after_delete_sizes = _artifact_sizes(database, temp_directory)
    connection.execute("CHECKPOINT")
    after_checkpoint_sizes = _artifact_sizes(database, temp_directory)
    connection.close()
    reopened = _connect(database, temp_directory)
    reopened_counts = _snapshot(reopened)
    reopened_process, reopened_ambient = _all_rows(reopened)
    exact_reopen = (
        reopened_process == expected_process and reopened_ambient == expected_ambient
    )
    reopened.close()

    logical_removed = (
        before_counts["process_rows"] - after_delete_counts["process_rows"]
        + before_counts["ambient_rows"]
        - after_delete_counts["ambient_rows"]
    )
    physical_before = before_sizes["database_bytes"] + before_sizes["wal_bytes"]
    physical_after = (
        after_checkpoint_sizes["database_bytes"] + after_checkpoint_sizes["wal_bytes"]
    )
    return {
        "pass": (
            logical_removed > 0
            and logical_removed
            == len(eligible_process_ids) + len(eligible_ambient_ids)
            and exact_delete
            and exact_reopen
            and reopened_counts == after_delete_counts
        ),
        "seeded": seeded,
        "cutoff_timestamp": cutoff,
        "before_counts": before_counts,
        "after_delete_counts": after_delete_counts,
        "reopened_counts": reopened_counts,
        "logical_rows_removed": logical_removed,
        "eligible_process_ids": {
            "count": len(eligible_process_ids),
            "sha256": _rows_digest([(identifier,) for identifier in eligible_process_ids]),
        },
        "eligible_ambient_ids": {
            "count": len(eligible_ambient_ids),
            "sha256": _rows_digest([(identifier,) for identifier in eligible_ambient_ids]),
        },
        "exact_eligible_rows_removed": exact_delete,
        "exact_unexpired_rows_and_fields_after_reopen": exact_reopen,
        "unexpired_process_digest": _rows_digest(expected_process),
        "unexpired_ambient_digest": _rows_digest(expected_ambient),
        "sizes_before_delete": before_sizes,
        "sizes_after_delete_before_checkpoint": after_delete_sizes,
        "sizes_after_checkpoint": after_checkpoint_sizes,
        "physical_file_bytes_delta_after_checkpoint": physical_after - physical_before,
        "physical_reclamation_observed": physical_after < physical_before,
        "qualifier": (
            "Logical retention and checkpoint persistence are asserted. File-length "
            "reclamation is only observed and is not required for this diagnostic."
        ),
    }


def _resource_usage(start_cpu: float, start_rss: int) -> dict[str, Any]:
    usage = resource.getrusage(resource.RUSAGE_SELF)
    return {
        "process_cpu_seconds": time.process_time() - start_cpu,
        "max_rss_raw": int(usage.ru_maxrss),
        "max_rss_start_raw": start_rss,
        "max_rss_unit": "bytes on macOS; KiB on Linux",
        "qualifier": (
            "CPU and maximum RSS cover this entire Python process, including setup, "
            "validation, and all three diagnostics. Killed worker RSS is excluded."
        ),
    }


def run_lifecycle(
    output: Path, rows: int = 100_000, repetitions: int = 30
) -> dict[str, Any]:
    """Run the bounded lifecycle qualification and write its JSON artifact."""
    if rows < PROCESS_ROWS_PER_MINUTE:
        raise ValueError(f"rows must be at least {PROCESS_ROWS_PER_MINUTE}")
    if repetitions < 1:
        raise ValueError("repetitions must be positive")
    versions = _assert_versions()
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    start_cpu = time.process_time()
    start_rss = int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss)
    wall_start = time.monotonic()

    root = Path(tempfile.mkdtemp(prefix="duckdb-lifecycle-", dir=output.parent))
    concurrent_dir = root / "concurrent"
    crash_dir = root / "crash"
    retention_dir = root / "retention"
    for directory in (concurrent_dir, crash_dir, retention_dir):
        directory.mkdir(parents=True)
    concurrent = _run_concurrent_lifecycle(concurrent_dir, rows, repetitions)
    crash = _run_crash_probe(crash_dir)
    retention = _run_retention_probe(retention_dir, rows)
    result = {
        "schema_version": 1,
        "pass": concurrent["pass"] and crash["pass"] and retention["pass"],
        "versions": versions,
        "parameters": {
            "historical_process_rows": rows,
            "minute_commits": repetitions,
            "duckdb_threads": 2,
            "duckdb_memory_limit": "128MB",
        },
        "artifact_root": str(root),
        "production_shape": {
            "minute_timestamp": "one exact RFC3339 text value for both families",
            "process_rows_per_minute": PROCESS_ROWS_PER_MINUTE,
            "process_identity": ["pid", "process_name"],
            "ambient_sources": list(AMBIENT_SOURCES),
            "missing_ambient": "absent rows",
            "experimental_integer_columns": "signed 64-bit BIGINT",
            "actual_producer_integer_widths": (
                "pid, memory_usage, and execution_sec are currently narrower Rust "
                "types; full numeric value preservation is outside this probe."
            ),
            "duckdb_transaction": "Process and Ambient rows committed together",
            "current_sqlite_difference": (
                "The current archive writer shares one tick timestamp but commits "
                "Process and Ambient through separate SQLite transactions."
            ),
        },
        "concurrent_lifecycle": concurrent,
        "process_crash": crash,
        "retention_checkpoint": retention,
        "resource_usage": _resource_usage(start_cpu, start_rss),
        "wall_time_ms": (time.monotonic() - wall_start) * 1_000,
        "limits": [
            "Synthetic rows and one-row-at-a-time inserts approximate the current writer shape; they do not prove production minute latency.",
            "The native schema is experimental and has no migration, generation publication, or application recovery integration.",
            "The pinned-reader result proves one DuckDB transaction snapshot in this run.",
            "SIGKILL tests process-crash behavior, not filesystem, OS, or power-loss durability.",
            "Retention reports logical deletion separately from observed physical file length.",
            "Sequence allocation requires uniqueness and non-reuse of a deleted highest committed ID; gap-free IDs are not required or claimed.",
            "Query cancellation and cross-platform behavior are untested.",
        ],
    }
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def main() -> int:
    # Parse the private subprocess form directly to keep the public CLI small.
    if "--crash-worker" in sys.argv:
        index = sys.argv.index("--crash-worker")
        values = sys.argv[index + 1 :]
        if len(values) != 4:
            raise SystemExit("--crash-worker requires DATABASE TEMP_DIRECTORY MODE MARKER")
        _crash_worker(Path(values[0]), Path(values[1]), values[2], Path(values[3]))
        return 0
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--rows", type=int, default=100_000)
    parser.add_argument("--repetitions", type=int, default=30)
    args = parser.parse_args()
    result = run_lifecycle(args.output, args.rows, args.repetitions)
    print(json.dumps({"pass": result["pass"], "output": str(args.output.resolve())}))
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
