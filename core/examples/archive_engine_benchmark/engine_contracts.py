#!/usr/bin/env python3
"""Small correctness probes for archive engine candidates.

SQLite is the query and storage oracle. Passing these probes does not approve a
production format; the wider lifecycle and measurement gates remain open.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import uuid
from pathlib import Path
from typing import Any, Callable

import pysqlite3 as sqlite3

try:
    import duckdb
except ImportError as error:  # Keep missing optional tooling reportable.
    duckdb = None
    DUCKDB_IMPORT_ERROR = str(error)
else:
    DUCKDB_IMPORT_ERROR = None


FLOAT_TOLERANCE = "abs(actual-reference) <= max(1e-9, 1e-12*abs(reference))"
REQUIRED_SQLITE_VERSION = "3.46.0"
AMBIENT_EPOCH_MS_SQL = (
    "(CAST(strftime('%s', timestamp) AS INTEGER) * 1000 + "
    "CAST(substr(strftime('%f', timestamp), 4, 3) AS INTEGER))"
)
CANONICAL_ROWS = [
    (1, 41, "worker\0alpha", -0.0, -(2**63), 5, "1969-12-31T23:59:59.999Z", None, b"\x00\xff"),
    (2, 41, "worker\0alpha", 5e-324, 2**63 - 1, 7, "2026-01-01T00:00:00.010Z", 1.25, b"\x80A"),
    (3, 41, "worker\0beta", math.pi, 42, 9, "2026-01-01T00:00:00.100Z", -0.0, b""),
    (4, 42, "worker\0alpha", 1.5, -17, 11, "2026-01-01T00:00:00.100Z", 5e-324, b"dup"),
]
ROUNDTRIP_COLUMNS = (
    "id",
    "pid",
    "process_name",
    "cpu_usage",
    "memory_usage",
    "execution_sec",
    "timestamp",
    "nullable_metric",
    "opaque",
)
PROCESS_QUERY_SQL = """
SELECT pid, process_name, AVG(cpu_usage), AVG(memory_usage), COUNT(*),
       MAX(execution_sec), MAX(timestamp)
FROM {source}
WHERE timestamp BETWEEN ? AND ?
GROUP BY pid, process_name
ORDER BY AVG(cpu_usage) DESC, pid ASC, process_name ASC
"""


def _float_bits(value: float) -> str:
    return struct.pack(">d", value).hex()


def _allowed_error(reference: float) -> float:
    return max(1e-9, 1e-12 * abs(reference))


def _float_equal(reference: float, actual: float) -> bool:
    return (
        math.isfinite(reference)
        and math.isfinite(actual)
        and (
            _float_bits(reference) == _float_bits(actual)
            or abs(actual - reference) <= _allowed_error(reference)
        )
    )


def _run_probe(name: str, function: Callable[[], dict[str, Any]]) -> dict[str, Any]:
    try:
        result = function()
        result.setdefault("status", "pass" if result.get("contract_preserved") else "fail")
        return result
    except Exception as error:  # Each probe must remain visible in the report.
        return {
            "status": "unavailable",
            "contract_preserved": False,
            "error": f"{type(error).__name__}: {error}",
            "probe": name,
        }


def _create_sqlite_fixture(path: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(path)
    connection.execute(
        """
        CREATE TABLE process_rows (
          id INTEGER PRIMARY KEY,
          pid INTEGER NOT NULL,
          process_name TEXT NOT NULL,
          cpu_usage REAL NOT NULL,
          memory_usage INTEGER NOT NULL,
          execution_sec INTEGER NOT NULL,
          timestamp TEXT NOT NULL,
          nullable_metric REAL,
          opaque BLOB NOT NULL
        )
        """
    )
    connection.executemany(
        "INSERT INTO process_rows VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        CANONICAL_ROWS,
    )
    connection.commit()
    return connection


def _create_duckdb_fixture(path: Path, source_rows: list[tuple[Any, ...]]) -> Any:
    if duckdb is None:
        raise RuntimeError(f"duckdb unavailable: {DUCKDB_IMPORT_ERROR}")
    connection = duckdb.connect(str(path))
    connection.execute(
        """
        CREATE TABLE process_rows (
          id BIGINT, pid BIGINT, process_name VARCHAR, cpu_usage DOUBLE,
          memory_usage BIGINT, execution_sec BIGINT, timestamp VARCHAR,
          nullable_metric DOUBLE, opaque BLOB
        )
        """
    )
    connection.executemany(
        "INSERT INTO process_rows VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        source_rows,
    )
    return connection


def _normalize_sqlite_row(row: tuple[Any, ...]) -> tuple[Any, ...]:
    return tuple(
        value.encode("utf-8") if isinstance(value, str) else value for value in row
    )


def _normalize_candidate_row(row: tuple[Any, ...]) -> tuple[Any, ...]:
    return tuple(
        value.encode("utf-8") if isinstance(value, str) else bytes(value)
        if isinstance(value, bytearray)
        else value
        for value in row
    )


def _row_mismatches(
    reference: list[tuple[Any, ...]], actual: list[tuple[Any, ...]]
) -> list[dict[str, Any]]:
    mismatches: list[dict[str, Any]] = []
    if len(reference) != len(actual):
        mismatches.append(
            {"field": "row_count", "reference": len(reference), "actual": len(actual)}
        )
        return mismatches
    for row_index, (left, right) in enumerate(zip(reference, actual, strict=True)):
        for column_index, (expected, observed) in enumerate(
            zip(left, right, strict=True)
        ):
            equal = (
                _float_bits(expected) == _float_bits(observed)
                if isinstance(expected, float) and isinstance(observed, float)
                else expected == observed
            )
            if not equal:
                mismatches.append(
                    {
                        "row": row_index,
                        "column": ROUNDTRIP_COLUMNS[column_index],
                        "reference_type": type(expected).__name__,
                        "actual_type": type(observed).__name__,
                        "reference": _json_value(expected),
                        "actual": _json_value(observed),
                    }
                )
    return mismatches


def _roundtrip_probe(
    sqlite_connection: sqlite3.Connection,
    duck_connection: Any,
    parquet_path: Path,
) -> dict[str, Any]:
    reference = [
        _normalize_sqlite_row(row)
        for row in sqlite_connection.execute(
            "SELECT * FROM process_rows ORDER BY id"
        ).fetchall()
    ]
    seeded = [_normalize_sqlite_row(row) for row in CANONICAL_ROWS]
    native = [
        _normalize_candidate_row(row)
        for row in duck_connection.execute(
            "SELECT * FROM process_rows ORDER BY id"
        ).fetchall()
    ]
    duck_connection.execute(
        f"COPY process_rows TO '{parquet_path.as_posix()}' "
        "(FORMAT PARQUET, COMPRESSION UNCOMPRESSED)"
    )
    parquet = [
        _normalize_candidate_row(row)
        for row in duck_connection.execute(
            "SELECT * FROM read_parquet(?) ORDER BY id", [str(parquet_path)]
        ).fetchall()
    ]
    native_mismatches = _row_mismatches(reference, native)
    parquet_mismatches = _row_mismatches(reference, parquet)
    identity_count = sqlite_connection.execute(
        """
        SELECT COUNT(*) FROM process_rows
        WHERE pid = 41 AND process_name = ?
        """,
        ("worker\0alpha",),
    ).fetchone()[0]
    return {
        "contract_preserved": not native_mismatches
        and not parquet_mismatches
        and identity_count == 2,
        "native_duckdb": {
            "exact": not native_mismatches,
            "mismatches": native_mismatches,
            "persistence_boundary": "seed connection closed and reopened before read",
        },
        "parquet_via_duckdb": {
            "exact": not parquet_mismatches,
            "mismatches": parquet_mismatches,
        },
        "tuple_identity": "(pid, process_name bytes)",
        "duplicate_tuple_multiplicity": identity_count,
        "float_bits_checked": ["minimum_subnormal", "pi"],
        "negative_zero_observation": {
            "seed_bits": _float_bits(-0.0),
            "sqlite_read_bits": {
                "cpu_usage": _float_bits(reference[0][3]),
                "nullable_metric": _float_bits(reference[2][7]),
            },
            "sqlite_preserved_seed_bits": (
                _float_bits(reference[0][3]) == _float_bits(-0.0)
                and _float_bits(reference[2][7]) == _float_bits(-0.0)
            ),
            "candidate_preserved_sqlite_observed_bits": (
                _float_bits(native[0][3]) == _float_bits(reference[0][3])
                and _float_bits(native[2][7]) == _float_bits(reference[2][7])
                and _float_bits(parquet[0][3]) == _float_bits(reference[0][3])
                and _float_bits(parquet[2][7]) == _float_bits(reference[2][7])
            ),
        },
        "nullable_fields_checked": True,
        "signed_i64_extremes_checked": True,
        "fixture_to_sqlite_changes": _row_mismatches(seeded, reference),
    }


def _sqlite_affinity_cases(connection: sqlite3.Connection) -> list[dict[str, Any]]:
    connection.execute("CREATE TABLE affinity_probe (integer_value INTEGER, text_value TEXT)")
    rows = [
        (42, 42),
        (1.5, 2.5),
        ("not-an-integer", "alpha"),
        (b"\xff\x00", b"\xff\x00"),
    ]
    connection.executemany("INSERT INTO affinity_probe VALUES (?, ?)", rows)
    connection.execute(
        "INSERT INTO affinity_probe VALUES (CAST(? AS TEXT), CAST(? AS TEXT))",
        (b"\xffbad", b"\xffbad"),
    )
    connection.commit()
    connection.text_factory = bytes
    observed = connection.execute(
        """
        SELECT rowid, typeof(integer_value), integer_value,
               typeof(text_value), text_value
        FROM affinity_probe ORDER BY rowid
        """
    ).fetchall()
    cases: list[dict[str, Any]] = []
    for rowid, integer_type, integer_value, text_type, text_value in observed:
        cases.append(
            {
                "row": rowid,
                "integer_value": {
                    "storage_class": integer_type.decode(),
                    "value": integer_value,
                },
                "text_value": {
                    "storage_class": text_type.decode(),
                    "value": text_value,
                },
            }
        )
    return cases


def _json_value(value: Any) -> Any:
    if isinstance(value, bytes):
        return {"bytes_hex": value.hex()}
    if isinstance(value, float):
        return {"value": value, "binary64_bits": _float_bits(value)}
    return value


def _try_duck_cast(
    connection: Any, value: Any, sqlite_storage_class: str, target: str
) -> dict[str, Any]:
    try:
        source = (
            value.decode("utf-8")
            if sqlite_storage_class == "text" and isinstance(value, bytes)
            else value
        )
    except UnicodeDecodeError as error:
        return {
            "status": "rejected",
            "error": f"invalid UTF-8: {error}",
            "sqlite_storage_class": sqlite_storage_class,
            "target_type": target,
        }
    try:
        candidate = connection.execute(
            f"SELECT CAST(? AS {target}), typeof(CAST(? AS {target}))",
            [source, source],
        ).fetchone()
    except Exception as error:
        return {
            "status": "rejected",
            "error": f"{type(error).__name__}: {error}",
            "sqlite_storage_class": sqlite_storage_class,
            "target_type": target,
        }
    expected_class = "integer" if target == "BIGINT" else "text"
    expected = source
    exact = sqlite_storage_class == expected_class and candidate[0] == expected
    return {
        "status": "exact" if exact else "changed",
        "sqlite_storage_class": sqlite_storage_class,
        "target_type": target,
        "source": _json_value(value),
        "candidate": _json_value(candidate[0]),
        "candidate_type": candidate[1],
    }


def _affinity_probe(
    sqlite_connection: sqlite3.Connection, duck_connection: Any, parquet_path: Path
) -> dict[str, Any]:
    cases = _sqlite_affinity_cases(sqlite_connection)
    conversions: list[dict[str, Any]] = []
    for case in cases:
        conversions.append(
            {
                "row": case["row"],
                "column": "integer_value",
                **_try_duck_cast(
                    duck_connection,
                    case["integer_value"]["value"],
                    case["integer_value"]["storage_class"],
                    "BIGINT",
                ),
            }
        )
        conversions.append(
            {
                "row": case["row"],
                "column": "text_value",
                **_try_duck_cast(
                    duck_connection,
                    case["text_value"]["value"],
                    case["text_value"]["storage_class"],
                    "VARCHAR",
                ),
            }
        )
    exact_rows = [
        conversion
        for conversion in conversions
        if conversion["status"] == "exact"
    ]
    duck_connection.execute(
        "CREATE TABLE affinity_exact (row_id BIGINT, column_name VARCHAR, value_text VARCHAR)"
    )
    duck_connection.executemany(
        "INSERT INTO affinity_exact VALUES (?, ?, ?)",
        [
            (item["row"], item["column"], str(item["candidate"]))
            for item in exact_rows
        ],
    )
    duck_connection.execute(
        f"COPY affinity_exact TO '{parquet_path.as_posix()}' (FORMAT PARQUET)"
    )
    parquet_rows = duck_connection.execute(
        "SELECT COUNT(*) FROM read_parquet(?)", [str(parquet_path)]
    ).fetchone()[0]
    changed = sum(item["status"] == "changed" for item in conversions)
    rejected = sum(item["status"] == "rejected" for item in conversions)
    return {
        "contract_preserved": changed == 0 and rejected == 0,
        "native_duckdb_conversions": conversions,
        "summary": {
            "exact": len(exact_rows),
            "changed": changed,
            "rejected": rejected,
        },
        "parquet_exact_subset_rows": parquet_rows,
        "mapping_evaluated": "fixed DuckDB BIGINT/VARCHAR columns",
        "failure_scope": (
            "The tested fixed-column mapping does not preserve SQLite mixed storage "
            "classes or invalid UTF-8 TEXT. A tagged representation was not evaluated."
        ),
        "invalid_utf8_text_is_reported_separately": True,
    }


def _normalize_query_rows(rows: list[tuple[Any, ...]]) -> list[tuple[Any, ...]]:
    return [
        tuple(value.encode("utf-8") if isinstance(value, str) else value for value in row)
        for row in rows
    ]


def _query_rows_equivalent(
    reference: list[tuple[Any, ...]], actual: list[tuple[Any, ...]]
) -> bool:
    if len(reference) != len(actual):
        return False
    for left, right in zip(reference, actual, strict=True):
        if left[:2] != right[:2] or left[4:] != right[4:]:
            return False
        if not _float_equal(left[2], right[2]) or not _float_equal(left[3], right[3]):
            return False
    return True


def _process_query_probe(
    sqlite_connection: sqlite3.Connection,
    duck_connection: Any,
    parquet_path: Path,
) -> dict[str, Any]:
    start = "1969-12-31T23:59:59.999Z"
    end = "2026-01-01T00:00:00.100Z"
    sqlite_connection.text_factory = str
    reference = _normalize_query_rows(
        sqlite_connection.execute(
            PROCESS_QUERY_SQL.format(source="process_rows"), (start, end)
        ).fetchall()
    )
    native = _normalize_query_rows(
        duck_connection.execute(
            PROCESS_QUERY_SQL.format(source="process_rows"), [start, end]
        ).fetchall()
    )
    parquet = _normalize_query_rows(
        duck_connection.execute(
            PROCESS_QUERY_SQL.format(source="read_parquet(?)"),
            [str(parquet_path), start, end],
        ).fetchall()
    )
    return {
        "contract_preserved": _query_rows_equivalent(reference, native)
        and _query_rows_equivalent(reference, parquet),
        "range": {"start": start, "end": end, "inclusive_text_between": True},
        "group_key": "(pid, process_name bytes)",
        "native_duckdb_equivalent": _query_rows_equivalent(reference, native),
        "parquet_equivalent": _query_rows_equivalent(reference, parquet),
        "group_count": len(reference),
        "latest_timestamp_preserved_as_original_text": True,
        "float_tolerance": FLOAT_TOLERANCE,
    }


def _ambient_probe(
    sqlite_path: Path, duck_connection: Any, parquet_path: Path
) -> dict[str, Any]:
    cases = [
        "1969-12-31T23:59:59.999Z",
        "2026-01-01T01:00:00.123+01:00",
        "2026-01-01T00:00:00.123456Z",
        "2026-01-01T00:00:00.123500Z",
        "2026-01-01T00:00:00.999500Z",
        "1969-12-31T23:59:59.999500Z",
        "2026-01-01 00:00:00.001",
        "not-a-time",
    ]
    connection = sqlite3.connect(sqlite_path)
    connection.execute("CREATE TABLE ambient_probe (id INTEGER, timestamp TEXT)")
    connection.executemany(
        "INSERT INTO ambient_probe VALUES (?, ?)", enumerate(cases, start=1)
    )
    reference = connection.execute(
        f"SELECT id, timestamp, {AMBIENT_EPOCH_MS_SQL} FROM ambient_probe ORDER BY id"
    ).fetchall()
    connection.commit()
    connection.close()
    duck_connection.execute("CREATE TABLE ambient_probe (id BIGINT, timestamp VARCHAR)")
    duck_connection.executemany(
        "INSERT INTO ambient_probe VALUES (?, ?)", enumerate(cases, start=1)
    )
    duck_connection.execute(
        f"COPY ambient_probe TO '{parquet_path.as_posix()}' (FORMAT PARQUET)"
    )
    native = duck_connection.execute(
        """
        SELECT id, timestamp,
               epoch_ms(TRY_CAST(timestamp AS TIMESTAMPTZ)),
               epoch_ms(TRY_CAST(
                 CASE
                   WHEN regexp_matches(timestamp, '(Z|[+-][0-9]{2}:[0-9]{2})$')
                     THEN timestamp
                   ELSE timestamp || 'Z'
                 END AS TIMESTAMPTZ
               ))
        FROM ambient_probe ORDER BY id
        """
    ).fetchall()
    parquet = duck_connection.execute(
        """
        SELECT id, timestamp,
               epoch_ms(TRY_CAST(timestamp AS TIMESTAMPTZ)),
               epoch_ms(TRY_CAST(
                 CASE
                   WHEN regexp_matches(timestamp, '(Z|[+-][0-9]{2}:[0-9]{2})$')
                     THEN timestamp
                   ELSE timestamp || 'Z'
                 END AS TIMESTAMPTZ
               ))
        FROM read_parquet(?) ORDER BY id
        """,
        [str(parquet_path)],
    ).fetchall()
    comparisons = []
    for sqlite_row, native_row, parquet_row in zip(
        reference, native, parquet, strict=True
    ):
        comparisons.append(
            {
                "id": sqlite_row[0],
                "timestamp": sqlite_row[1],
                "sqlite_epoch_ms": sqlite_row[2],
                "duckdb_epoch_ms": native_row[2],
                "parquet_epoch_ms": parquet_row[2],
                "duckdb_explicit_utc_epoch_ms": native_row[3],
                "parquet_explicit_utc_epoch_ms": parquet_row[3],
                "original_text_preserved": sqlite_row[1]
                == native_row[1]
                == parquet_row[1],
                "membership_value_equal": sqlite_row[2]
                == native_row[2]
                == parquet_row[2],
                "explicit_utc_membership_value_equal": sqlite_row[2]
                == native_row[3]
                == parquet_row[3],
            }
        )
    return {
        "contract_preserved": all(
            item["original_text_preserved"]
            and item["explicit_utc_membership_value_equal"]
            for item in comparisons
        ),
        "comparisons": comparisons,
        "raw_timestamp_preserved": all(
            item["original_text_preserved"] for item in comparisons
        ),
        "explicit_utc_epoch_semantics_preserved": all(
            item["explicit_utc_membership_value_equal"] for item in comparisons
        ),
        "invalid_timestamp_remains_null_membership": comparisons[-1][
            "sqlite_epoch_ms"
        ]
        is None,
        "candidate_parsing_is_diagnostic_only": True,
        "utc_normalization_is_not_sufficient_for_rounding_parity": any(
            item["original_text_preserved"]
            and not item["explicit_utc_membership_value_equal"]
            for item in comparisons
        ),
        "direct_parse_is_session_timezone_sensitive": any(
            item["duckdb_epoch_ms"] != item["duckdb_explicit_utc_epoch_ms"]
            or item["parquet_epoch_ms"]
            != item["parquet_explicit_utc_epoch_ms"]
            for item in comparisons
        ),
    }


def _cancellation_probe(
    sqlite_path: Path, duck_connection: Any, parquet_path: Path
) -> dict[str, Any]:
    values = [2**63 - 3, 2**63 - 4, -(2**63) + 2052, -(2**63) + 2053]
    sqlite_connection = sqlite3.connect(sqlite_path)
    sqlite_connection.execute("CREATE TABLE cancellation_probe (value INTEGER)")
    sqlite_connection.executemany(
        "INSERT INTO cancellation_probe VALUES (?)", [(value,) for value in values]
    )
    oracle = sqlite_connection.execute(
        "SELECT AVG(value) FROM cancellation_probe"
    ).fetchone()[0]
    sqlite_connection.commit()
    sqlite_connection.close()
    duck_connection.execute("CREATE TABLE cancellation_probe (value BIGINT)")
    duck_connection.executemany(
        "INSERT INTO cancellation_probe VALUES (?)", [(value,) for value in values]
    )
    duck_connection.execute(
        f"COPY cancellation_probe TO '{parquet_path.as_posix()}' (FORMAT PARQUET)"
    )
    native_average = duck_connection.execute(
        "SELECT AVG(value) FROM cancellation_probe"
    ).fetchone()[0]
    parquet_average = duck_connection.execute(
        "SELECT AVG(value) FROM read_parquet(?)", [str(parquet_path)]
    ).fetchone()[0]
    native_decimal = duck_connection.execute(
        "SELECT AVG(CAST(value AS DECIMAL(38, 0))) FROM cancellation_probe"
    ).fetchone()[0]
    parquet_decimal = duck_connection.execute(
        "SELECT AVG(CAST(value AS DECIMAL(38, 0))) FROM read_parquet(?)",
        [str(parquet_path)],
    ).fetchone()[0]
    return {
        "contract_preserved": _float_equal(oracle, native_average)
        and _float_equal(oracle, parquet_average),
        "input_decimal_strings": [str(value) for value in values],
        "sqlite_oracle_average": oracle,
        "native_duckdb_average": native_average,
        "parquet_duckdb_average": parquet_average,
        "native_allowed_error": _allowed_error(oracle),
        "native_absolute_error": abs(native_average - oracle),
        "explicit_decimal_diagnostic": {
            "native": str(native_decimal),
            "parquet": str(parquet_decimal),
            "accepted_design": False,
        },
        "float_tolerance": FLOAT_TOLERANCE,
    }


def run_contracts(output: Path) -> dict[str, Any]:
    """Run bounded synthetic probes and retain their artifacts under output."""
    if sqlite3.sqlite_version != REQUIRED_SQLITE_VERSION:
        raise RuntimeError(
            f"SQLite {REQUIRED_SQLITE_VERSION} is required, "
            f"found {sqlite3.sqlite_version}"
        )
    output = output.expanduser().resolve()
    output.mkdir(parents=True, exist_ok=True)
    run_directory = output / f"contracts-{uuid.uuid4().hex}"
    run_directory.mkdir()
    sqlite_path = run_directory / "oracle.sqlite3"
    duckdb_path = run_directory / "candidate.duckdb"
    canonical_parquet = run_directory / "canonical.parquet"
    affinity_parquet = run_directory / "affinity-exact-subset.parquet"
    ambient_sqlite_path = run_directory / "ambient-oracle.sqlite3"
    ambient_parquet = run_directory / "ambient.parquet"
    cancellation_sqlite_path = run_directory / "cancellation-oracle.sqlite3"
    cancellation_parquet = run_directory / "cancellation.parquet"

    sqlite_connection = _create_sqlite_fixture(sqlite_path)
    try:
        if duckdb is None:
            unavailable = {
                "status": "unavailable",
                "contract_preserved": False,
                "error": f"duckdb import failed: {DUCKDB_IMPORT_ERROR}",
            }
            probes = {
                name: dict(unavailable)
                for name in (
                    "typed_roundtrip",
                    "sqlite_affinity_and_invalid_utf8",
                    "process_query",
                    "ambient_timestamp",
                    "cancellation",
                )
            }
            duckdb_version = None
        else:
            source_rows = sqlite_connection.execute(
                "SELECT * FROM process_rows ORDER BY id"
            ).fetchall()
            duck_connection = _create_duckdb_fixture(duckdb_path, source_rows)
            duck_connection.close()
            duck_connection = duckdb.connect(str(duckdb_path))
            try:
                probes = {
                    "typed_roundtrip": _run_probe(
                        "typed_roundtrip",
                        lambda: _roundtrip_probe(
                            sqlite_connection, duck_connection, canonical_parquet
                        ),
                    ),
                    "sqlite_affinity_and_invalid_utf8": _run_probe(
                        "sqlite_affinity_and_invalid_utf8",
                        lambda: _affinity_probe(
                            sqlite_connection, duck_connection, affinity_parquet
                        ),
                    ),
                    "process_query": _run_probe(
                        "process_query",
                        lambda: _process_query_probe(
                            sqlite_connection, duck_connection, canonical_parquet
                        ),
                    ),
                    "ambient_timestamp": _run_probe(
                        "ambient_timestamp",
                        lambda: _ambient_probe(
                            ambient_sqlite_path, duck_connection, ambient_parquet
                        ),
                    ),
                    "cancellation": _run_probe(
                        "cancellation",
                        lambda: _cancellation_probe(
                            cancellation_sqlite_path,
                            duck_connection,
                            cancellation_parquet,
                        ),
                    ),
                }
                duckdb_version = duck_connection.execute(
                    "SELECT version()"
                ).fetchone()[0]
            finally:
                duck_connection.close()
    finally:
        sqlite_connection.close()

    completed = all(probe["status"] != "unavailable" for probe in probes.values())
    return {
        "schema_version": 1,
        "completed": completed,
        "production_format_accepted": False,
        "acceptance_note": (
            "Focused correctness probes cannot approve production storage; "
            "lifecycle, performance, resource, recovery, and bundle gates remain."
        ),
        "identity_contract": {
            "process": "(pid, process_name bytes); PID alone is not identity",
            "ambient": "original source and timestamp text remain authoritative",
        },
        "versions": {
            "sqlite": sqlite3.sqlite_version,
            "sqlite_binding": "pysqlite3 0.5.4; same engine version as Core, "
            "different binding/build",
            "duckdb": duckdb_version,
        },
        "artifacts": {
            "directory": str(run_directory),
            "sqlite_oracles": [
                str(sqlite_path),
                str(ambient_sqlite_path),
                str(cancellation_sqlite_path),
            ],
            "duckdb": str(duckdb_path),
            "parquet": [
                str(canonical_parquet),
                str(affinity_parquet),
                str(ambient_parquet),
                str(cancellation_parquet),
            ],
        },
        "probes": probes,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    report = run_contracts(arguments.output)
    rendered = json.dumps(report, indent=2, sort_keys=True, ensure_ascii=False)
    report_path = arguments.output.expanduser().resolve() / "engine-contracts.json"
    report_path.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
