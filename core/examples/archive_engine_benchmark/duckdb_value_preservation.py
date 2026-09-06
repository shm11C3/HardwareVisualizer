#!/usr/bin/env python3
"""Bounded DuckDB prototype for lossless SQLite value preservation.

This proves storage round trips for SQLite's observed storage classes. It does
not approve query semantics, migration, recovery, or a production format.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

import duckdb
import pysqlite3 as sqlite3

REQUIRED_SQLITE_VERSION = "3.46.0"
REQUIRED_DUCKDB_VERSION = "v1.5.5"

COLUMNS = (
    "family",
    "pid",
    "process_name",
    "cpu_usage",
    "memory_usage",
    "timestamp",
    "source",
    "temperature",
    "humidity",
    "opaque",
)
QUERY_COLUMNS = {
    "process": {"pid", "process_name", "cpu_usage", "memory_usage", "timestamp"},
    "ambient": {"timestamp", "source", "temperature", "humidity"},
}
TAG = {"null": 0, "integer": 1, "real": 2, "text": 3, "blob": 4}
TAG_NAME = {value: key for key, value in TAG.items()}


@dataclass(frozen=True)
class Cell:
    dataset: str
    row_id: int
    ordinal: int
    storage_class: str
    payload: bytes


@dataclass(frozen=True)
class SourceRow:
    row_id: int
    family: str
    cells: tuple[Cell, ...]


def _sql_path(path: Path) -> str:
    return path.as_posix().replace("'", "''")


def _encode(storage_class: str, value: Any) -> bytes:
    if storage_class == "null":
        if value is not None:
            raise ValueError("NULL cell carried a value")
        return b""
    if storage_class == "integer":
        return struct.pack(">q", value)
    if storage_class == "real":
        return struct.pack(">d", value)
    if storage_class == "text":
        if not isinstance(value, bytes):
            raise TypeError("SQLite TEXT must be read with text_factory=bytes")
        return value
    if storage_class == "blob":
        return bytes(value)
    raise ValueError(f"unsupported SQLite storage class: {storage_class}")


def _decode(cell: Cell) -> Any:
    if cell.storage_class == "null":
        return None
    if cell.storage_class == "integer":
        return struct.unpack(">q", cell.payload)[0]
    if cell.storage_class == "real":
        return struct.unpack(">d", cell.payload)[0]
    if cell.storage_class in {"text", "blob"}:
        return cell.payload
    raise ValueError(f"unsupported tag: {cell.storage_class}")


def _configure_duckdb(connection: Any, spill_directory: Path) -> None:
    spill_directory.mkdir(parents=True)
    connection.execute(f"SET temp_directory = '{_sql_path(spill_directory)}'")
    connection.execute("SET threads = 1")


def _edge_source(path: Path) -> list[Cell]:
    connection = sqlite3.connect(path)
    try:
        connection.execute("CREATE TABLE edge_cells (row_id INTEGER PRIMARY KEY, value)")
        connection.executemany(
            "INSERT INTO edge_cells VALUES (?, ?)",
            [
                (-(2**63), b"row-id-min"),
                (1, None),
                (2, -(2**63)),
                (3, 2**63 - 1),
                (4, -1),
                (5, -0.0),
                (6, math.nextafter(0.0, 1.0)),
                (7, math.inf),
                (8, -math.inf),
                (9, "same\0bytes"),
                (10, b"same\0bytes"),
                (11, ""),
                (12, b""),
                (2**63 - 1, b"row-id-max"),
            ],
        )
        connection.execute(
            "INSERT INTO edge_cells VALUES (13, CAST(? AS TEXT))", (b"\xff\x00bad",)
        )
        connection.commit()
        connection.text_factory = bytes
        rows = connection.execute(
            "SELECT row_id, typeof(value), value FROM edge_cells ORDER BY row_id"
        ).fetchall()
        return [
            Cell("edge", row_id, 0, storage_class.decode(), _encode(storage_class.decode(), value))
            for row_id, storage_class, value in rows
        ]
    finally:
        connection.close()


def _create_realistic_source(path: Path, row_count: int, exception_rate: float) -> None:
    connection = sqlite3.connect(path)
    try:
        connection.execute(
            """
            CREATE TABLE samples (
              row_id INTEGER PRIMARY KEY,
              family TEXT NOT NULL,
              pid INTEGER,
              process_name TEXT,
              cpu_usage REAL,
              memory_usage INTEGER,
              timestamp TEXT NOT NULL,
              source TEXT,
              temperature REAL,
              humidity REAL,
              opaque BLOB NOT NULL
            )
            """
        )
        batch: list[tuple[Any, ...]] = []
        for row_id in range(1, row_count + 1):
            timestamp = f"2026-01-01T00:{(row_id // 60) % 60:02d}:{row_id % 60:02d}.{row_id % 1000:03d}Z"
            if row_id % 2:
                cpu_usage = {
                    1: -0.0,
                    3: math.nextafter(0.0, 1.0),
                    5: math.inf,
                }.get(row_id, float(row_id % 100) / 8.0)
                memory_usage = {
                    1: -(2**63),
                    3: 2**63 - 1,
                }.get(row_id, 1024 + row_id * 4096)
                row = (
                    row_id,
                    "process",
                    100 + row_id % 17,
                    f"worker\0{row_id % 5}",
                    cpu_usage,
                    memory_usage,
                    timestamp,
                    None,
                    None,
                    None,
                    struct.pack(">I", row_id),
                )
            else:
                temperature = {
                    2: -0.0,
                    4: -math.inf,
                }.get(row_id, 18.0 + (row_id % 80) / 10.0)
                humidity = (
                    math.nextafter(0.0, 1.0)
                    if row_id == 2
                    else None
                    if row_id % 10 == 0
                    else 35.0 + row_id % 50
                )
                row = (
                    row_id,
                    "ambient",
                    None,
                    None,
                    None,
                    None,
                    timestamp,
                    f"meter\0{row_id % 3}",
                    temperature,
                    humidity,
                    struct.pack(">I", row_id),
                )
            batch.append(row)
        connection.executemany("INSERT INTO samples VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", batch)

        exception_rows = round(row_count * exception_rate)
        if exception_rows:
            selected = []
            for index in range(exception_rows):
                row_id = 1 + index * row_count // exception_rows
                wants_ambient = index % 2 == 1
                if (row_id % 2 == 0) != wants_ambient:
                    row_id = row_id + 1 if row_id < row_count else row_id - 1
                selected.append(row_id)
            for index, row_id in enumerate(selected):
                family = "process" if row_id % 2 else "ambient"
                if family == "process":
                    connection.execute(
                        "UPDATE samples SET process_name=CAST(? AS TEXT), memory_usage=? WHERE row_id=?",
                        (b"\xffproc\x00", 1.5 if index % 2 == 0 else b"\x80mem", row_id),
                    )
                else:
                    connection.execute(
                        "UPDATE samples SET source=CAST(? AS TEXT), temperature=? WHERE row_id=?",
                        (b"\xffmeter\x00", "not-a-temperature", row_id),
                    )
                if index % 3 == 0:
                    connection.execute(
                        "UPDATE samples SET timestamp=? WHERE row_id=?",
                        (b"2026-01-01T00:00:00.999500Z", row_id),
                    )
        connection.commit()
    finally:
        connection.close()


def _read_source_rows(path: Path) -> list[SourceRow]:
    connection = sqlite3.connect(path)
    try:
        connection.text_factory = bytes
        expressions = ", ".join(
            f"typeof({column}), {column}" for column in COLUMNS
        )
        rows = connection.execute(
            f"SELECT row_id, {expressions} FROM samples ORDER BY row_id"
        ).fetchall()
        result: list[SourceRow] = []
        for row in rows:
            row_id = row[0]
            pairs = row[1:]
            cells = []
            for ordinal, column in enumerate(COLUMNS):
                storage_class = pairs[ordinal * 2].decode()
                value = pairs[ordinal * 2 + 1]
                cells.append(
                    Cell("samples", row_id, ordinal, storage_class, _encode(storage_class, value))
                )
            family = _decode(cells[0]).decode("utf-8")
            result.append(SourceRow(row_id, family, tuple(cells)))
        return result
    finally:
        connection.close()


def _encode_envelope(cells: tuple[Cell, ...]) -> bytes:
    envelope = bytearray(struct.pack(">H", len(cells)))
    for cell in cells:
        envelope.extend(
            struct.pack(">HBI", cell.ordinal, TAG[cell.storage_class], len(cell.payload))
        )
        envelope.extend(cell.payload)
    return bytes(envelope)


def _decode_envelope(dataset: str, row_id: int, envelope: bytes) -> list[Cell]:
    view = memoryview(envelope)
    if len(view) < 2:
        raise ValueError("truncated row envelope")
    count = struct.unpack_from(">H", view)[0]
    offset = 2
    cells: list[Cell] = []
    for _ in range(count):
        if offset + 7 > len(view):
            raise ValueError("truncated cell header")
        ordinal, tag, payload_length = struct.unpack_from(">HBI", view, offset)
        offset += 7
        if offset + payload_length > len(view):
            raise ValueError("truncated cell payload")
        payload = bytes(view[offset : offset + payload_length])
        offset += payload_length
        cells.append(Cell(dataset, row_id, ordinal, TAG_NAME[tag], payload))
    if offset != len(view):
        raise ValueError("trailing row-envelope bytes")
    return cells


def _tagged_roundtrip(path: Path, spill: Path, cells: Iterable[Cell]) -> dict[str, Any]:
    cell_list = list(cells)
    grouped: dict[tuple[str, int], list[Cell]] = {}
    for cell in cell_list:
        grouped.setdefault((cell.dataset, cell.row_id), []).append(cell)
    envelope_rows = [
        (dataset, row_id, _encode_envelope(tuple(row_cells)))
        for (dataset, row_id), row_cells in grouped.items()
    ]

    connection = duckdb.connect(str(path))
    _configure_duckdb(connection, spill)
    started = time.perf_counter()
    connection.execute(
        """
        CREATE TABLE tagged_rows (
          dataset VARCHAR NOT NULL,
          row_id BIGINT NOT NULL,
          envelope BLOB NOT NULL,
          PRIMARY KEY (dataset, row_id)
        )
        """
    )
    connection.executemany(
        "INSERT INTO tagged_rows VALUES (?, ?, ?)", envelope_rows
    )
    insert_ms = (time.perf_counter() - started) * 1000.0
    connection.execute("CHECKPOINT")
    connection.close()

    connection = duckdb.connect(str(path), read_only=True)
    observed = connection.execute(
        "SELECT dataset, row_id, envelope FROM tagged_rows ORDER BY dataset, row_id"
    ).fetchall()
    connection.close()
    actual = [
        cell
        for dataset, row_id, envelope in observed
        for cell in _decode_envelope(dataset, row_id, bytes(envelope))
    ]
    mismatches = _compare_cells(cell_list, actual)
    return {
        "roundtrip_exact": not mismatches,
        "mismatches": mismatches[:20],
        "source_rows": len(envelope_rows),
        "persisted_rows": len(observed),
        "source_cells": len(cell_list),
        "persisted_cells": len(actual),
        "insert_ms": insert_ms,
        "insert_ms_scope": (
            "CREATE TABLE plus Python executemany using DuckDB connection defaults; "
            "no explicit transaction"
        ),
        "database_bytes": path.stat().st_size,
        "query_support": (
            "Tag-aware decode or projections are required. No SQL aggregate or timestamp "
            "semantics are accepted by this storage-only result."
        ),
    }

def _compare_cells(reference: list[Cell], actual: list[Cell]) -> list[dict[str, Any]]:
    mismatches: list[dict[str, Any]] = []
    if len(reference) != len(actual):
        mismatches.append({"kind": "cell_count", "reference": len(reference), "actual": len(actual)})
        return mismatches
    for expected, observed in zip(reference, actual, strict=True):
        if expected != observed:
            mismatches.append(
                {
                    "key": [expected.dataset, expected.row_id, expected.ordinal],
                    "expected_tag": expected.storage_class,
                    "actual_tag": observed.storage_class,
                    "expected_payload_hex": expected.payload.hex(),
                    "actual_payload_hex": observed.payload.hex(),
                }
            )
    return mismatches


def _expected_storage_class(family: str, column: str, value: Any) -> str:
    if value is None:
        return "null"
    if column in {"family", "process_name", "timestamp", "source"}:
        return "text"
    if column in {"pid", "memory_usage"}:
        return "integer"
    if column in {"cpu_usage", "temperature", "humidity"}:
        return "real"
    if column == "opaque":
        return "blob"
    raise ValueError(column)


def _fits_typed_column(cell: Cell, expected_storage_class: str) -> bool:
    if cell.storage_class != expected_storage_class:
        return False
    if cell.storage_class == "text":
        try:
            cell.payload.decode("utf-8")
        except UnicodeDecodeError:
            return False
    return True


def _typed_value(cell: Cell) -> Any:
    value = _decode(cell)
    if cell.storage_class == "text":
        return value.decode("utf-8")
    return value


def _sidecar_roundtrip(path: Path, spill: Path, source_rows: list[SourceRow]) -> dict[str, Any]:
    connection = duckdb.connect(str(path))
    _configure_duckdb(connection, spill)
    connection.execute(
        """
        CREATE TABLE typed_rows (
          row_id BIGINT PRIMARY KEY, family VARCHAR NOT NULL, pid BIGINT,
          process_name VARCHAR, cpu_usage DOUBLE, memory_usage BIGINT,
          timestamp VARCHAR, source VARCHAR, temperature DOUBLE,
          humidity DOUBLE, opaque BLOB
        )
        """
    )
    connection.execute(
        """
        CREATE TABLE exceptional_cells (
          row_id BIGINT NOT NULL, ordinal SMALLINT NOT NULL,
          storage_tag UTINYINT NOT NULL, payload BLOB NOT NULL,
          PRIMARY KEY (row_id, ordinal)
        )
        """
    )
    typed_rows = []
    sidecar = []
    exceptional_rows: set[int] = set()
    exceptional_query_rows: set[int] = set()
    for source_row in source_rows:
        values: list[Any] = [source_row.row_id]
        for column, cell in zip(COLUMNS, source_row.cells, strict=True):
            decoded = _decode(cell)
            expected = _expected_storage_class(source_row.family, column, decoded)
            if _fits_typed_column(cell, expected):
                values.append(_typed_value(cell))
            else:
                values.append(None)
                sidecar.append((cell.row_id, cell.ordinal, TAG[cell.storage_class], cell.payload))
                exceptional_rows.add(cell.row_id)
                if column in QUERY_COLUMNS[source_row.family]:
                    exceptional_query_rows.add(cell.row_id)
        typed_rows.append(tuple(values))

    started = time.perf_counter()
    connection.executemany(
        "INSERT INTO typed_rows VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)", typed_rows
    )
    if sidecar:
        connection.executemany("INSERT INTO exceptional_cells VALUES (?, ?, ?, ?)", sidecar)
    insert_ms = (time.perf_counter() - started) * 1000.0
    connection.execute("CHECKPOINT")
    connection.close()

    connection = duckdb.connect(str(path), read_only=True)
    native_rows = connection.execute("SELECT * FROM typed_rows ORDER BY row_id").fetchall()
    exception_rows = connection.execute(
        "SELECT row_id, ordinal, storage_tag, payload FROM exceptional_cells ORDER BY row_id, ordinal"
    ).fetchall()
    connection.close()
    exception_map = {
        (row_id, ordinal): Cell("samples", row_id, ordinal, TAG_NAME[tag], bytes(payload))
        for row_id, ordinal, tag, payload in exception_rows
    }
    actual: list[Cell] = []
    for row in native_rows:
        row_id = row[0]
        family = row[1]
        for ordinal, (column, value) in enumerate(zip(COLUMNS, row[1:], strict=True)):
            exceptional = exception_map.get((row_id, ordinal))
            if exceptional is not None:
                actual.append(exceptional)
                continue
            storage_class = _expected_storage_class(family, column, value)
            if storage_class == "text":
                value = value.encode("utf-8")
            actual.append(Cell("samples", row_id, ordinal, storage_class, _encode(storage_class, value)))
    reference = [cell for row in source_rows for cell in row.cells]
    mismatches = _compare_cells(reference, actual)
    typed_only_allowed = not exceptional_query_rows
    return {
        "roundtrip_exact": not mismatches,
        "mismatches": mismatches[:20],
        "source_rows": len(source_rows),
        "persisted_rows": len(native_rows),
        "sidecar_cells": len(sidecar),
        "rows_with_any_exceptions": len(exceptional_rows),
        "exception_rows_by_family": {
            family: sum(
                row.row_id in exceptional_rows and row.family == family
                for row in source_rows
            )
            for family in ("process", "ambient")
        },
        "rows_with_query_column_exceptions": len(exceptional_query_rows),
        "insert_ms": insert_ms,
        "insert_ms_scope": (
            "Python executemany only; table creation excluded; DuckDB connection defaults "
            "and no explicit transaction"
        ),
        "database_bytes": path.stat().st_size,
        "typed_only_query_allowed": typed_only_allowed,
        "query_routing": (
            "typed projection permitted: no query-column exceptions"
            if typed_only_allowed
            else "refuse typed-only query or route every exceptional row through tag-aware evaluation"
        ),
        "silent_exception_row_drop_allowed": False,
        "query_semantics_proved": False,
    }


def _edge_probe(run_directory: Path) -> dict[str, Any]:
    sqlite_path = run_directory / "edge-source.sqlite3"
    duckdb_path = run_directory / "edge-tagged.duckdb"
    cells = _edge_source(sqlite_path)
    result = _tagged_roundtrip(
        duckdb_path, run_directory / "edge-tagged-spill", cells
    )
    by_row = {cell.row_id: cell for cell in cells}
    result.update(
        {
            "observed_real_bits": {
                str(row_id): by_row[row_id].payload.hex() for row_id in (5, 6, 7, 8)
            },
            "sqlite_observed_negative_zero": by_row[5].payload.hex()
            == struct.pack(">d", -0.0).hex(),
            "tagged_roundtrip_preserved_observed_negative_zero": (
                result["roundtrip_exact"]
                and by_row[5].payload.hex() == struct.pack(">d", -0.0).hex()
            ),
            "sqlite_observed_both_infinities": (
                by_row[7].payload == struct.pack(">d", math.inf)
                and by_row[8].payload == struct.pack(">d", -math.inf)
            ),
            "text_blob_same_bytes_distinguished": (
                by_row[9].payload == by_row[10].payload
                and by_row[9].storage_class == "text"
                and by_row[10].storage_class == "blob"
            ),
            "invalid_utf8_text_preserved": by_row[13].storage_class == "text"
            and by_row[13].payload == b"\xff\x00bad",
            "row_ids_preserved": [cell.row_id for cell in cells]
            == [-(2**63), *range(1, 14), 2**63 - 1],
            "extreme_row_ids_checked_as_decimal_strings": [
                str(-(2**63)),
                str(2**63 - 1),
            ],
            "artifacts": [str(sqlite_path), str(duckdb_path)],
        }
    )
    return result


def _workload_probe(
    run_directory: Path, row_count: int, exception_rate: float
) -> dict[str, Any]:
    suffix = f"{int(exception_rate * 100):02d}pct"
    sqlite_path = run_directory / f"workload-{suffix}-source.sqlite3"
    tagged_path = run_directory / f"workload-{suffix}-tagged.duckdb"
    sidecar_path = run_directory / f"workload-{suffix}-sidecar.duckdb"
    _create_realistic_source(sqlite_path, row_count, exception_rate)
    source_rows = _read_source_rows(sqlite_path)
    tagged = _tagged_roundtrip(
        tagged_path,
        run_directory / f"workload-{suffix}-tagged-spill",
        (cell for row in source_rows for cell in row.cells),
    )
    sidecar = _sidecar_roundtrip(
        sidecar_path,
        run_directory / f"workload-{suffix}-sidecar-spill",
        source_rows,
    )
    observed_exception_cells = sidecar["sidecar_cells"]
    process_identities: dict[tuple[bytes, bytes], int] = {}
    ambient_identities: dict[bytes, int] = {}
    for row in source_rows:
        if row.family == "process":
            key = (row.cells[1].payload, row.cells[2].payload)
            process_identities[key] = process_identities.get(key, 0) + 1
        else:
            key = row.cells[6].payload
            ambient_identities[key] = ambient_identities.get(key, 0) + 1
    return {
        "requested_exception_row_rate": exception_rate,
        "row_count": row_count,
        "source_sqlite_bytes": sqlite_path.stat().st_size,
        "observed_exception_cells": observed_exception_cells,
        "observed_exception_cell_rate": observed_exception_cells / (row_count * len(COLUMNS)),
        "tagged_every_value": tagged,
        "typed_with_exception_sidecar": sidecar,
        "roundtrip_contract_passed": tagged["roundtrip_exact"] and sidecar["roundtrip_exact"],
        "row_ids_and_multiplicity_preserved": (
            tagged["persisted_rows"] == sidecar["persisted_rows"] == row_count
        ),
        "source_identity_multiplicity": {
            "max_process_pid_name_observations": max(process_identities.values()),
            "max_ambient_source_observations": max(ambient_identities.values()),
        },
        "storage_bytes_ratio_sidecar_over_tagged": (
            sidecar["database_bytes"] / tagged["database_bytes"]
            if tagged["database_bytes"]
            else None
        ),
        "artifacts": [str(sqlite_path), str(tagged_path), str(sidecar_path)],
    }


def run_value_preservation(output: Path, row_count: int = 20_000) -> dict[str, Any]:
    """Run exact-value storage probes and retain isolated synthetic artifacts."""
    if row_count < 100:
        raise ValueError("row_count must be at least 100")
    if sqlite3.sqlite_version != REQUIRED_SQLITE_VERSION:
        raise RuntimeError(
            f"SQLite {REQUIRED_SQLITE_VERSION} required, found {sqlite3.sqlite_version}"
        )
    version_connection = duckdb.connect(":memory:")
    try:
        duckdb_version = version_connection.execute("SELECT version()").fetchone()[0]
    finally:
        version_connection.close()
    if duckdb_version != REQUIRED_DUCKDB_VERSION:
        raise RuntimeError(f"DuckDB {REQUIRED_DUCKDB_VERSION} required, found {duckdb_version}")

    output = output.expanduser().resolve()
    output.mkdir(parents=True, exist_ok=True)
    run_directory = output / f"duckdb-value-preservation-{uuid.uuid4().hex}"
    run_directory.mkdir()
    edge = _edge_probe(run_directory)
    workloads = {
        "0_percent": _workload_probe(run_directory, row_count, 0.0),
        "1_percent": _workload_probe(run_directory, row_count, 0.01),
    }
    roundtrip_passed = edge["roundtrip_exact"] and all(
        item["roundtrip_contract_passed"] for item in workloads.values()
    )
    return {
        "schema_version": 1,
        "completed": True,
        "roundtrip_contract_passed": roundtrip_passed,
        "query_contract_accepted": False,
        "production_format_accepted": False,
        "decision": (
            "Tagged cells preserve observed SQLite values exactly. Query adapters must "
            "prove range, grouping, numeric, null, and exceptional-row semantics separately."
        ),
        "identity_contract": {
            "row": "SQLite row_id and multiplicity are preserved",
            "process": "(pid, process_name bytes); PID alone is not identity",
            "ambient": "source bytes and original timestamp bytes remain authoritative",
        },
        "representation": {
            "tagged_row_envelope": (
                "(dataset, row_id, envelope BLOB); envelope repeats "
                "(ordinal, storage_tag, payload_length, payload)"
            ),
            "payloads": {
                "NULL": "empty payload plus NULL tag",
                "INTEGER": "signed i64 big-endian bytes",
                "REAL": "observed IEEE-754 binary64 bits",
                "TEXT": "exact SQLite text bytes",
                "BLOB": "exact bytes with a distinct tag",
            },
            "typed_sidecar": (
                "normal typed columns; exceptional cells become NULL in the typed row and "
                "retain tag+payload in a keyed sidecar"
            ),
        },
        "versions": {
            "sqlite": sqlite3.sqlite_version,
            "sqlite_binding": "pysqlite3 0.5.4",
            "duckdb": duckdb_version,
        },
        "artifact_directory": str(run_directory),
        "edge_cases": edge,
        "workloads": workloads,
        "timestamp_policy": (
            "Original timestamp bytes are retained. The existing SQLite predicate adapter "
            "remains separate evidence; this prototype adds no timestamp normalization."
        ),
        "measurement_scope": (
            "Synthetic storage size and Python executemany timing only. Tagged timing includes "
            "CREATE TABLE while sidecar timing excludes table creation; both use connection "
            "default transaction behavior without an explicit transaction, so they are not a "
            "fair relative engine, format, or CPU comparison. No Rust IPC, query latency, RSS, "
            "recovery, migration, or bundle-size claim."
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--rows", type=int, default=20_000)
    arguments = parser.parse_args()
    report = run_value_preservation(arguments.output, arguments.rows)
    rendered = json.dumps(report, indent=2, sort_keys=True)
    report_path = arguments.output.expanduser().resolve() / "duckdb-value-preservation.json"
    report_path.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)
    return 0 if report["roundtrip_contract_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
