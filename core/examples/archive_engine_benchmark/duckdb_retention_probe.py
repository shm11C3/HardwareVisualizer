#!/usr/bin/env python3
"""Measure retained-capacity behavior for synthetic Process and Ambient rows.

This standalone experiment never opens an application database. It compares
bounded DuckDB and SQLite lifecycle runs; it does not select a production
engine or establish a migration/recovery contract.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import resource
import tempfile
import threading
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Callable

import duckdb
import pysqlite3 as sqlite3


EXPECTED_DUCKDB_VERSION = "1.5.5"
EXPECTED_SQLITE_VERSION = "3.46.0"
PROCESS_ROWS_PER_MINUTE = 15
AMBIENT_SOURCES = ("cpu-package", "gpu-edge")
BASE_TIME = datetime(2026, 1, 1, tzinfo=timezone.utc)
PROCESS_COLUMNS = (
    "id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp"
)
AMBIENT_COLUMNS = "id, source, temperature, humidity, timestamp"


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


def _timestamp(minute: int) -> str:
    value = BASE_TIME + timedelta(minutes=minute, milliseconds=minute % 997)
    return value.isoformat(timespec="milliseconds").replace("+00:00", "Z")


def _rows_for_minutes(
    first_minute: int,
    count: int,
    first_process_id: int,
    first_ambient_id: int,
) -> tuple[list[tuple[Any, ...]], list[tuple[Any, ...]]]:
    process: list[tuple[Any, ...]] = []
    ambient: list[tuple[Any, ...]] = []
    process_id = first_process_id
    ambient_id = first_ambient_id
    for minute in range(first_minute, first_minute + count):
        timestamp = _timestamp(minute)
        for rank in range(PROCESS_ROWS_PER_MINUTE):
            process.append(
                (
                    process_id,
                    1_000 + rank,
                    f"synthetic-worker-{rank:02d}",
                    float((minute * 7 + rank * 11) % 100) + 0.25,
                    32_000_000 + rank * 1_000_000 + minute * 1_024,
                    minute * 60 + rank,
                    timestamp,
                )
            )
            process_id += 1
        # An absent source is represented by no row, as in the producer.
        if minute % 7:
            for source_index, source in enumerate(AMBIENT_SOURCES):
                ambient.append(
                    (
                        ambient_id,
                        source,
                        38.0 + (minute + source_index * 3) % 17 * 0.5,
                        None
                        if (minute + source_index) % 3
                        else 0.35 + minute % 5 * 0.01,
                        timestamp,
                    )
                )
                ambient_id += 1
    return process, ambient


def _quote_sql_string(value: Path) -> str:
    return str(value).replace("'", "''")


def _connect_duckdb(
    database: Path, temp_directory: Path
) -> duckdb.DuckDBPyConnection:
    temp_directory.mkdir(parents=True, exist_ok=True)
    connection = duckdb.connect(str(database))
    connection.execute("SET threads = 2")
    connection.execute("SET memory_limit = '128MB'")
    connection.execute(
        f"SET temp_directory = '{_quote_sql_string(temp_directory)}'"
    )
    return connection


def _connect_sqlite(database: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(database, isolation_level=None, timeout=5)
    connection.execute("PRAGMA journal_mode=WAL")
    connection.execute("PRAGMA synchronous=NORMAL")
    connection.execute("PRAGMA busy_timeout=5000")
    return connection


def _create_schema(connection: Any, engine: str) -> None:
    if engine == "duckdb":
        id_definition = "BIGINT PRIMARY KEY"
        integer_definition = "BIGINT"
        text_definition = "VARCHAR"
        real_definition = "DOUBLE"
        timestamp_definition = "VARCHAR"
    else:
        id_definition = "INTEGER PRIMARY KEY AUTOINCREMENT"
        integer_definition = "INTEGER"
        text_definition = "TEXT"
        real_definition = "REAL"
        timestamp_definition = "DATETIME"
    connection.execute(
        f"""
        CREATE TABLE process_stats (
            id {id_definition},
            pid {integer_definition} NOT NULL,
            process_name {text_definition} NOT NULL,
            cpu_usage {real_definition} NOT NULL,
            memory_usage {integer_definition} NOT NULL,
            execution_sec {integer_definition} NOT NULL,
            timestamp {timestamp_definition} NOT NULL
        )
        """
    )
    connection.execute(
        f"""
        CREATE TABLE ambient_archive (
            id {id_definition},
            source {text_definition} NOT NULL,
            temperature {real_definition} NOT NULL,
            humidity {real_definition},
            timestamp {timestamp_definition} NOT NULL
        )
        """
    )
    connection.execute(
        "CREATE INDEX idx_process_stats_timestamp ON process_stats(timestamp)"
    )
    connection.execute(
        "CREATE INDEX idx_ambient_archive_timestamp ON ambient_archive(timestamp)"
    )


def _insert_rows(
    connection: Any,
    engine: str,
    process: list[tuple[Any, ...]],
    ambient: list[tuple[Any, ...]],
) -> None:
    begin = "BEGIN TRANSACTION" if engine == "duckdb" else "BEGIN IMMEDIATE"
    connection.execute(begin)
    try:
        for offset in range(0, len(process), 5_000):
            connection.executemany(
                "INSERT INTO process_stats VALUES (?, ?, ?, ?, ?, ?, ?)",
                process[offset : offset + 5_000],
            )
        for offset in range(0, len(ambient), 5_000):
            connection.executemany(
                "INSERT INTO ambient_archive VALUES (?, ?, ?, ?, ?)",
                ambient[offset : offset + 5_000],
            )
        connection.execute("COMMIT")
    except BaseException:
        connection.execute("ROLLBACK")
        raise


def _delete_before(connection: Any, engine: str, cutoff: str) -> None:
    begin = "BEGIN TRANSACTION" if engine == "duckdb" else "BEGIN IMMEDIATE"
    connection.execute(begin)
    try:
        connection.execute(
            "DELETE FROM process_stats WHERE timestamp < ?", [cutoff]
        )
        connection.execute(
            "DELETE FROM ambient_archive WHERE timestamp < ?", [cutoff]
        )
        connection.execute("COMMIT")
    except BaseException:
        connection.execute("ROLLBACK")
        raise


def _path_metrics(path: Path) -> dict[str, int]:
    if not path.exists():
        return {"logical_bytes": 0, "allocated_bytes": 0}
    stat = path.stat()
    return {
        "logical_bytes": stat.st_size,
        "allocated_bytes": stat.st_blocks * 512,
    }


def _directory_allocated_bytes(directory: Path) -> int:
    allocated = 0
    try:
        paths = list(directory.rglob("*"))
    except FileNotFoundError:
        return 0
    for path in paths:
        try:
            if path.is_file():
                allocated += path.stat().st_blocks * 512
        except FileNotFoundError:
            continue
    return allocated


def _directory_logical_bytes(directory: Path) -> int:
    logical = 0
    try:
        paths = list(directory.rglob("*"))
    except FileNotFoundError:
        return 0
    for path in paths:
        try:
            if path.is_file():
                logical += path.stat().st_size
        except FileNotFoundError:
            continue
    return logical


def _sqlite_internal(connection: sqlite3.Connection) -> dict[str, int]:
    page_size = int(connection.execute("PRAGMA page_size").fetchone()[0])
    page_count = int(connection.execute("PRAGMA page_count").fetchone()[0])
    free_pages = int(connection.execute("PRAGMA freelist_count").fetchone()[0])
    return {
        "page_size": page_size,
        "total_pages": page_count,
        "used_pages": page_count - free_pages,
        "free_pages": free_pages,
        "used_bytes": (page_count - free_pages) * page_size,
        "free_bytes": free_pages * page_size,
    }


def _duckdb_internal(connection: duckdb.DuckDBPyConnection) -> dict[str, Any]:
    cursor = connection.execute("PRAGMA database_size")
    columns = [description[0] for description in cursor.description]
    return dict(zip(columns, cursor.fetchone(), strict=True))


def _engine_metrics(
    connection: Any,
    engine: str,
    database: Path,
    temp_directory: Path,
    step: str,
) -> dict[str, Any]:
    process_rows = int(
        connection.execute("SELECT count(*) FROM process_stats").fetchone()[0]
    )
    ambient_rows = int(
        connection.execute("SELECT count(*) FROM ambient_archive").fetchone()[0]
    )
    if engine == "duckdb":
        wal = Path(f"{database}.wal")
        shared_memory = None
        internal = _duckdb_internal(connection)
    else:
        wal = Path(f"{database}-wal")
        shared_memory = Path(f"{database}-shm")
        internal = _sqlite_internal(connection)
    database_metrics = _path_metrics(database)
    wal_metrics = _path_metrics(wal)
    shared_memory_metrics = (
        _path_metrics(shared_memory)
        if shared_memory is not None
        else {"logical_bytes": 0, "allocated_bytes": 0}
    )
    spill_allocated = (
        _directory_allocated_bytes(temp_directory)
        if temp_directory.exists()
        else 0
    )
    spill_logical = (
        _directory_logical_bytes(temp_directory)
        if temp_directory.exists()
        else 0
    )
    return {
        "step": step,
        "process_rows": process_rows,
        "ambient_rows": ambient_rows,
        "database": database_metrics,
        "wal": wal_metrics,
        "shared_memory": shared_memory_metrics,
        "spill_logical_bytes": spill_logical,
        "spill_allocated_bytes": spill_allocated,
        "measured_logical_bytes": (
            database_metrics["logical_bytes"]
            + wal_metrics["logical_bytes"]
            + shared_memory_metrics["logical_bytes"]
            + spill_logical
        ),
        "measured_allocated_bytes": (
            database_metrics["allocated_bytes"]
            + wal_metrics["allocated_bytes"]
            + shared_memory_metrics["allocated_bytes"]
            + spill_allocated
        ),
        "measured_files": (
            ["database", "wal", "spill"]
            if engine == "duckdb"
            else ["database", "wal", "shared_memory", "spill"]
        ),
        "internal": internal,
    }


def _normalize_row(row: tuple[Any, ...]) -> tuple[Any, ...]:
    return tuple(row)


def _validate_table(
    connection: Any,
    table: str,
    columns: str,
    expected: list[tuple[Any, ...]],
) -> dict[str, Any]:
    cursor = connection.execute(f"SELECT {columns} FROM {table} ORDER BY id")
    expected_digest = hashlib.sha256()
    actual_digest = hashlib.sha256()
    mismatches: list[str] = []
    checked = 0
    while actual_batch := cursor.fetchmany(5_000):
        expected_batch = expected[checked : checked + len(actual_batch)]
        for offset, actual in enumerate(actual_batch):
            actual = _normalize_row(actual)
            expected_row = expected_batch[offset] if offset < len(expected_batch) else None
            actual_digest.update(
                json.dumps(actual, ensure_ascii=False, separators=(",", ":")).encode()
            )
            actual_digest.update(b"\n")
            if expected_row is None or actual != expected_row:
                if len(mismatches) < 10:
                    mismatches.append(
                        f"row {checked + offset}: {actual!r} != {expected_row!r}"
                    )
        checked += len(actual_batch)
    for expected_row in expected:
        expected_digest.update(
            json.dumps(expected_row, ensure_ascii=False, separators=(",", ":")).encode()
        )
        expected_digest.update(b"\n")
    if checked != len(expected):
        mismatches.append(f"checked {checked} rows, expected {len(expected)}")
    return {
        "pass": not mismatches
        and actual_digest.hexdigest() == expected_digest.hexdigest(),
        "checked_rows": checked,
        "expected_rows": len(expected),
        "actual_sha256": actual_digest.hexdigest(),
        "expected_sha256": expected_digest.hexdigest(),
        "mismatches": mismatches,
    }


def _validate_all(
    connection: Any,
    expected_process: list[tuple[Any, ...]],
    expected_ambient: list[tuple[Any, ...]],
) -> dict[str, Any]:
    process = _validate_table(
        connection, "process_stats", PROCESS_COLUMNS, expected_process
    )
    ambient = _validate_table(
        connection, "ambient_archive", AMBIENT_COLUMNS, expected_ambient
    )
    return {"pass": process["pass"] and ambient["pass"], "process": process, "ambient": ambient}


def _index_report(connection: Any, engine: str) -> dict[str, Any]:
    if engine == "duckdb":
        names = [
            row[0]
            for row in connection.execute(
                """
                SELECT index_name
                FROM duckdb_indexes()
                WHERE table_name IN ('process_stats', 'ambient_archive')
                ORDER BY index_name
                """
            ).fetchall()
        ]
    else:
        names = [
            row[0]
            for row in connection.execute(
                """
                SELECT name
                FROM sqlite_master
                WHERE type = 'index'
                  AND tbl_name IN ('process_stats', 'ambient_archive')
                ORDER BY name
                """
            ).fetchall()
            if not str(row[0]).startswith("sqlite_autoindex")
        ]
    expected = ["idx_ambient_archive_timestamp", "idx_process_stats_timestamp"]
    return {"pass": names == expected, "names": names, "expected": expected}


def _checkpoint(connection: Any, engine: str) -> dict[str, Any]:
    started = time.monotonic()
    if engine == "duckdb":
        connection.execute("CHECKPOINT")
        detail = None
    else:
        row = connection.execute("PRAGMA wal_checkpoint(TRUNCATE)").fetchone()
        detail = {"busy": int(row[0]), "log_frames": int(row[1]), "checkpointed": int(row[2])}
    return {"duration_ms": (time.monotonic() - started) * 1_000, "detail": detail}


def _growth_classification(allocated: list[int], block_bytes: int) -> dict[str, Any]:
    deltas = [
        allocated[index] - allocated[index - 1]
        for index in range(1, len(allocated))
    ]
    tolerance = max(block_bytes * 2, 8_192)
    if any(delta < -tolerance for delta in deltas):
        observation = "shrink_observed"
    elif allocated and max(allocated) - min(allocated) <= tolerance:
        observation = "stable_or_reused"
    elif deltas and all(delta > tolerance for delta in deltas):
        observation = "bounded_monotonic_growth_observed"
    else:
        observation = "step_growth_or_reuse_observed"
    return {
        "observation": observation,
        "checkpoint_allocated_bytes": allocated,
        "cycle_deltas_bytes": deltas,
        "qualifier": (
            "A bounded cycle count can show shrink, reuse, or monotonic growth in "
            "this run; it cannot prove asymptotically unbounded growth."
        ),
    }


def _sample_copy_peak(
    directories: list[Path], operation: Callable[[], None]
) -> tuple[float, int]:
    stop = threading.Event()
    peak = [sum(_directory_allocated_bytes(path) for path in directories)]

    def sample() -> None:
        while not stop.wait(0.002):
            peak[0] = max(
                peak[0],
                sum(_directory_allocated_bytes(path) for path in directories),
            )

    thread = threading.Thread(target=sample, name="copy-disk-sampler")
    thread.start()
    started = time.monotonic()
    try:
        operation()
    finally:
        duration_ms = (time.monotonic() - started) * 1_000
        stop.set()
        thread.join()
        peak[0] = max(
            peak[0],
            sum(_directory_allocated_bytes(path) for path in directories),
        )
    return duration_ms, peak[0]


def _compact_copy(
    connection: Any,
    engine: str,
    source_database: Path,
    compact_directory: Path,
) -> tuple[Path, dict[str, Any]]:
    compact_directory.mkdir(parents=True)
    target = compact_directory / (
        "compacted.duckdb" if engine == "duckdb" else "compacted.sqlite3"
    )

    if engine == "duckdb":
        source_name = str(connection.execute("SELECT current_database()").fetchone()[0])

        def copy() -> None:
            connection.execute(
                f"ATTACH '{_quote_sql_string(target)}' AS compacted"
            )
            try:
                connection.execute(
                    f'COPY FROM DATABASE "{source_name}" TO compacted'
                )
            finally:
                connection.execute("DETACH compacted")

    else:

        def copy() -> None:
            connection.execute(f"VACUUM INTO '{_quote_sql_string(target)}'")

    before_source = _path_metrics(source_database)
    duration_ms, peak = _sample_copy_peak(
        [source_database.parent], copy
    )
    after_source = _path_metrics(source_database)
    return target, {
        "method": (
            "COPY FROM DATABASE source TO compacted"
            if engine == "duckdb"
            else "VACUUM INTO compacted"
        ),
        "duration_ms": duration_ms,
        "observed_peak_combined_allocated_bytes_lower_bound": peak,
        "source_database_before": before_source,
        "source_database_after": after_source,
        "source_kept": source_database.exists(),
        "target": _path_metrics(target),
        "sampler_interval_ms": 2,
        "peak_qualifier": (
            "This is a sampled lower bound at 2 ms intervals across the source "
            "and compact-output directories; it is not an exact peak."
        ),
    }


def _run_engine(
    engine: str,
    directory: Path,
    initial_minutes: int,
    retained_minutes: int,
    append_minutes: int,
    cycles: int,
) -> dict[str, Any]:
    directory.mkdir(parents=True)
    database = directory / (
        "archive.duckdb" if engine == "duckdb" else "archive.sqlite3"
    )
    temp_directory = directory / "spill"
    connection = (
        _connect_duckdb(database, temp_directory)
        if engine == "duckdb"
        else _connect_sqlite(database)
    )
    _create_schema(connection, engine)
    indexes = _index_report(connection, engine)
    expected_process, expected_ambient = _rows_for_minutes(
        0, initial_minutes, 1, 1
    )
    next_process_id = len(expected_process) + 1
    next_ambient_id = len(expected_ambient) + 1
    _insert_rows(connection, engine, expected_process, expected_ambient)
    timeline = [
        _engine_metrics(
            connection, engine, database, temp_directory, "initial_append"
        )
    ]
    initial_checkpoint = _checkpoint(connection, engine)
    timeline.append(
        _engine_metrics(
            connection, engine, database, temp_directory, "initial_checkpoint"
        )
    )
    connection.close()
    connection = (
        _connect_duckdb(database, temp_directory)
        if engine == "duckdb"
        else _connect_sqlite(database)
    )
    initial_full_validation = _validate_all(
        connection, expected_process, expected_ambient
    )
    initial_cutoff = _timestamp(initial_minutes - retained_minutes)
    _delete_before(connection, engine, initial_cutoff)
    expected_process = [
        row for row in expected_process if str(row[-1]) >= initial_cutoff
    ]
    expected_ambient = [
        row for row in expected_ambient if str(row[-1]) >= initial_cutoff
    ]
    after_initial_delete = _engine_metrics(
        connection,
        engine,
        database,
        temp_directory,
        "initial_retention_delete",
    )
    initial_purge_checkpoint = _checkpoint(connection, engine)
    after_initial_checkpoint = _engine_metrics(
        connection,
        engine,
        database,
        temp_directory,
        "initial_purge_checkpoint",
    )
    connection.close()
    connection = (
        _connect_duckdb(database, temp_directory)
        if engine == "duckdb"
        else _connect_sqlite(database)
    )
    initial_purge_validation = _validate_all(
        connection, expected_process, expected_ambient
    )
    after_initial_reopen = _engine_metrics(
        connection,
        engine,
        database,
        temp_directory,
        "initial_purge_reopen",
    )
    timeline.extend(
        [after_initial_delete, after_initial_checkpoint, after_initial_reopen]
    )
    next_minute = initial_minutes
    cycle_reports = []
    checkpoint_allocated = [after_initial_checkpoint["measured_allocated_bytes"]]

    for cycle in range(1, cycles + 1):
        appended_process, appended_ambient = _rows_for_minutes(
            next_minute,
            append_minutes,
            next_process_id,
            next_ambient_id,
        )
        _insert_rows(connection, engine, appended_process, appended_ambient)
        expected_process.extend(appended_process)
        expected_ambient.extend(appended_ambient)
        next_minute += append_minutes
        next_process_id += len(appended_process)
        next_ambient_id += len(appended_ambient)
        after_append = _engine_metrics(
            connection,
            engine,
            database,
            temp_directory,
            f"cycle_{cycle}_append",
        )

        first_retained_minute = next_minute - retained_minutes
        cutoff = _timestamp(first_retained_minute)
        _delete_before(connection, engine, cutoff)
        expected_process = [
            row for row in expected_process if str(row[-1]) >= cutoff
        ]
        expected_ambient = [
            row for row in expected_ambient if str(row[-1]) >= cutoff
        ]
        after_delete = _engine_metrics(
            connection,
            engine,
            database,
            temp_directory,
            f"cycle_{cycle}_delete",
        )
        checkpoint_result = _checkpoint(connection, engine)
        after_checkpoint = _engine_metrics(
            connection,
            engine,
            database,
            temp_directory,
            f"cycle_{cycle}_checkpoint",
        )
        checkpoint_allocated.append(after_checkpoint["measured_allocated_bytes"])
        connection.close()
        connection = (
            _connect_duckdb(database, temp_directory)
            if engine == "duckdb"
            else _connect_sqlite(database)
        )
        validation = _validate_all(
            connection, expected_process, expected_ambient
        )
        after_reopen = _engine_metrics(
            connection,
            engine,
            database,
            temp_directory,
            f"cycle_{cycle}_reopen",
        )
        timeline.extend(
            [after_append, after_delete, after_checkpoint, after_reopen]
        )
        cycle_reports.append(
            {
                "cycle": cycle,
                "cutoff_timestamp": cutoff,
                "checkpoint": checkpoint_result,
                "validation_after_reopen": validation,
            }
        )

    compact_directory = directory / "compact-copy"
    target, copy_report = _compact_copy(
        connection,
        engine,
        database,
        compact_directory,
    )
    source_validation = _validate_all(
        connection, expected_process, expected_ambient
    )
    connection.close()
    compact_temp = compact_directory / "spill"
    compact_connection = (
        _connect_duckdb(target, compact_temp)
        if engine == "duckdb"
        else _connect_sqlite(target)
    )
    compact_validation = _validate_all(
        compact_connection, expected_process, expected_ambient
    )
    compact_indexes = _index_report(compact_connection, engine)
    compact_metrics = _engine_metrics(
        compact_connection,
        engine,
        target,
        compact_temp,
        "compacted_reopen",
    )
    compact_connection.close()
    validations = [initial_full_validation, initial_purge_validation] + [
        report["validation_after_reopen"] for report in cycle_reports
    ]
    block_bytes = (
        int(timeline[-1]["internal"].get("block_size", 262_144))
        if engine == "duckdb"
        else int(timeline[-1]["internal"]["page_size"])
    )
    return {
        "pass": (
            indexes["pass"]
            and all(validation["pass"] for validation in validations)
            and source_validation["pass"]
            and compact_validation["pass"]
            and compact_indexes["pass"]
        ),
        "engine": engine,
        "database": str(database),
        "temp_directory": str(temp_directory),
        "schema": {
            "primary_keys": "explicit integer id primary keys",
            "indexes": indexes,
            "omitted": (
                "No production indexes are omitted for Process or Ambient: both "
                "current timestamp indexes are present. DuckDB uses BIGINT IDs and "
                "numeric columns; SQLite uses the shipped INTEGER/REAL/TEXT/DATETIME "
                "affinities."
            ),
            "migration_source": (
                "src-tauri/src/infrastructure/database/migration.rs: Process table "
                "definition, Process timestamp index v8, Ambient table and index v15"
            ),
            "process_identity": (
                "The opaque (pid, process_name) pair is preserved as stored; this "
                "probe does not reinterpret process lifetime or name semantics."
            ),
            "transaction_shape": (
                "Each synthetic append/delete step covers Process and Ambient in "
                "one transaction for controlled engine comparison."
            ),
        },
        "initial_full_validation": initial_full_validation,
        "initial_purge": {
            "cutoff_timestamp": initial_cutoff,
            "checkpoint": initial_purge_checkpoint,
            "validation_after_reopen": initial_purge_validation,
            "retained_fraction": retained_minutes / initial_minutes,
        },
        "initial_checkpoint": initial_checkpoint,
        "cycles": cycle_reports,
        "timeline": timeline,
        "growth": _growth_classification(checkpoint_allocated, block_bytes),
        "copy_compaction": {
            **copy_report,
            "reference_kind": "fresh file containing the same survivor rows",
            "source_validation_after_copy": source_validation,
            "compacted_validation_after_reopen": compact_validation,
            "compacted_indexes": compact_indexes,
            "compacted_metrics": compact_metrics,
        },
    }


def run_retention(
    output: Path,
    initial_minutes: int = 42_000,
    retained_minutes: int = 8_400,
    append_minutes: int = 1_050,
    cycles: int = 8,
) -> dict[str, Any]:
    """Run both engines serially and retain databases beside the JSON artifact."""
    if retained_minutes < 2 or initial_minutes < retained_minutes:
        raise ValueError("initial_minutes must be at least retained_minutes >= 2")
    if not 0 < append_minutes <= retained_minutes:
        raise ValueError("append_minutes must be in 1..retained_minutes")
    if cycles < 3:
        raise ValueError("cycles must be at least 3 to observe reuse behavior")
    versions = _assert_versions()
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    artifact_root = Path(
        tempfile.mkdtemp(prefix="duckdb-retention-", dir=output.parent)
    )
    started = time.monotonic()
    duckdb_result = _run_engine(
        "duckdb",
        artifact_root / "duckdb",
        initial_minutes,
        retained_minutes,
        append_minutes,
        cycles,
    )
    sqlite_result = _run_engine(
        "sqlite",
        artifact_root / "sqlite",
        initial_minutes,
        retained_minutes,
        append_minutes,
        cycles,
    )
    usage = resource.getrusage(resource.RUSAGE_SELF)
    result = {
        "schema_version": 1,
        "pass": duckdb_result["pass"] and sqlite_result["pass"],
        "versions": versions,
        "parameters": {
            "retained_minutes": retained_minutes,
            "initial_minutes": initial_minutes,
            "append_minutes_per_cycle": append_minutes,
            "cycles": cycles,
            "steady_process_rows": retained_minutes * PROCESS_ROWS_PER_MINUTE,
            "expected_ambient_rows_approximate": retained_minutes
            * len(AMBIENT_SOURCES)
            * 6
            // 7,
        },
        "proposed_final_matrix": {
            "initial_minutes": 42_000,
            "retained_minutes": 8_400,
            "initial_retained_fraction": 0.20,
            "append_minutes_per_cycle": 1_050,
            "cycles": 8,
            "initial_process_rows": 630_000,
            "steady_process_rows": 126_000,
            "total_process_rows_written_per_engine": (
                42_000 + 1_050 * 8
            )
            * PROCESS_ROWS_PER_MINUTE,
            "reason": (
                "The initial purge retains 20% of 630,000 Process rows. Retained "
                "and appended minute counts are multiples of seven, so the missing-"
                "Ambient pattern also keeps a stable row count across eight cycles."
            ),
        },
        "artifact_root": str(artifact_root),
        "duckdb": duckdb_result,
        "sqlite": sqlite_result,
        "wall_time_ms": (time.monotonic() - started) * 1_000,
        "resource_usage": {
            "process_cpu_seconds": usage.ru_utime + usage.ru_stime,
            "max_rss_raw": int(usage.ru_maxrss),
            "max_rss_unit": "bytes on macOS; KiB on Linux",
            "qualifier": (
                "CPU and RSS are process-wide and include fixture creation, both "
                "engines, integrity scans, checkpoints, and copy compaction."
            ),
        },
        "proof_method": (
            "Every reopen streams all surviving IDs and fields in ID order, compares "
            "them with deterministic expected tuples, and requires matching SHA-256 "
            "digests. A count-only match cannot pass."
        ),
        "limits": [
            "All inputs and databases are synthetic and isolated from application data.",
            "Bounded cycles describe observed capacity behavior; they do not prove an asymptotic upper bound.",
            "Allocated bytes use stat blocks; filesystem compression, sparse allocation, and delayed writes can affect them.",
            "DuckDB copy uses COPY FROM DATABASE into a new file while retaining the source; SQLite control uses VACUUM INTO after WAL checkpoints.",
            "The copy disk peak is a sampled lower bound, not an exact peak.",
            "This does not test power loss, migration publication, query concurrency, cancellation, or production SQLx bindings.",
        ],
    }
    output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--initial-minutes", type=int, default=42_000)
    parser.add_argument("--retained-minutes", type=int, default=8_400)
    parser.add_argument("--append-minutes", type=int, default=1_050)
    parser.add_argument("--cycles", type=int, default=8)
    args = parser.parse_args()
    result = run_retention(
        args.output,
        initial_minutes=args.initial_minutes,
        retained_minutes=args.retained_minutes,
        append_minutes=args.append_minutes,
        cycles=args.cycles,
    )
    print(
        json.dumps(
            {"pass": result["pass"], "output": str(args.output.resolve())}
        )
    )
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
