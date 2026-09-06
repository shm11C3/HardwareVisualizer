#!/usr/bin/env python3
"""Bounded SQLite, DuckDB, and Parquet archive query experiment for #2052."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import resource
import struct
import sys
import time
from typing import Any, Iterable, Sequence

import duckdb
import pysqlite3 as sqlite3


EPOCH_MS_SQL = "(CAST(strftime('%s', timestamp) AS INTEGER) * 1000 + CAST(substr(strftime('%f', timestamp), 4, 3) AS INTEGER))"
BATCH_ROWS = 500
EXPORT_BATCH_ROWS = 10_000
PARQUET_ROW_GROUP_SIZE = 122_880
NULL_TOKEN = "__HARDVIZ_NULL_2052__"
FLOAT_TOLERANCE = "abs(actual-reference) <= max(1e-9, 1e-12*abs(reference))"
STRATEGIES = ("sqlite", "duckdb", "parquet")
REQUIRED_SQLITE_VERSION = "3.46.0"
PROCESS_COLUMNS = (
    "id",
    "pid",
    "process_name",
    "cpu_usage",
    "memory_usage",
    "execution_sec",
    "timestamp",
)
AMBIENT_COLUMNS = (
    "id",
    "source",
    "temperature",
    "humidity",
    "timestamp",
    "epoch_ms",
)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--range-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=7)
    parser.add_argument("--threads", type=int, default=2)
    parser.add_argument("--memory-limit", default="128MB")
    parser.add_argument("--mode", choices=("prepare", "query", "all"), default="all")
    parser.add_argument("--strategy", choices=STRATEGIES)
    args = parser.parse_args(argv)
    if args.repetitions <= 0 or args.threads <= 0:
        parser.error("--repetitions and --threads must be positive")
    args.source = args.source.resolve()
    args.range_report = args.range_report.resolve()
    args.output = args.output.resolve()
    if not args.source.is_file():
        parser.error("--source must be an existing SQLite file")
    if not args.range_report.is_file():
        parser.error("--range-report must be an existing report.json")
    if args.mode == "query" and args.strategy is None:
        parser.error("--strategy is required with --mode query")
    if args.mode != "query" and args.strategy is not None:
        parser.error("--strategy is only valid with --mode query")
    if args.mode in ("prepare", "all") and args.output.exists():
        parser.error("--output must not exist for prepare/all mode")
    if args.mode == "query" and not args.output.is_dir():
        parser.error("--output must be an existing prepared directory in query mode")
    return args


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    if sqlite3.sqlite_version != REQUIRED_SQLITE_VERSION:
        raise SystemExit(
            f"pysqlite3 SQLite {REQUIRED_SQLITE_VERSION} is required; "
            f"found {sqlite3.sqlite_version}"
        )
    if args.mode in ("prepare", "all"):
        args.output.mkdir(parents=True, exist_ok=False)
        prepared = prepare(args)
        write_json(args.output / "prepared.json", prepared)
        if args.mode == "prepare":
            print(json.dumps(prepared, indent=2))
            return 0
    else:
        prepared = load_prepared(args)

    if args.mode == "query":
        destination = args.output / f"query-{args.strategy}.json"
        if destination.exists():
            raise SystemExit(f"refusing existing query result {destination}")
        result = run_isolated_strategy(args, prepared, args.strategy)
        write_json(destination, result)
        print(json.dumps(result, indent=2))
        return 0

    result = run_combined(args, prepared)
    write_json(args.output / "report.json", result)
    print(json.dumps(result, indent=2))
    return 0


def prepare(args: argparse.Namespace) -> dict[str, Any]:
    source_sha256 = sha256_file(args.source)
    range_sha256 = sha256_file(args.range_report)
    range_report = json.loads(args.range_report.read_text())
    ranges = read_ranges(range_report)
    source_commit = range_report["environment"]["git_commit"]
    source_size = args.source.stat().st_size
    source_counts = source_row_counts(args.source)
    export_dir = args.output / "csv-export"
    parquet_dir = args.output / "parquet"
    temp_dir = args.output / "duckdb-temp"
    export_dir.mkdir()
    parquet_dir.mkdir()
    temp_dir.mkdir()

    export_started = phase_started()
    export_counts = export_source(args.source, export_dir)
    export_metrics = phase_finished(export_started)
    if export_counts != source_counts:
        raise RuntimeError(f"CSV export count mismatch: {export_counts} != {source_counts}")

    native_path = args.output / "archive.duckdb"
    build_metrics = build_native_and_parquet(
        native_path,
        export_dir,
        parquet_dir,
        temp_dir,
        args.threads,
        args.memory_limit,
    )

    verification = verify_round_trip(args.source, native_path, parquet_dir, args)
    if not verification["all_passed"]:
        raise RuntimeError("DuckDB/Parquet round-trip verification failed")

    source_after_sha256 = sha256_file(args.source)
    if source_after_sha256 != source_sha256 or args.source.stat().st_size != source_size:
        raise RuntimeError("read-only source changed during preparation")

    prepared = {
        "format": "hardviz-archive-engine-prepared-v1",
        "production_candidate_accepted": False,
        "scope": "synthetic query-engine comparison; no production wiring",
        "config": config_report(args),
        "versions": version_report(range_report),
        "source": {
            "path": str(args.source),
            "sha256": source_sha256,
            "size_bytes": source_size,
            "process_rows": source_counts["process_rows"],
            "ambient_rows": source_counts["ambient_rows"],
            "source_commit": source_commit,
            "range_report_path": str(args.range_report),
            "range_report_sha256": range_sha256,
        },
        "ranges": ranges,
        "sqlite_query_policy": sqlite_policy_report(args.source),
        "semantic_adapters": {
            "ambient_epoch_ms": (
                "Computed by the source SQLite EPOCH_MS_SQL expression during export and "
                "stored as BIGINT in DuckDB and Parquet; this does not prove native "
                "DuckDB timestamp parsing equivalence."
            )
        },
        "build": {
            "csv_export": export_metrics | export_counts,
            "duckdb_and_parquet": build_metrics,
            "parquet_compression": "ZSTD",
            "parquet_row_group_size": PARQUET_ROW_GROUP_SIZE,
        },
        "storage": storage_report(args.output),
        "round_trip": verification,
        "plans": query_plans(args.source, native_path, parquet_dir, ranges, args),
        "process_peak_rss": rss_report(),
        "limitations": limitations(),
    }
    return prepared


def load_prepared(args: argparse.Namespace) -> dict[str, Any]:
    manifest_path = args.output / "prepared.json"
    if not manifest_path.is_file():
        raise SystemExit(f"missing prepared manifest {manifest_path}")
    prepared = json.loads(manifest_path.read_text())
    if prepared.get("format") != "hardviz-archive-engine-prepared-v1":
        raise SystemExit("unsupported prepared manifest")
    if sha256_file(args.source) != prepared["source"]["sha256"]:
        raise SystemExit("--source does not match prepared source SHA-256")
    if sha256_file(args.range_report) != prepared["source"]["range_report_sha256"]:
        raise SystemExit("--range-report does not match prepared report SHA-256")
    expected = prepared["config"]
    for key, actual in (
        ("threads", args.threads),
        ("memory_limit", args.memory_limit),
        ("repetitions", args.repetitions),
    ):
        if expected[key] != actual:
            raise SystemExit(f"{key} differs from prepared configuration")
    return prepared


def read_ranges(report: dict[str, Any]) -> list[dict[str, Any]]:
    ranges = report["query_experiment"]["ranges"]
    return [
        {
            "name": item["name"],
            "start": item["start"],
            "end": item["end"],
            "start_epoch_ms": sqlite_epoch_ms(item["start"]),
            "end_epoch_ms": sqlite_epoch_ms(item["end"]),
            "represented_minutes": item["represented_minutes"],
        }
        for item in ranges
    ]


def sqlite_epoch_ms(timestamp: str) -> int:
    connection = sqlite3.connect(":memory:")
    try:
        value = connection.execute(
            f"SELECT {EPOCH_MS_SQL} FROM (SELECT ? AS timestamp)", (timestamp,)
        ).fetchone()
        return int(value[0])
    finally:
        connection.close()


def source_row_counts(source: Path) -> dict[str, int]:
    connection = open_sqlite_read_only(source)
    try:
        return {
            "process_rows": connection.execute("SELECT COUNT(*) FROM PROCESS_STATS").fetchone()[0],
            "ambient_rows": connection.execute("SELECT COUNT(*) FROM AMBIENT_ARCHIVE").fetchone()[0],
        }
    finally:
        connection.close()


def export_source(source: Path, export_dir: Path) -> dict[str, int]:
    connection = open_sqlite_read_only(source)
    try:
        process_rows = export_query(
            connection,
            "SELECT id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp "
            "FROM PROCESS_STATS ORDER BY id",
            export_dir / "process.csv",
            PROCESS_COLUMNS,
        )
        ambient_rows = export_query(
            connection,
            "SELECT id, source, temperature, humidity, timestamp, "
            f"{EPOCH_MS_SQL} AS epoch_ms FROM AMBIENT_ARCHIVE ORDER BY id",
            export_dir / "ambient.csv",
            AMBIENT_COLUMNS,
        )
        return {"process_rows": process_rows, "ambient_rows": ambient_rows}
    finally:
        connection.close()


def export_query(
    connection: sqlite3.Connection,
    query: str,
    destination: Path,
    columns: Sequence[str],
) -> int:
    count = 0
    cursor = connection.execute(query)
    with destination.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(columns)
        while rows := cursor.fetchmany(EXPORT_BATCH_ROWS):
            for row in rows:
                writer.writerow([NULL_TOKEN if value is None else value for value in row])
            count += len(rows)
    return count


def build_native_and_parquet(
    native_path: Path,
    export_dir: Path,
    parquet_dir: Path,
    temp_dir: Path,
    threads: int,
    memory_limit: str,
) -> dict[str, Any]:
    total_started = phase_started()
    connection = open_duckdb(native_path, temp_dir, threads, memory_limit)
    try:
        native_started = phase_started()
        connection.execute(
            "CREATE TABLE process_stats (id BIGINT, pid BIGINT, process_name VARCHAR, "
            "cpu_usage DOUBLE, memory_usage BIGINT, execution_sec BIGINT, timestamp VARCHAR)"
        )
        connection.execute(
            "CREATE TABLE ambient_archive (id BIGINT, source VARCHAR, temperature DOUBLE, "
            "humidity DOUBLE, timestamp VARCHAR, epoch_ms BIGINT)"
        )
        connection.execute(
            "INSERT INTO process_stats SELECT * FROM read_csv(?, header=true, "
            "columns={'id':'BIGINT','pid':'BIGINT','process_name':'VARCHAR',"
            "'cpu_usage':'DOUBLE','memory_usage':'BIGINT','execution_sec':'BIGINT',"
            "'timestamp':'VARCHAR'}, nullstr=?)",
            [str(export_dir / "process.csv"), NULL_TOKEN],
        )
        connection.execute(
            "INSERT INTO ambient_archive SELECT * FROM read_csv(?, header=true, "
            "columns={'id':'BIGINT','source':'VARCHAR','temperature':'DOUBLE',"
            "'humidity':'DOUBLE','timestamp':'VARCHAR','epoch_ms':'BIGINT'}, nullstr=?)",
            [str(export_dir / "ambient.csv"), NULL_TOKEN],
        )
        native_metrics = phase_finished(native_started)
        parquet_started = phase_started()
        connection.execute(
            f"COPY (SELECT * FROM process_stats ORDER BY id) TO {sql_string(parquet_dir / 'process.parquet')} "
            f"(FORMAT PARQUET, COMPRESSION ZSTD, ROW_GROUP_SIZE {PARQUET_ROW_GROUP_SIZE})"
        )
        connection.execute(
            f"COPY (SELECT * FROM ambient_archive ORDER BY id) TO {sql_string(parquet_dir / 'ambient.parquet')} "
            f"(FORMAT PARQUET, COMPRESSION ZSTD, ROW_GROUP_SIZE {PARQUET_ROW_GROUP_SIZE})"
        )
        parquet_metrics = phase_finished(parquet_started)
        connection.execute("CHECKPOINT")
    finally:
        connection.close()
    return {
        "total": phase_finished(total_started),
        "native_import": native_metrics,
        "parquet_write": parquet_metrics,
    }


def verify_round_trip(
    source: Path,
    native_path: Path,
    parquet_dir: Path,
    args: argparse.Namespace,
) -> dict[str, Any]:
    families = {}
    for family in ("process", "ambient"):
        source_connection = open_sqlite_read_only(source)
        native_connection = open_duckdb(
            native_path, args.output / "duckdb-temp", args.threads, args.memory_limit, read_only=True
        )
        parquet_connection = open_duckdb(
            None, args.output / "duckdb-temp", args.threads, args.memory_limit
        )
        try:
            source_cursor = source_connection.execute(round_trip_sql("sqlite", family, parquet_dir))
            native_cursor = native_connection.execute(round_trip_sql("duckdb", family, parquet_dir))
            parquet_cursor = parquet_connection.execute(round_trip_sql("parquet", family, parquet_dir))
            comparison = compare_round_trip_streams(
                family, source_cursor, native_cursor, parquet_cursor
            )
            families[family] = comparison
        finally:
            source_connection.close()
            native_connection.close()
            parquet_connection.close()
    return {
        "all_passed": all(value["passed"] for value in families.values()),
        "comparison": "bounded 500-row streaming; integers/text/null exact and REAL bitwise",
        "storage_class_scope": "normal generated fixture types; adversarial SQLite storage classes are covered separately",
        "families": families,
    }


def compare_round_trip_streams(
    family: str,
    source_cursor: Any,
    native_cursor: Any,
    parquet_cursor: Any,
) -> dict[str, Any]:
    count = 0
    digests = [hashlib.sha256(), hashlib.sha256(), hashlib.sha256()]
    float_indexes = {3} if family == "process" else {2, 3}
    while True:
        batches = [cursor.fetchmany(BATCH_ROWS) for cursor in (source_cursor, native_cursor, parquet_cursor)]
        lengths = {len(batch) for batch in batches}
        if len(lengths) != 1:
            return {"passed": False, "rows": count, "error": "batch length mismatch"}
        if not batches[0]:
            break
        for rows in zip(*batches):
            reference = rows[0]
            for actual in rows[1:]:
                if not exact_row_equal(reference, actual, float_indexes):
                    return {
                        "passed": False,
                        "rows": count,
                        "error": f"{family} row mismatch at source id {reference[0]}",
                    }
            for digest, row in zip(digests, rows):
                digest_row(digest, row)
            count += 1
    digest_values = [digest.hexdigest() for digest in digests]
    return {
        "passed": len(set(digest_values)) == 1,
        "rows": count,
        "source_sha256": digest_values[0],
        "duckdb_sha256": digest_values[1],
        "parquet_sha256": digest_values[2],
    }


def run_combined(args: argparse.Namespace, prepared: dict[str, Any]) -> dict[str, Any]:
    repetitions = []
    for repetition in range(args.repetitions):
        order = STRATEGIES[repetition % 3 :] + STRATEGIES[: repetition % 3]
        strategy_results = {}
        for strategy in order:
            strategy_results[strategy] = time_strategy_once(args, prepared, strategy)
        assert_timed_counts(strategy_results)
        repetitions.append(
            {"repetition": repetition + 1, "execution_order": order, "strategies": strategy_results}
        )
    timed_process_peak_rss = rss_report()
    validation = validate_queries(args, prepared)
    if not validation["all_passed"]:
        raise RuntimeError("query differential validation failed")
    assert_validation_counts(repetitions, validation)
    return base_query_report(args, prepared) | {
        "mode": "combined_rotating",
        "repetitions": repetitions,
        "timed_process_peak_rss": timed_process_peak_rss,
        "validation": validation,
        "storage_after_queries": storage_report(args.output),
        "process_peak_rss": rss_report(),
    }


def run_isolated_strategy(
    args: argparse.Namespace, prepared: dict[str, Any], strategy: str
) -> dict[str, Any]:
    repetitions = [time_strategy_once(args, prepared, strategy) for _ in range(args.repetitions)]
    timed_process_peak_rss = rss_report()
    validation = validate_queries(args, prepared, strategies=(strategy,))
    if not validation["all_passed"]:
        raise RuntimeError(f"{strategy} query validation failed")
    assert_validation_counts(repetitions, validation)
    return base_query_report(args, prepared) | {
        "mode": "isolated_strategy",
        "strategy": strategy,
        "repetitions": repetitions,
        "timed_process_peak_rss": timed_process_peak_rss,
        "validation": validation,
        "storage_after_queries": storage_report(args.output),
        "process_peak_rss": rss_report(),
    }


def base_query_report(args: argparse.Namespace, prepared: dict[str, Any]) -> dict[str, Any]:
    return {
        "format": "hardviz-archive-engine-query-v1",
        "production_candidate_accepted": False,
        "config": config_report(args),
        "source": prepared["source"],
        "ranges": prepared["ranges"],
        "timing_contract": {
            "client_batch_rows": BATCH_ROWS,
            "connection_lifecycle": (
                "Each repetition opens a fresh connection before timing and closes it after "
                "all four queries. Connection open is excluded. Repetitions may use OS/cache-primed "
                "data but do not reuse an engine connection or prepared plan."
            ),
            "execute_ms": "execute call only",
            "first_page_ms": "query start through first fetchmany(500)",
            "complete_fetch_ms": "remaining result drain after the first page",
            "total_ms": "query start through complete drain",
            "validation": "separate untimed bounded streaming comparison",
        },
        "sqlite_query_policy": sqlite_policy_report(args.source),
        "memory_contract": (
            "DuckDB memory_limit constrains DuckDB-managed memory, not whole-process RSS; "
            "ru_maxrss is a process-lifetime high-water mark and not incremental query memory."
        ),
        "limitations": limitations(),
    }


def time_strategy_once(
    args: argparse.Namespace, prepared: dict[str, Any], strategy: str
) -> dict[str, Any]:
    connection = open_strategy(strategy, args)
    try:
        result = {"process": {}, "ambient": {}}
        for family in ("process", "ambient"):
            for range_item in prepared["ranges"]:
                result[family][range_item["name"]] = timed_drain(
                    connection,
                    query_sql(strategy, family, args.output / "parquet"),
                    query_parameters(family, range_item),
                )
        return result
    finally:
        connection.close()


def timed_drain(connection: Any, sql: str, parameters: Sequence[Any]) -> dict[str, Any]:
    wall_started = time.perf_counter_ns()
    cpu_started = time.process_time_ns()
    execute_started = time.perf_counter_ns()
    cursor = connection.execute(sql, parameters)
    execute_ms = elapsed_ms(execute_started)
    first_page = cursor.fetchmany(BATCH_ROWS)
    first_page_ms = elapsed_ms(wall_started)
    first_page_cpu_ms = elapsed_cpu_ms(cpu_started)
    complete_started = time.perf_counter_ns()
    complete_cpu_started = time.process_time_ns()
    row_count = len(first_page)
    while rows := cursor.fetchmany(BATCH_ROWS):
        row_count += len(rows)
    complete_fetch_ms = elapsed_ms(complete_started)
    complete_fetch_cpu_ms = elapsed_cpu_ms(complete_cpu_started)
    return {
        "execute_ms": execute_ms,
        "first_page_ms": first_page_ms,
        "complete_fetch_ms": complete_fetch_ms,
        "total_ms": elapsed_ms(wall_started),
        "first_page_process_cpu_ms": first_page_cpu_ms,
        "complete_fetch_process_cpu_ms": complete_fetch_cpu_ms,
        "total_process_cpu_ms": elapsed_cpu_ms(cpu_started),
        "first_page_rows": len(first_page),
        "row_count": row_count,
    }


def assert_timed_counts(strategy_results: dict[str, Any]) -> None:
    reference = strategy_results["sqlite"]
    for family in ("process", "ambient"):
        for range_name, metrics in reference[family].items():
            counts = {
                strategy_results[strategy][family][range_name]["row_count"]
                for strategy in STRATEGIES
            }
            if len(counts) != 1:
                raise RuntimeError(f"timed row count mismatch for {family}/{range_name}: {counts}")
            if metrics["row_count"] not in counts:
                raise AssertionError("unreachable timed count comparison")


def assert_validation_counts(repetitions: Sequence[dict[str, Any]], validation: dict[str, Any]) -> None:
    for result in repetitions:
        strategy_results = result.get("strategies")
        candidates = strategy_results.values() if strategy_results else (result,)
        for candidate in candidates:
            for family in ("process", "ambient"):
                for range_name, metrics in candidate[family].items():
                    expected = validation["ranges"][family][range_name]["rows"]
                    if metrics["row_count"] != expected:
                        raise RuntimeError(
                            f"timed/validated row count mismatch for {family}/{range_name}: "
                            f"{metrics['row_count']} != {expected}"
                        )


def validate_queries(
    args: argparse.Namespace,
    prepared: dict[str, Any],
    strategies: Sequence[str] = STRATEGIES,
) -> dict[str, Any]:
    results = {}
    all_passed = True
    for family in ("process", "ambient"):
        results[family] = {}
        for range_item in prepared["ranges"]:
            reference_connection = open_strategy("sqlite", args)
            candidate_connections = {
                strategy: open_strategy(strategy, args)
                for strategy in strategies
                if strategy != "sqlite"
            }
            try:
                reference_cursor = reference_connection.execute(
                    query_sql("sqlite", family, args.output / "parquet"),
                    query_parameters(family, range_item),
                )
                candidates = {
                    strategy: connection.execute(
                        query_sql(strategy, family, args.output / "parquet"),
                        query_parameters(family, range_item),
                    )
                    for strategy, connection in candidate_connections.items()
                }
                validation = compare_query_streams(family, reference_cursor, candidates)
                results[family][range_item["name"]] = validation
                all_passed &= validation["passed"]
            finally:
                reference_connection.close()
                for connection in candidate_connections.values():
                    connection.close()
    return {
        "all_passed": all_passed,
        "float_tolerance": FLOAT_TOLERANCE,
        "discrete_fields": "exact",
        "client_batch_rows": BATCH_ROWS,
        "ranges": results,
    }


def compare_query_streams(family: str, reference: Any, candidates: dict[str, Any]) -> dict[str, Any]:
    count = 0
    while True:
        reference_rows = reference.fetchmany(BATCH_ROWS)
        candidate_batches = {
            strategy: cursor.fetchmany(BATCH_ROWS) for strategy, cursor in candidates.items()
        }
        if any(len(rows) != len(reference_rows) for rows in candidate_batches.values()):
            return {"passed": False, "rows": count, "error": "batch length mismatch"}
        if not reference_rows:
            break
        for offset, expected in enumerate(reference_rows):
            for strategy, rows in candidate_batches.items():
                if not query_row_equal(family, expected, rows[offset]):
                    return {
                        "passed": False,
                        "rows": count,
                        "error": f"{strategy} mismatch at result row {count}",
                    }
            count += 1
    return {"passed": True, "rows": count}


def query_row_equal(family: str, expected: Sequence[Any], actual: Sequence[Any]) -> bool:
    if family == "ambient":
        return exact_row_equal(expected, actual, {2, 3})
    if len(expected) != len(actual):
        return False
    for index, (left, right) in enumerate(zip(expected, actual)):
        if index in (2, 3):
            if not float_equal(float(left), float(right)):
                return False
        elif left != right:
            return False
    return True


def query_sql(strategy: str, family: str, parquet_dir: Path) -> str:
    if family == "process":
        relation = process_relation(strategy, parquet_dir)
        return (
            "SELECT pid, process_name, AVG(cpu_usage) AS avg_cpu, "
            "AVG(memory_usage) AS avg_memory, COUNT(*) AS sample_count, "
            "MAX(execution_sec) AS max_execution, MAX(timestamp) AS latest "
            f"FROM {relation} WHERE timestamp BETWEEN ? AND ? "
            "GROUP BY pid, process_name "
            "ORDER BY avg_cpu DESC, pid ASC, process_name ASC"
        )
    relation = ambient_relation(strategy, parquet_dir)
    if strategy == "sqlite":
        predicate = EPOCH_MS_SQL
    else:
        predicate = "epoch_ms"
    return (
        "SELECT id, source, temperature, humidity, timestamp "
        f"FROM {relation} WHERE {predicate} BETWEEN ? AND ? ORDER BY id"
    )


def query_parameters(family: str, range_item: dict[str, Any]) -> tuple[Any, Any]:
    if family == "process":
        return range_item["start"], range_item["end"]
    return range_item["start_epoch_ms"], range_item["end_epoch_ms"]


def round_trip_sql(strategy: str, family: str, parquet_dir: Path) -> str:
    if family == "process":
        relation = process_relation(strategy, parquet_dir)
        return (
            "SELECT id, pid, process_name, cpu_usage, memory_usage, execution_sec, timestamp "
            f"FROM {relation} ORDER BY id"
        )
    relation = ambient_relation(strategy, parquet_dir)
    epoch = f"{EPOCH_MS_SQL} AS epoch_ms" if strategy == "sqlite" else "epoch_ms"
    return (
        "SELECT id, source, temperature, humidity, timestamp, "
        f"{epoch} FROM {relation} ORDER BY id"
    )


def process_relation(strategy: str, parquet_dir: Path) -> str:
    if strategy == "sqlite":
        return "PROCESS_STATS"
    if strategy == "duckdb":
        return "process_stats"
    return f"read_parquet({sql_string(parquet_dir / 'process.parquet')})"


def ambient_relation(strategy: str, parquet_dir: Path) -> str:
    if strategy == "sqlite":
        return "AMBIENT_ARCHIVE"
    if strategy == "duckdb":
        return "ambient_archive"
    return f"read_parquet({sql_string(parquet_dir / 'ambient.parquet')})"


def query_plans(
    source: Path,
    native_path: Path,
    parquet_dir: Path,
    ranges: list[dict[str, Any]],
    args: argparse.Namespace,
) -> dict[str, Any]:
    first = ranges[0]
    plans = {}
    for strategy in STRATEGIES:
        connection = (
            open_sqlite_read_only(source)
            if strategy == "sqlite"
            else open_duckdb(
                native_path if strategy == "duckdb" else None,
                args.output / "duckdb-temp",
                args.threads,
                args.memory_limit,
                read_only=strategy == "duckdb",
            )
        )
        try:
            plans[strategy] = {}
            for family in ("process", "ambient"):
                prefix = "EXPLAIN QUERY PLAN " if strategy == "sqlite" else "EXPLAIN "
                rows = connection.execute(
                    prefix + query_sql(strategy, family, parquet_dir),
                    query_parameters(family, first),
                ).fetchall()
                plans[strategy][family] = [list(row) for row in rows]
        finally:
            connection.close()
    return plans


def open_strategy(strategy: str, args: argparse.Namespace) -> Any:
    if strategy == "sqlite":
        return open_sqlite_read_only(args.source)
    return open_duckdb(
        args.output / "archive.duckdb" if strategy == "duckdb" else None,
        args.output / "duckdb-temp",
        args.threads,
        args.memory_limit,
        read_only=strategy == "duckdb",
    )


def open_sqlite_read_only(path: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(f"{path.as_uri()}?mode=ro", uri=True)
    connection.execute("PRAGMA query_only = ON")
    connection.execute("PRAGMA temp_store = FILE")
    connection.execute("PRAGMA temp.cache_size = -16384")
    return connection


def sqlite_policy_report(path: Path) -> dict[str, Any]:
    connection = open_sqlite_read_only(path)
    try:
        return {
            "query_only": connection.execute("PRAGMA query_only").fetchone()[0],
            "temp_store": connection.execute("PRAGMA temp_store").fetchone()[0],
            "main_cache_size": connection.execute("PRAGMA main.cache_size").fetchone()[0],
            "temp_cache_size": connection.execute("PRAGMA temp.cache_size").fetchone()[0],
            "expected_temp_store": "FILE (numeric value 1)",
            "configured_temp_cache_kib": 16384,
        }
    finally:
        connection.close()


def open_duckdb(
    path: Path | None,
    temp_dir: Path,
    threads: int,
    memory_limit: str,
    read_only: bool = False,
) -> duckdb.DuckDBPyConnection:
    connection = duckdb.connect(str(path) if path else ":memory:", read_only=read_only)
    connection.execute(f"SET threads = {int(threads)}")
    connection.execute("SET memory_limit = ?", [memory_limit])
    connection.execute("SET temp_directory = ?", [str(temp_dir)])
    return connection


def exact_row_equal(
    expected: Sequence[Any], actual: Sequence[Any], float_indexes: set[int]
) -> bool:
    if len(expected) != len(actual):
        return False
    for index, (left, right) in enumerate(zip(expected, actual)):
        if index in float_indexes and left is not None and right is not None:
            if struct.pack(">d", float(left)) != struct.pack(">d", float(right)):
                return False
        elif left != right:
            return False
    return True


def float_equal(reference: float, actual: float) -> bool:
    if not math.isfinite(reference) or not math.isfinite(actual):
        return False
    if struct.pack(">d", reference) == struct.pack(">d", actual):
        return True
    return abs(actual - reference) <= max(1e-9, 1e-12 * abs(reference))


def digest_row(digest: Any, row: Sequence[Any]) -> None:
    digest.update(struct.pack(">I", len(row)))
    for value in row:
        if value is None:
            digest.update(b"n")
        elif isinstance(value, int):
            digest.update(b"i" + int(value).to_bytes(8, "big", signed=True))
        elif isinstance(value, float):
            digest.update(b"f" + struct.pack(">d", value))
        else:
            encoded = str(value).encode("utf-8")
            digest.update(b"s" + struct.pack(">Q", len(encoded)) + encoded)


def storage_report(output: Path) -> dict[str, Any]:
    return {
        "duckdb": path_footprint(output / "archive.duckdb"),
        "parquet": path_footprint(output / "parquet"),
        "csv_export": path_footprint(output / "csv-export"),
        "duckdb_temp_at_boundary": path_footprint(output / "duckdb-temp"),
        "definition": (
            "logical file lengths including DuckDB main/WAL/metadata, Parquet metadata, "
            "CSV staging, and temp files present at the observation boundary; transient "
            "temp-file peak is not sampled inside timed drains"
        ),
    }


def path_footprint(path: Path) -> dict[str, Any]:
    files = []
    if path.is_file():
        candidates: Iterable[Path] = [path, Path(f"{path}.wal")]
    elif path.is_dir():
        candidates = sorted(item for item in path.rglob("*") if item.is_file())
    else:
        candidates = []
    total = 0
    for candidate in candidates:
        if candidate.is_file():
            size = candidate.stat().st_size
            total += size
            files.append({"path": str(candidate), "bytes": size})
    return {"total_bytes": total, "files": files}


def version_report(range_report: dict[str, Any]) -> dict[str, Any]:
    return {
        "python": sys.version,
        "duckdb": duckdb.__version__,
        "python_sqlite": sqlite3.sqlite_version,
        "sqlite_client": "pysqlite3 client binding benchmark",
        "source_benchmark_sqlite": range_report["sqlite"]["version"],
    }


def config_report(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "repetitions": args.repetitions,
        "threads": args.threads,
        "memory_limit": args.memory_limit,
        "client_batch_rows": BATCH_ROWS,
        "export_batch_rows": EXPORT_BATCH_ROWS,
        "parquet_compression": "ZSTD",
        "parquet_row_group_size": PARQUET_ROW_GROUP_SIZE,
    }


def limitations() -> list[str]:
    return [
        "Synthetic immutable source databases only; no production migration, maintenance, retention, or query wiring.",
        "Ambient epoch_ms is a stored source-SQLite semantic adapter, not evidence of native DuckDB timestamp compatibility.",
        "DuckDB memory_limit is not a whole-process memory cap; ru_maxrss is a process-lifetime high-water mark.",
        "Transient DuckDB temp-file peak is not sampled inside timed result drains.",
        "Query results are drained in bounded client batches, but engines may allocate or spill internally.",
        "Broad SQLite storage-class and concurrency contracts are handled by separate probes; failures there keep production acceptance false.",
        "Cache state is not controlled cold-cache evidence.",
    ]


def rss_report() -> dict[str, Any]:
    raw = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    if sys.platform == "darwin":
        value = int(raw)
        unit = "bytes"
    else:
        value = int(raw) * 1024
        unit = "KiB converted to bytes on this platform"
    return {
        "maximum_resident_set_size_bytes": value,
        "source_unit": unit,
        "scope": "entire current process lifetime; not incremental query memory",
    }


def phase_started() -> tuple[int, int]:
    return time.perf_counter_ns(), time.process_time_ns()


def phase_finished(started: tuple[int, int]) -> dict[str, float]:
    return {
        "wall_ms": elapsed_ms(started[0]),
        "process_cpu_ms": elapsed_cpu_ms(started[1]),
    }


def elapsed_ms(started_ns: int) -> float:
    return (time.perf_counter_ns() - started_ns) / 1_000_000


def elapsed_cpu_ms(started_ns: int) -> float:
    return (time.process_time_ns() - started_ns) / 1_000_000


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while block := handle.read(1024 * 1024):
            digest.update(block)
    return digest.hexdigest()


def sql_string(value: Path | str) -> str:
    return "'" + str(value).replace("'", "''") + "'"


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n")


if __name__ == "__main__":
    raise SystemExit(main())
