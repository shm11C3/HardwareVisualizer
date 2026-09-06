#!/usr/bin/env python3
"""Bounded SQLite/DuckDB archive-query compatibility probe for #2083."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import duckdb
import pysqlite3 as sqlite3

SQLITE_VERSION = "3.46.0"
DUCKDB_VERSION = "v1.5.5"
EPOCH_MS = (
    "(CAST(strftime('%s', {column}) AS INTEGER) * 1000 + "
    "CAST(substr(strftime('%f', {column}), 4, 3) AS INTEGER))"
)
TABLES = {
    "PROCESS_STATS": (
        ("pid", "integer"),
        ("process_name", "text"),
        ("cpu_usage", "real"),
        ("memory_usage", "integer"),
        ("execution_sec", "integer"),
        ("timestamp", "text"),
    ),
    "AMBIENT_ARCHIVE": (
        ("source", "text"),
        ("temperature", "real"),
        ("humidity", ("real", "null")),
        ("timestamp", "text"),
    ),
    "DATA_ARCHIVE": (
        ("cpu_temperature_avg", ("real", "null")),
        ("timestamp", "text"),
    ),
}
QUERY_COLUMNS = {
    "PROCESS_STATS": {column for column, _ in TABLES["PROCESS_STATS"]},
    "AMBIENT_ARCHIVE": {"source", "temperature", "timestamp"},
    "DATA_ARCHIVE": {"cpu_temperature_avg", "timestamp"},
}
TAG = {"null": 0, "integer": 1, "real": 2, "text": 3, "blob": 4}
TAG_NAME = {value: key for key, value in TAG.items()}
AMBIENT_CALL_SITE_INVENTORY = {
    "covered_direct_read": [
        "src-tauri/src/commands/hardware.rs::get_ambient_archive_series",
        "src-tauri/src/services/archive_history_service.rs::fetch_ambient_archive_series",
        "core/src/infrastructure/database/archive_queries.rs::select_ambient_archive_series",
    ],
    "out_of_scope_raw_reads": [
        "core/src/infrastructure/database/cooling_thermal_delta_daily_summary.rs::select_thermal_delta_minutes_for_range",
        "core/src/infrastructure/database/cooling_thermal_delta_daily_summary.rs::max_pairable_ambient_archive_timestamp_before",
        "core/src/infrastructure/database/cooling_covariate_daily_summary.rs::max_classifiable_pairable_ambient_archive_timestamp_before",
    ],
    "related_but_not_this_raw_query": [
        "core/src/infrastructure/database/ambient_archive.rs owns insert and retention delete",
        "Cooling Insight reads derived daily/hourly/thermal-delta/covariate summaries",
        "MetricsSnapshot supplies archive writes; it does not read AMBIENT_ARCHIVE",
    ],
}
PROCESS_SQL = """SELECT
  pid,
  process_name,
  AVG(cpu_usage) AS avg_cpu_usage,
  AVG(memory_usage) AS avg_memory_usage,
  MAX(execution_sec) AS total_execution_sec,
  MAX(timestamp) AS latest_timestamp,
  COUNT(*) AS diagnostic_count
FROM PROCESS_STATS
WHERE timestamp BETWEEN ? AND ?
GROUP BY pid, process_name
ORDER BY avg_cpu_usage DESC"""


@dataclass(frozen=True)
class Cell:
    table: str
    row_id: int
    column: str
    storage: str
    payload: bytes


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def encode(storage: str, value: Any) -> bytes:
    if storage == 'null':
        return b''
    if storage == 'integer':
        return struct.pack('>q', value)
    if storage == 'real':
        return struct.pack('>d', value)
    if storage == 'text':
        if not isinstance(value, bytes):
            raise TypeError('SQLite TEXT must be read as bytes')
        return value
    if storage == 'blob':
        return bytes(value)
    raise ValueError(storage)


def decode(cell: Cell) -> Any:
    if cell.storage == 'null':
        return None
    if cell.storage == 'integer':
        return struct.unpack('>q', cell.payload)[0]
    if cell.storage == 'real':
        return struct.unpack('>d', cell.payload)[0]
    return cell.payload


def fits(cell: Cell, expected: str | tuple[str, ...]) -> bool:
    accepted = (expected,) if isinstance(expected, str) else expected
    if cell.storage not in accepted:
        return False
    if cell.storage == 'text':
        try:
            cell.payload.decode('utf-8')
        except UnicodeDecodeError:
            return False
    return True


def typed(cell: Cell) -> Any:
    value = decode(cell)
    return value.decode('utf-8') if cell.storage == 'text' else value


def create_source(path: Path, exceptional: bool) -> bool:
    db = sqlite3.connect(path)
    try:
        db.executescript(
            """
            CREATE TABLE PROCESS_STATS (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              pid INTEGER NOT NULL,
              process_name TEXT NOT NULL,
              cpu_usage REAL NOT NULL,
              memory_usage INTEGER NOT NULL,
              execution_sec INTEGER NOT NULL,
              timestamp DATETIME NOT NULL
            );
            CREATE TABLE AMBIENT_ARCHIVE (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              source TEXT NOT NULL,
              temperature REAL NOT NULL,
              humidity REAL,
              timestamp DATETIME NOT NULL
            );
            CREATE TABLE DATA_ARCHIVE (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              cpu_temperature_avg REAL,
              timestamp DATETIME NOT NULL
            );
            """
        )
        process = [
            (1, 7, 'alpha\x00worker', 1.0, 100, 3, '2026-01-01T00:00:00.000Z'),
            (2, 7, 'alpha\x00worker', 3.0, 200, 9, '2026-01-01T00:02:00.000Z'),
            (3, 7, 'beta', 5.0, 2 ** 53 + 1, 4, '2026-01-01T00:01:00.000Z'),
            (4, 7, 'beta', 5.0, 2 ** 53 + 3, 8, '2026-01-01T00:03:00.000Z'),
            (5, 10, 'cancel', 4.0, 2 ** 63 - 3, 1, '2026-01-01T00:04:00.000Z'),
            (6, 10, 'cancel', 4.0, 2 ** 63 - 4, 2, '2026-01-01T00:05:00.000Z'),
            (7, 10, 'cancel', 4.0, -2 ** 63 + 2052, 3, '2026-01-01T00:06:00.000Z'),
            (8, 10, 'cancel', 4.0, -2 ** 63 + 2053, 4, '2026-01-01T00:07:00.000Z'),
            (9, 20, 'tie-a', 6.0, -2 ** 63, 1, '2026-01-01T00:08:00.000Z'),
            (10, 21, 'tie-b', 6.0, 2 ** 63 - 1, 1, '2026-01-01T00:08:00.000Z'),
            (11, 99, 'end', 2.0, 512, 1, '2026-01-01T00:10:00.000Z'),
            (12, 98, 'outside', 99.0, 1, 1, '2026-01-01T00:10:00.001Z'),
            (13, 11, 'i64-order', 4.5, 2 ** 63 - 1, 1, '2026-01-01T00:04:10.000Z'),
            (14, 11, 'i64-order', 4.5, 1, 2, '2026-01-01T00:04:20.000Z'),
            (15, 11, 'i64-order', 4.5, -2 ** 63 + 1, 3, '2026-01-01T00:04:30.000Z'),
        ]
        ambient = [
            (1, 'room-a', 20.0, None, '2026-01-01T00:00:00.000Z'),
            (2, 'room-b', 22.0, 50.0, '2026-01-01 00:00:30.250+00:00'),
            (3, 'room-a', 24.0, None, '2026-01-01T01:01:00.999+01:00'),
            (4, 'room-a', 20.000000000000004, math.nextafter(0.0, 1.0), '2026-01-01T00:02:00Z'),
            (5, 'room-z', 30.0, None, '2026-01-01T00:10:00.000Z'),
            (6, 'outside', 100.0, None, '2026-01-01T00:10:00.001Z'),
        ]
        data = [
            (1, 50.0, '2026-01-01T00:00:00.000Z'),
            (2, 52.0, '2026-01-01T00:00:50Z'),
            (3, None, '2026-01-01T00:01:00Z'),
            (4, None, '2026-01-01T00:01:30Z'),
            (5, 60.00000000000001, '2026-01-01T00:02:00Z'),
            (6, 70.0, '2026-01-01T00:10:00Z'),
            (7, 200.0, '2026-01-01T00:10:00.001Z'),
        ]
        db.executemany('INSERT INTO PROCESS_STATS VALUES (?,?,?,?,?,?,?)', process)
        db.executemany('INSERT INTO AMBIENT_ARCHIVE VALUES (?,?,?,?,?)', ambient)
        db.executemany('INSERT INTO DATA_ARCHIVE VALUES (?,?,?)', data)
        try:
            db.execute("INSERT INTO PROCESS_STATS VALUES (100,1,'null',NULL,1,1,'2026-01-01T00:00:00Z')")
            null_rejected = False
        except sqlite3.IntegrityError:
            null_rejected = True
        if exceptional:
            db.execute('INSERT INTO PROCESS_STATS VALUES (20,30,CAST(? AS TEXT),?,3,1,?)', (b'\xffbad-name', sqlite3.Binary(b'\x00cpu'), '2026-01-01T00:03:00Z'))
            db.execute('INSERT INTO PROCESS_STATS VALUES (21,31,?,1.0,3,1,?)', (sqlite3.Binary(b'blob-name'), '2026-01-01T00:03:00Z'))
            db.execute('INSERT INTO AMBIENT_ARCHIVE VALUES (20,CAST(? AS TEXT),?,NULL,?)', (b'\xffbad-source', 'not-a-temperature', '2026-01-01T00:03:00Z'))
            db.execute("INSERT INTO AMBIENT_ARCHIVE VALUES (21,'bad-time',1.0,NULL,CAST(? AS TEXT))", (b'\xffbad-time',))
        db.commit()
        return null_rejected
    finally:
        db.close()


def read_cells(path: Path) -> list[Cell]:
    db = sqlite3.connect(path)
    db.text_factory = bytes
    try:
        out = []
        for table, specs in TABLES.items():
            expr = ','.join((f'typeof({c}),{c}' for c, _ in specs))
            for row in db.execute(f'SELECT id,{expr} FROM {table} ORDER BY id'):
                for n, (column, _) in enumerate(specs):
                    storage = row[1 + n * 2].decode()
                    out.append(Cell(table, row[0], column, storage, encode(storage, row[2 + n * 2])))
        return out
    finally:
        db.close()


def create_candidate(source: Path, target: Path, spill: Path) -> list[dict[str, Any]]:
    grouped = {}
    for cell in read_cells(source):
        grouped.setdefault((cell.table, cell.row_id), {})[cell.column] = cell
    source_db = sqlite3.connect(source)
    db = duckdb.connect(str(target))
    spill.mkdir()
    db.execute(f"SET temp_directory='{spill.as_posix()}'")
    db.execute('SET threads=1')
    try:
        db.execute('CREATE TABLE PROCESS_STATS(id BIGINT PRIMARY KEY,pid BIGINT,process_name VARCHAR,cpu_usage DOUBLE,memory_usage BIGINT,execution_sec BIGINT,timestamp VARCHAR)')
        db.execute('CREATE TABLE AMBIENT_ARCHIVE(id BIGINT PRIMARY KEY,source VARCHAR,temperature DOUBLE,humidity DOUBLE,timestamp VARCHAR,epoch_ms BIGINT)')
        db.execute('CREATE TABLE DATA_ARCHIVE(id BIGINT PRIMARY KEY,cpu_temperature_avg DOUBLE,timestamp VARCHAR,epoch_ms BIGINT)')
        db.execute('CREATE TABLE exceptional_cells(table_name VARCHAR,row_id BIGINT,column_name VARCHAR,storage_tag UTINYINT,payload BLOB,PRIMARY KEY(table_name,row_id,column_name))')
        exceptions = []
        for (table, row_id), row in grouped.items():
            values = [row_id]
            for column, expected in TABLES[table]:
                cell = row[column]
                if fits(cell, expected):
                    values.append(typed(cell))
                else:
                    values.append(None)
                    exceptions.append((table, row_id, column, TAG[cell.storage], cell.payload))
            if table != 'PROCESS_STATS':
                values.append(source_db.execute(f"SELECT {EPOCH_MS.format(column='timestamp')} FROM {table} WHERE id=?", (row_id,)).fetchone()[0])
            db.execute(f"INSERT INTO {table} VALUES ({','.join(('?' for _ in values))})", values)
        if exceptions:
            db.executemany('INSERT INTO exceptional_cells VALUES (?,?,?,?,?)', exceptions)
        db.execute('CHECKPOINT')
    finally:
        source_db.close()
        db.close()
    return [{'table': t, 'row_id': i, 'column': c, 'tag': TAG_NAME[tag]} for t, i, c, tag, _ in exceptions]


def candidate_cells(path: Path) -> list[Cell]:
    db = duckdb.connect(str(path), read_only=True)
    try:
        exceptions = {(t, i, c): Cell(t, i, c, TAG_NAME[tag], bytes(payload)) for t, i, c, tag, payload in db.execute('SELECT * FROM exceptional_cells').fetchall()}
        out = []
        for table, specs in TABLES.items():
            for row in db.execute(f"SELECT id,{','.join((c for c, _ in specs))} FROM {table} ORDER BY id").fetchall():
                for n, (column, expected) in enumerate(specs):
                    if (table, row[0], column) in exceptions:
                        out.append(exceptions[table, row[0], column])
                        continue
                    value = row[n + 1]
                    accepted = (expected,) if isinstance(expected, str) else expected
                    storage = 'null' if value is None else next((x for x in accepted if x != 'null'))
                    if storage == 'text':
                        value = value.encode()
                    out.append(Cell(table, row[0], column, storage, encode(storage, value)))
        return out
    finally:
        db.close()


def serial(value: Any) -> Any:
    if value is None:
        return {'type': 'null'}
    if isinstance(value, float):
        return {'type': 'f64', 'repr': repr(value), 'bits': struct.pack('>d', value).hex()}
    if isinstance(value, int):
        return {'type': 'i64', 'decimal': str(value)}
    if isinstance(value, str):
        value = value.encode()
    if isinstance(value, bytes):
        return {'type': 'bytes', 'hex': value.hex()}
    raise TypeError(type(value))


def rows_json(rows):
    return [[serial(v) for v in row] for row in rows]


def ambient_sql(epoch: str, duckdb_mode: bool = False) -> str:
    cpu_epoch = epoch.replace("AMBIENT_ARCHIVE", "DATA_ARCHIVE")
    division = "//" if duckdb_mode else "/"
    numeric_type = "DOUBLE" if duckdb_mode else "REAL"
    return f"""WITH ambient AS (
      SELECT (({epoch}) {division} 60000) AS minute_key,
             MIN({epoch}) AS minute_epoch_ms,
             AVG(CAST(temperature AS {numeric_type})) AS ambient_value
      FROM AMBIENT_ARCHIVE
      WHERE {epoch} BETWEEN ? AND ?
      GROUP BY minute_key
    ), cpu AS (
      SELECT (({cpu_epoch}) {division} 60000) AS minute_key,
             AVG(CAST(cpu_temperature_avg AS {numeric_type})) AS cpu_value
      FROM DATA_ARCHIVE
      WHERE {cpu_epoch} BETWEEN ? AND ?
        AND cpu_temperature_avg IS NOT NULL
      GROUP BY minute_key
    )
    SELECT ((ambient.minute_epoch_ms {division} ?) * ?) AS bucket_timestamp,
           AVG(ambient_value) AS ambient_avg,
           AVG(cpu_value - ambient_value) AS delta_avg,
           COUNT(ambient_value) AS minute_count
    FROM ambient
    LEFT JOIN cpu ON cpu.minute_key = ambient.minute_key
    GROUP BY 1
    ORDER BY 1 ASC"""


def ambient_query(db, source: bool):
    start, end, width = (1767225600000, 1767226200000, 120000)
    epoch = EPOCH_MS.format(column='AMBIENT_ARCHIVE.timestamp') if source else 'AMBIENT_ARCHIVE.epoch_ms'
    rows = db.execute(ambient_sql(epoch, not source), (start, end, start, end, width, width)).fetchall()
    by_time = {r[0]: r for r in rows}
    filled = [by_time.get(t, (t, None, None, 0)) for t in range(start // width * width, end // width * width + 1, width)]
    sources = db.execute(f'SELECT DISTINCT source FROM AMBIENT_ARCHIVE WHERE {epoch} BETWEEN ? AND ? ORDER BY source ASC', (start, end)).fetchall()
    return {'buckets': rows_json(filled), 'sources': rows_json(sources)}


def rank_bands(rows):
    bands = []
    for row in rows:
        bits = struct.pack('>d', row[2]).hex()
        name = row[1].encode() if isinstance(row[1], str) else row[1]
        if not bands or bands[-1]['avg_cpu_bits'] != bits:
            bands.append({'avg_cpu_bits': bits, 'identities_unordered_within_tie': []})
        bands[-1]['identities_unordered_within_tie'].append([str(row[0]), name.hex()])
    for band in bands:
        band['identities_unordered_within_tie'].sort()
    return bands


def supported_queries(source_path: Path, candidate_path: Path) -> dict[str, Any]:
    source = sqlite3.connect(f"file:{source_path}?mode=ro", uri=True)
    source.text_factory = bytes
    candidate = duckdb.connect(str(candidate_path), read_only=True)
    try:
        bounds = ("2026-01-01T00:00:00.000Z", "2026-01-01T00:10:00.000Z")
        sqlite_process = source.execute(PROCESS_SQL, bounds).fetchall()
        duckdb_process = candidate.execute(PROCESS_SQL, bounds).fetchall()
        sqlite_ambient = ambient_query(source, True)
        duckdb_ambient = ambient_query(candidate, False)
    finally:
        source.close()
        candidate.close()

    def identity(row: tuple[Any, ...]) -> tuple[int, bytes]:
        name = row[1].encode() if isinstance(row[1], str) else row[1]
        return (row[0], name)

    sqlite_rows = rows_json(sorted(sqlite_process, key=identity))
    duckdb_rows = rows_json(sorted(duckdb_process, key=identity))
    sqlite_ranks = rank_bands(sqlite_process)
    duckdb_ranks = rank_bands(duckdb_process)
    process_exact = sqlite_rows == duckdb_rows and sqlite_ranks == duckdb_ranks
    ambient_exact = sqlite_ambient == duckdb_ambient
    sqlite_cancel = next(row for row in sqlite_process if row[0] == 10)
    duckdb_cancel = next(row for row in duckdb_process if row[0] == 10)
    sqlite_integer = next(row for row in sqlite_process if row[0] == 11)
    duckdb_integer = next(row for row in duckdb_process if row[0] == 11)
    return {
        "process": {
            "contract_exact": process_exact,
            "sqlite_rows": sqlite_rows,
            "duckdb_rows": duckdb_rows,
            "sqlite_rank_bands": sqlite_ranks,
            "duckdb_rank_bands": duckdb_ranks,
            "rank_bands_exact": sqlite_ranks == duckdb_ranks,
            "tie_order": "unspecified inside equal AVG(cpu_usage); identities are compared as an unordered band",
            "count": "COUNT(*) is diagnostic; current ProcessStatRecord does not return it",
            "cancellation": {
                "inputs": ["9223372036854775805", "9223372036854775804", "-9223372036854773756", "-9223372036854773755"],
                "sqlite": serial(sqlite_cancel[3]),
                "duckdb": serial(duckdb_cancel[3]),
                "exact": struct.pack(">d", sqlite_cancel[3]) == struct.pack(">d", duckdb_cancel[3]),
                "finding": "direct AVG agrees; the earlier 1024.5 versus 1024.0 failure required chunk partial-sum reaggregation",
            },
            "integer_avg_order_probe": {
                "inputs": ["9223372036854775807", "1", "-9223372036854775807"],
                "sqlite": serial(sqlite_integer[3]),
                "duckdb": serial(duckdb_integer[3]),
                "exact": struct.pack(">d", sqlite_integer[3]) == struct.pack(">d", duckdb_integer[3]),
                "mathematical_average": "1/3",
                "binary64_is_exact_rational": False,
                "finding": "both engines return the same rounded binary64 value; BIGINT inputs do not make AVG exact rational arithmetic",
            },
        },
        "ambient_archive_series": {
            "contract_exact": ambient_exact,
            "sqlite": sqlite_ambient,
            "duckdb": duckdb_ambient,
            "range": {"inclusive": True, "start_epoch_ms": 1767225600000, "end_epoch_ms": 1767226200000},
            "bucket_timestamp": "Start",
            "fixture_notes": [
                "minute 1 has only NULL CPU temperature values, so delta remains NULL",
                "a non-FLOAT32-representable fraction verifies the DuckDB DOUBLE adapter",
                "inclusive endpoints, fractions, offset spelling, duplicate minutes and end+1ms exclusion are exercised",
            ],
        },
        "query_contract_passed": process_exact and ambient_exact,
    }


def refused_queries(source_path: Path, candidate_path: Path, blockers: list[dict[str, Any]]) -> dict[str, Any]:
    source = sqlite3.connect(f"file:{source_path}?mode=ro", uri=True)
    source.text_factory = bytes
    try:
        process = source.execute(
            PROCESS_SQL,
            ("2026-01-01T00:00:00.000Z", "2026-01-01T00:10:00.000Z"),
        ).fetchall()
        ambient = ambient_query(source, True)
        counts = {
            table: source.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
            for table in TABLES
        }
    finally:
        source.close()
    return {
        "selected_backend": "source_sqlite",
        "typed_candidate_selected": False,
        "candidate_query_opened": False,
        "blockers": blockers,
        "unsupported_result_returned": False,
        "source_query_remained_usable": bool(process) and bool(ambient["buckets"]),
        "source_result_counts": {
            "process_groups": len(process),
            "ambient_buckets": len(ambient["buckets"]),
            "table_rows": counts,
        },
    }


def scenario(output: Path, name: str, exceptional: bool) -> dict[str, Any]:
    folder = output / name
    folder.mkdir()
    source = folder / "source.sqlite3"
    candidate = folder / "typed-sidecar.duckdb"
    null_rejected = create_source(source, exceptional)
    source_hash_before = sha256(source)
    exceptions = create_candidate(source, candidate, folder / "duckdb-spill")
    candidate_hash_before = sha256(candidate)
    exact = sorted(read_cells(source), key=repr) == sorted(candidate_cells(candidate), key=repr)
    blockers = [
        item
        for item in exceptions
        if item["column"] in QUERY_COLUMNS[item["table"]]
    ]
    query = (
        refused_queries(source, candidate, blockers)
        if blockers
        else supported_queries(source, candidate)
    )
    source_hash_after = sha256(source)
    candidate_hash_after = sha256(candidate)
    assert source_hash_before == source_hash_after
    assert candidate_hash_before == candidate_hash_after
    return {
        "source": {"path": str(source), "sha256": source_hash_before, "bytes": source.stat().st_size},
        "candidate": {"path": str(candidate), "sha256": candidate_hash_before, "bytes": candidate.stat().st_size},
        "artifacts_unchanged_by_query_or_refusal": {
            "source_sha256_equal": source_hash_before == source_hash_after,
            "candidate_sha256_equal": candidate_hash_before == candidate_hash_after,
        },
        "process_null_insert_rejected_by_current_schema": null_rejected,
        "typed_sidecar": {
            "roundtrip_exact": exact,
            "exception_cells": exceptions,
            "query_blockers": blockers,
        },
        "tagged": {
            "evaluator_implemented": False,
            "query_selected": False,
            "decision": "refuse before backend selection; storage round-trip is not query support",
        },
        "query": query,
        "query_contract_passed": False if blockers else query["query_contract_passed"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if sqlite3.sqlite_version != SQLITE_VERSION:
        raise RuntimeError(f"SQLite {SQLITE_VERSION} required, found {sqlite3.sqlite_version}")
    check = duckdb.connect(":memory:")
    version = check.execute("SELECT version()").fetchone()[0]
    check.close()
    if version != DUCKDB_VERSION:
        raise RuntimeError(f"DuckDB {DUCKDB_VERSION} required, found {version}")

    output = args.output.expanduser().resolve()
    output.mkdir(parents=True, exist_ok=False)
    scenarios = {
        "normal_typed": scenario(output, "normal-typed", False),
        "exceptional_cells": scenario(output, "exceptional-cells", True),
    }
    repo = Path(__file__).resolve().parents[3]
    source_file = Path(__file__).resolve()
    report = {
        "schema_version": 1,
        "completed": True,
        "issue": 2083,
        "versions": {"sqlite": sqlite3.sqlite_version, "duckdb": version, "python": sys.version},
        "source_revision": {
            "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
            "dirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=repo, text=True)),
        },
        "source_file": {"path": str(source_file), "sha256": sha256(source_file)},
        "command": [sys.executable, *sys.argv],
        "identity": {
            "process": "exact (pid, process_name bytes); PID alone is not identity",
            "ambient": "source labels stay distinct and sorted; no portable source id is inferred",
        },
        "covered": [
            "archive_queries::select_process_stats: six returned columns, grouping, inclusive raw-TEXT range and optional CPU ranking",
            "archive_queries::select_ambient_archive_series: Start buckets, source set, inclusive epoch range, minute pairing and gap fill",
        ],
        "ambient_call_site_inventory": AMBIENT_CALL_SITE_INVENTORY,
        "not_covered": [
            "cooling_daily_summary::select_archive_minutes_for_range (DATA_ARCHIVE half-open lane)",
            "cooling_thermal_delta_daily_summary raw pairing query and watermark lookup",
            "daily summary mutation/retention and downstream Insight or Snapshot consumers",
            "End bucket timestamps, recovery, concurrent migration, Rust SQLx decoding and performance",
        ],
        "sql": {
            "process": PROCESS_SQL,
            "ambient_sqlite": ambient_sql(EPOCH_MS.format(column="AMBIENT_ARCHIVE.timestamp")),
            "ambient_duckdb": ambient_sql("AMBIENT_ARCHIVE.epoch_ms", True),
        },
        "scenarios": scenarios,
        "roundtrip_contract_passed": all(s["typed_sidecar"]["roundtrip_exact"] for s in scenarios.values()),
        "query_contract_passed": all(s["query_contract_passed"] for s in scenarios.values()),
        "production_candidate_accepted": False,
        "decision": "Keep #2083 open: typed fit is not SQLite arithmetic proof, and participating exceptional cells require refusal until an evaluator exists.",
        "exit_status_contract": "zero means diagnostic completion and exact round trips; acceptance is only the JSON flags",
    }
    report_path = output / "result.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"report": str(report_path), "query_contract_passed": report["query_contract_passed"]}, sort_keys=True))
    return 0 if report["roundtrip_contract_passed"] else 1

if __name__ == "__main__":
    raise SystemExit(main())
