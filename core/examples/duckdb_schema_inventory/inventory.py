#!/usr/bin/env python3
"""Rebuild the current App SQLite schema and inventory its code owners.

This is an investigation tool. It creates only a synthetic temporary SQLite
file, uses the real App migration list and Core SQLx migrator, and never opens
an application database.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
MANUAL_ANNOTATIONS_REVIEWED_AT_HEAD = "b8cf82dd0f9b770730e050a3e654df6e2d69adf9"
REVIEWED_PRODUCTION_SOURCE_SHA256 = {
    'core/src/infrastructure/database/ambient_archive.rs': '5d43ab5923d1df556d99c3acaaffb45d090babd4137d2a5535cb2353a798aa2f',
    'core/src/infrastructure/database/archive_queries.rs': '29513376ecfa7f1b4eec6dd1aca656b86099da1343d7be4b093e5006572b53b1',
    'core/src/infrastructure/database/cooling_baseline.rs': 'b940a9283fb58fca255bc57a6c3d089f7f478b24f9f7afcd611a0e28a11d908d',
    'core/src/infrastructure/database/cooling_covariate_daily_summary.rs': 'c43688910012f4a0f83b28e71bf71a8c0b7de781c64bf8b9210230710ff8d518',
    'core/src/infrastructure/database/cooling_daily_summary.rs': 'dbbb4f0eba2355e15b7e5d767a62bcf1ed84cf466686009aaa426b68495db1e8',
    'core/src/infrastructure/database/cooling_delta_baseline.rs': 'e5358fbe9c416bc8096e70c23a7220abbdcdb376d0583ea49b698a6dbb8ef316',
    'core/src/infrastructure/database/cooling_fan_daily_summary.rs': '42edbda173accd6e584d5e8707953d6a859ce471a132e61bc8a3a0e1e1f44893',
    'core/src/infrastructure/database/cooling_hourly_summary.rs': 'b3d733bf12d21f430ad68cf9e44509f000d1ddd5c8aba469a9c1ba4011b0b989',
    'core/src/infrastructure/database/cooling_thermal_delta_daily_summary.rs': 'adaa519b17877f7f806b37e7c81521e78399072013b2b86987dbb4f1a166fec7',
    'core/src/infrastructure/database/fan_archive.rs': '831e843de23920e54903686f173e9a86053166522f1906997a2ac5aaac2ae9e6',
    'core/src/infrastructure/database/gpu_archive.rs': '2f9efeff2d1af5368384a49ab671524158d01bd36c62175914d5d17ee9eb3c00',
    'core/src/infrastructure/database/hardware_archive.rs': '42768dabb93438a59e42925fda7ad23b0e50745efe01e9046fda7f8a9922a81a',
    'core/src/infrastructure/database/migrate.rs': '43814c135c78b0677728b7960b0522bde0bf2e113eac098bc4d73bd04c436ce4',
    'core/src/infrastructure/database/process_stats.rs': 'e08ff3d3e59a1f35a58f737759b3e204958c29cc7b7d7a1310cee62900806c14',
    'core/src/infrastructure/database/storage_health.rs': '55d06788fea90f44bd31896ee38035cd274b949127649013e4cea2cd8533db0b',
    'core/src/persistence/archive.rs': '86d4559d25124ef4adfc1281e3098471a7ba171ed6220657db7d97b0d616fa44',
    'core/src/persistence/cooling_band_comparison.rs': '27d15cacbb6042d940b0e3a07371a0fd36b28b36e821d86d937de6063aedbe2c',
    'core/src/persistence/cooling_baseline.rs': '232226987a1f5e38886f531eea0a1e911b13b70fbd67f979e93d45a658c89f00',
    'core/src/persistence/cooling_baseline_delta.rs': 'c9db68790a982ec21e332612d15c3f90ba56988a806d390ad99aacc77a74469e',
    'core/src/persistence/cooling_covariate_comparison.rs': '269036c86056f9ef4f88f6d06cffd1752d8e1393461ad138b1804a94ca750f2d',
    'core/src/persistence/cooling_covariate_rollup.rs': '57c66295adf2bf7f166ec7c0d68cfc78f9344883666057f948c96fb379de4645',
    'core/src/persistence/cooling_delta_baseline.rs': 'adb50fbed382a79c80ecb7cf3a1bb46bdeb5bb3cb165c31f147aa010c6af1598',
    'core/src/persistence/cooling_fan_rollup.rs': 'deff656b6f9715ae438714c09770ff906fc13c130591c24a241189059cde304d',
    'core/src/persistence/cooling_fan_trend.rs': '690c838d8a754f8dee67537fa07e184f41410ea0956e34483613aa1a16baf399',
    'core/src/persistence/cooling_load_temperature_explorer.rs': '2789a0983f1dc87f2da3f550c829fc014a54b43f7ee7354d10379606de58125a',
    'core/src/persistence/cooling_rollup.rs': 'a49b3a315c7de48c7dc86e973f68eb3df8496389fbd3d97d372b87270245cc34',
    'core/src/persistence/cooling_trend.rs': 'd18a50ab61fad15d24516acdd33b683afbd43ed352764eac9c543552cb9c66b4',
    'core/src/persistence/preflight.rs': '15c1ab4c5c8c9daec049914ca4862d79c1c01f311f60604db99c1181b5767fc9',
    'core/src/persistence/storage_health.rs': '55335b271e356d71870914e620530c9df26216696814750e2ba95c22f7c64fb3',
    'src-tauri/src/infrastructure/database/migration.rs': '18f0b5ddcb2ddff215ca31b2e639b8fb7d3b3fd84e8a80dbf0d6267e14fdd78b',
    'src-tauri/src/services/hardware_service.rs': '6d3c9ccbe7e1674fbdd903ddefcf5020036169ded6c9fcb06f1039b56cdd7c76',
}

TABLE_CONTRACTS: dict[str, dict[str, Any]] = {
    "DATA_ARCHIVE": {
        "domain": "Hardware Archive minute system/CPU/power observations",
        "write_owners": ["core/src/infrastructure/database/hardware_archive.rs", "core/src/persistence/archive.rs"],
        "read_query_consumers": ["core/src/infrastructure/database/archive_queries.rs"],
        "rollup_consumers": [
            "core/src/infrastructure/database/cooling_daily_summary.rs",
            "core/src/infrastructure/database/cooling_thermal_delta_daily_summary.rs",
            "core/src/infrastructure/database/cooling_covariate_daily_summary.rs",
            "core/src/persistence/cooling_rollup.rs",
        ],
        "mutation": "append minute row; scheduled retention delete",
        "lifetime": "raw archive; user retentionDays and scheduled deletion",
        "direct_copy_required": True,
        "copy_reason": "Raw source facts and row IDs must survive; expired rows cannot be reconstructed.",
        "identity": "INTEGER PRIMARY KEY rowid; domain queries identify samples by stored id/timestamp, not a portable hardware entity key",
        "dialect_concerns": ["SQLite DATETIME affinity and original timestamp representation", "INTEGER PRIMARY KEY rowid allocation/high-water semantics", "SQLite numeric affinity and mixed storage classes"],
    },
    "GPU_DATA_ARCHIVE": {
        "domain": "Hardware Archive minute GPU observations",
        "write_owners": ["core/src/infrastructure/database/gpu_archive.rs", "core/src/persistence/archive.rs"],
        "read_query_consumers": ["core/src/infrastructure/database/archive_queries.rs"],
        "rollup_consumers": [],
        "mutation": "append per-GPU minute rows; scheduled retention delete",
        "lifetime": "raw archive; user retentionDays and scheduled deletion",
        "direct_copy_required": True,
        "copy_reason": "Raw source facts, opaque gpu_id/name attribution, multiplicity, and row IDs are not derivable after expiry.",
        "identity": "gpu_id is an opaque archived producer value; gpu_name remains separately queried; row id is INTEGER PRIMARY KEY",
        "dialect_concerns": ["SQLite DATETIME affinity", "nullable legacy gpu_id", "INTEGER PRIMARY KEY rowid allocation", "numeric storage-class preservation"],
    },
    "PROCESS_STATS": {
        "domain": "Hardware Archive ranked process observations",
        "write_owners": ["core/src/infrastructure/database/process_stats.rs", "core/src/persistence/archive.rs"],
        "read_query_consumers": ["core/src/infrastructure/database/archive_queries.rs"],
        "rollup_consumers": [],
        "mutation": "batched append for each archive cycle; scheduled retention delete",
        "lifetime": "raw archive; user retentionDays and scheduled deletion",
        "direct_copy_required": True,
        "copy_reason": "Multiplicity, exact cells, AUTOINCREMENT id state, and recorded (pid, process_name) tuples cannot be regenerated.",
        "identity": "query grouping identity is recorded (pid, process_name); id is AUTOINCREMENT storage identity",
        "dialect_concerns": ["AUTOINCREMENT sqlite_sequence import", "SQLite AVG/numeric-class behavior", "TEXT byte preservation", "inclusive timestamp range semantics"],
    },
    "AMBIENT_ARCHIVE": {
        "domain": "one-minute ambient observations per Sensor Source Label",
        "write_owners": ["core/src/infrastructure/database/ambient_archive.rs", "core/src/persistence/archive.rs"],
        "read_query_consumers": ["core/src/infrastructure/database/archive_queries.rs"],
        "rollup_consumers": [
            "core/src/infrastructure/database/cooling_thermal_delta_daily_summary.rs",
            "core/src/infrastructure/database/cooling_covariate_daily_summary.rs",
            "core/src/persistence/cooling_rollup.rs",
        ],
        "mutation": "transactional multi-source append per minute; scheduled retention delete",
        "lifetime": "raw archive; user retentionDays and scheduled deletion",
        "direct_copy_required": True,
        "copy_reason": "Source-labelled rows and missing/null humidity cannot be recreated after expiry.",
        "identity": "source is the Sensor Source Label; id is AUTOINCREMENT storage identity",
        "dialect_concerns": ["AUTOINCREMENT sqlite_sequence import", "SQLite DATETIME parsing/epoch predicate behavior", "source TEXT bytes and collation", "nullable humidity"],
    },
    "FAN_ARCHIVE": {
        "domain": "one-minute fan observations per fan source",
        "write_owners": ["core/src/infrastructure/database/fan_archive.rs", "core/src/persistence/archive.rs"],
        "read_query_consumers": ["core/src/infrastructure/database/archive_queries.rs", "core/src/persistence/cooling_fan_trend.rs"],
        "rollup_consumers": ["core/src/persistence/cooling_fan_rollup.rs", "core/src/persistence/cooling_covariate_rollup.rs", "core/src/persistence/cooling_rollup.rs"],
        "mutation": "transactional multi-fan append per minute; scheduled retention delete",
        "lifetime": "raw archive; user retentionDays and scheduled deletion",
        "direct_copy_required": True,
        "copy_reason": "Per-fan source, real zero RPM, multiplicity, and IDs cannot be reconstructed.",
        "identity": "source identifies the reported fan; id is INTEGER PRIMARY KEY rowid without AUTOINCREMENT",
        "dialect_concerns": ["INTEGER PRIMARY KEY rowid allocation without sqlite_sequence", "SQLite timestamp predicate", "zero RPM must remain distinct from missing row"],
    },
    "storage_devices": {
        "domain": "mutable Storage Health device identity/current activity",
        "write_owners": ["core/src/infrastructure/database/storage_health.rs", "core/src/persistence/storage_health.rs"],
        "read_query_consumers": ["core/src/infrastructure/database/storage_health.rs", "src-tauri/src/services/hardware_service.rs"],
        "rollup_consumers": [],
        "mutation": "upsert device attributes and update is_active in same refresh transaction as daily records",
        "lifetime": "independent long-lived mutable identity table",
        "direct_copy_required": True,
        "copy_reason": "Device identity, first_seen_at and serial_hash are independently persisted and cannot be replayed from retained daily rows.",
        "identity": "id is produced from the Storage Health identity contract; daily records join device_id to this exact TEXT key",
        "dialect_concerns": ["TEXT primary-key uniqueness/collation", "COALESCE update semantics", "boolean encoded as INTEGER", "u64-to-i64 narrowing at writer boundary"],
    },
    "storage_health_daily_records": {
        "domain": "mutable-per-day Storage Health history",
        "write_owners": ["core/src/infrastructure/database/storage_health.rs", "core/src/persistence/storage_health.rs"],
        "read_query_consumers": ["core/src/infrastructure/database/storage_health.rs", "src-tauri/src/services/hardware_service.rs"],
        "rollup_consumers": [],
        "mutation": "upsert by (device_id,date); retention delete",
        "lifetime": "Storage Health retention, separate from Hardware Archive",
        "direct_copy_required": True,
        "copy_reason": "Daily rows are independent records; older source observations are not retained elsewhere.",
        "identity": "UNIQUE(device_id,date) with FK to storage_devices.id",
        "dialect_concerns": ["ON CONFLICT composite unique behavior", "foreign-key enforcement", "warning_reasons JSON stored as TEXT", "NOCASE presentation ordering in latest-record query"],
    },
    "cooling_daily_summary": {
        "domain": "daily absolute cooling/load/power summary",
        "write_owners": ["core/src/infrastructure/database/cooling_daily_summary.rs", "core/src/persistence/cooling_rollup.rs"],
        "read_query_consumers": ["core/src/persistence/cooling_trend.rs", "core/src/persistence/cooling_band_comparison.rs", "core/src/persistence/cooling_load_temperature_explorer.rs", "core/src/persistence/cooling_baseline.rs"],
        "rollup_consumers": [],
        "mutation": "upsert by date in atomic daily-rollup transaction; retention delete with pinned-window exemption",
        "lifetime": "long-lived Cooling summary; independent fixed retention",
        "direct_copy_required": True,
        "copy_reason": "Summary days can outlive raw minutes and therefore cannot be regenerated after conversion.",
        "identity": "date TEXT primary key",
        "dialect_concerns": ["ON CONFLICT update semantics", "nullable metric columns versus zero counts", "TEXT date ordering", "retention exclusions"],
    },
    "cooling_hourly_summary": {
        "domain": "hourly Cooling timeline summary",
        "write_owners": ["core/src/infrastructure/database/cooling_hourly_summary.rs", "core/src/persistence/cooling_rollup.rs"],
        "read_query_consumers": ["core/src/persistence/cooling_trend.rs", "core/src/persistence/cooling_load_temperature_explorer.rs"],
        "rollup_consumers": [],
        "mutation": "upsert by hour_start in atomic daily-rollup transaction; retention delete with pinned-window exemption",
        "lifetime": "long-lived Cooling summary; same fixed retention policy as daily absolute summary",
        "direct_copy_required": True,
        "copy_reason": "Hourly projections may outlive raw minutes and are part of user-visible history.",
        "identity": "hour_start TEXT primary key",
        "dialect_concerns": ["TEXT time ordering", "ON CONFLICT update semantics", "pinned baseline retention window"],
    },
    "cooling_fan_daily_summary": {
        "domain": "daily Cooling fan summary",
        "write_owners": ["core/src/infrastructure/database/cooling_fan_daily_summary.rs", "core/src/persistence/cooling_rollup.rs"],
        "read_query_consumers": ["core/src/persistence/cooling_fan_trend.rs"],
        "rollup_consumers": [],
        "mutation": "upsert by (date,source) in atomic daily-rollup transaction; retention delete",
        "lifetime": "long-lived Cooling summary; independent fixed retention",
        "direct_copy_required": True,
        "copy_reason": "Per-fan daily rows can outlive FAN_ARCHIVE.",
        "identity": "composite primary key (date,source)",
        "dialect_concerns": ["composite primary key conflict target", "TEXT source/date preservation", "zero RPM semantics"],
    },
    "cooling_thermal_delta_daily_summary": {
        "domain": "daily per-source Thermal Delta summary",
        "write_owners": ["core/src/infrastructure/database/cooling_thermal_delta_daily_summary.rs", "core/src/persistence/cooling_rollup.rs"],
        "read_query_consumers": ["core/src/persistence/cooling_baseline_delta.rs", "core/src/persistence/cooling_covariate_comparison.rs", "core/src/persistence/cooling_band_comparison.rs"],
        "rollup_consumers": [],
        "mutation": "upsert by (date,source) in atomic daily-rollup transaction; retention delete with pinned delta-baseline exemption",
        "lifetime": "long-lived Cooling summary, outlives raw ambient/hardware rows",
        "direct_copy_required": True,
        "copy_reason": "Source-specific deltas cannot be regenerated when paired raw minutes expire.",
        "identity": "composite primary key (date,source); source must match baseline source",
        "dialect_concerns": ["nullable per-band values and defaulted counts", "composite-key upsert", "source attribution", "retention exemption"],
    },
    "cooling_covariate_daily_summary": {
        "domain": "daily per-source/load-band sufficient statistics",
        "write_owners": ["core/src/infrastructure/database/cooling_covariate_daily_summary.rs", "core/src/persistence/cooling_rollup.rs"],
        "read_query_consumers": ["core/src/persistence/cooling_covariate_comparison.rs"],
        "rollup_consumers": [],
        "mutation": "upsert by (date,source,band) in atomic daily-rollup transaction; retention delete with pinned delta-baseline exemption",
        "lifetime": "long-lived Cooling covariate summary",
        "direct_copy_required": True,
        "copy_reason": "Sufficient statistics outlive their paired raw minutes and are not safely reconstructible later.",
        "identity": "composite primary key (date,source,band)",
        "dialect_concerns": ["binary64 aggregate/sufficient-statistic preservation", "composite-key upsert", "zero count as absence flag", "retention exemption"],
    },
    "cooling_fan_covariate_daily_summary": {
        "domain": "daily per-source/fan/load-band sufficient statistics",
        "write_owners": ["core/src/infrastructure/database/cooling_covariate_daily_summary.rs", "core/src/persistence/cooling_rollup.rs"],
        "read_query_consumers": ["core/src/persistence/cooling_covariate_comparison.rs"],
        "rollup_consumers": [],
        "mutation": "upsert by (date,source,fan_source,band) in atomic daily-rollup transaction; retention delete with pinned delta-baseline exemption",
        "lifetime": "long-lived Cooling fan covariate summary",
        "direct_copy_required": True,
        "copy_reason": "Sufficient statistics outlive raw fan/ambient/hardware rows.",
        "identity": "composite primary key (date,source,fan_source,band)",
        "dialect_concerns": ["binary64 sums", "four-column conflict target", "source/fan attribution", "retention exemption"],
    },
    "cooling_baseline": {
        "domain": "write-once absolute Cooling baseline",
        "write_owners": ["core/src/infrastructure/database/cooling_baseline.rs", "core/src/persistence/cooling_baseline.rs"],
        "read_query_consumers": ["core/src/persistence/cooling_band_comparison.rs", "core/src/persistence/cooling_load_temperature_explorer.rs", "core/src/persistence/cooling_rollup.rs"],
        "rollup_consumers": [],
        "mutation": "INSERT OR IGNORE singleton; intentionally not updated or retained away",
        "lifetime": "independent pinned baseline",
        "direct_copy_required": True,
        "copy_reason": "Pinned user reference is write-once and cannot be recomputed from expired source days.",
        "identity": "singleton id=1 enforced by CHECK",
        "dialect_concerns": ["CHECK(id = 1)", "INSERT OR IGNORE semantics", "write-once preservation", "binary64 value"],
    },
    "cooling_delta_baseline": {
        "domain": "write-once source-specific Thermal Delta baseline",
        "write_owners": ["core/src/infrastructure/database/cooling_delta_baseline.rs", "core/src/persistence/cooling_delta_baseline.rs"],
        "read_query_consumers": ["core/src/persistence/cooling_baseline_delta.rs", "core/src/persistence/cooling_covariate_comparison.rs", "core/src/persistence/cooling_band_comparison.rs", "core/src/persistence/cooling_rollup.rs"],
        "rollup_consumers": [],
        "mutation": "INSERT OR IGNORE singleton; intentionally not updated",
        "lifetime": "independent pinned source-specific baseline",
        "direct_copy_required": True,
        "copy_reason": "Pinned reference and selected source cannot be recreated after source rows expire.",
        "identity": "singleton id=1 plus exact Sensor Source Label",
        "dialect_concerns": ["CHECK(id = 1)", "INSERT OR IGNORE", "source identity", "binary64 value"],
    },
    "_sqlx_migrations": {
        "domain": "SQLite schema migration history/checksums",
        "write_owners": ["core/src/infrastructure/database/migrate.rs", "src-tauri/src/infrastructure/database/migration.rs"],
        "read_query_consumers": ["core/src/persistence/preflight.rs"],
        "rollup_consumers": [],
        "mutation": "SQLx appends migration result rows",
        "lifetime": "database metadata",
        "direct_copy_required": False,
        "copy_reason": "Do not replay SQLite migration metadata as DuckDB authority; retain it as source-validation evidence and create engine-specific native migration metadata.",
        "identity": "version primary key plus checksum of exact immutable SQL",
        "dialect_concerns": ["SQLx migration table shape/checksum", "SQLite ReversibleUp history is not native DuckDB DDL", "preflight max successful version contract"],
    },
    "sqlite_sequence": {
        "domain": "SQLite AUTOINCREMENT high-water metadata",
        "write_owners": ["SQLite engine via PROCESS_STATS and AMBIENT_ARCHIVE AUTOINCREMENT"],
        "read_query_consumers": [],
        "rollup_consumers": [],
        "mutation": "SQLite updates on AUTOINCREMENT inserts",
        "lifetime": "database identity metadata",
        "direct_copy_required": False,
        "copy_reason": "Translate/import per-table next-ID state into the candidate allocation design; an empty synthetic sequence table does not prove production high-water is empty.",
        "identity": "high-water for PROCESS_STATS and AMBIENT_ARCHIVE",
        "dialect_concerns": ["DuckDB sequence/identity semantics differ", "deleted highest IDs must not be reused for AUTOINCREMENT tables", "production stored state remains unmeasured"],
    },
}

TRANSACTION_BOUNDARIES = [
    {
        "name": "storage_health_refresh",
        "tables": ["storage_devices", "storage_health_daily_records"],
        "source": "core/src/infrastructure/database/storage_health.rs",
        "contract": "device upserts, daily-record upserts, and active-device updates commit together",
    },
    {
        "name": "daily_cooling_rollup",
        "tables": ["cooling_daily_summary", "cooling_hourly_summary", "cooling_fan_daily_summary", "cooling_thermal_delta_daily_summary", "cooling_covariate_daily_summary", "cooling_fan_covariate_daily_summary"],
        "source": "core/src/persistence/cooling_rollup.rs",
        "contract": "all projections for one completed day commit or retry together",
    },
    {
        "name": "ambient_cycle_rows",
        "tables": ["AMBIENT_ARCHIVE"],
        "source": "core/src/infrastructure/database/ambient_archive.rs",
        "contract": "all accepted ambient sources for one archive cycle commit together",
    },
    {
        "name": "fan_cycle_rows",
        "tables": ["FAN_ARCHIVE"],
        "source": "core/src/infrastructure/database/fan_archive.rs",
        "contract": "all accepted fans for one archive cycle commit together",
    },
    {
        "name": "process_cycle_rows",
        "tables": ["PROCESS_STATS"],
        "source": "core/src/infrastructure/database/process_stats.rs",
        "contract": "all ranked process rows for one archive cycle commit together",
    },
]


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def reviewed_source_hashes(repo: Path) -> dict[str, str | None]:
    return {
        path: hashlib.sha256(source.read_bytes()).hexdigest() if source.is_file() else None
        for path in REVIEWED_PRODUCTION_SOURCE_SHA256
        for source in [repo / path]
    }


def annotation_freshness(actual_hashes: dict[str, str | None]) -> dict[str, Any]:
    mismatches = [
        {
            "path": path,
            "reviewed_sha256": reviewed_hash,
            "current_sha256": actual_hashes.get(path),
        }
        for path, reviewed_hash in REVIEWED_PRODUCTION_SOURCE_SHA256.items()
        if actual_hashes.get(path) != reviewed_hash
    ]
    return {
        "current": not mismatches,
        "reviewed_source_sha256": REVIEWED_PRODUCTION_SOURCE_SHA256,
        "current_source_sha256": actual_hashes,
        "mismatches": mismatches,
    }


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=repo, text=True, check=True, capture_output=True).stdout.strip()


def line_kind(lines: list[str], index: int, path: str) -> str:
    if path.endswith("migration.rs"):
        return "migration_definition_or_migration_test"
    test_start = next((i for i, line in enumerate(lines) if "#[cfg(test)]" in line), len(lines))
    if index >= test_start:
        return "test_reference"
    stripped = lines[index].lstrip()
    if stripped.startswith("//") or stripped.startswith("///") or stripped.startswith("//!"):
        return "comment_or_doc_reference"
    window = " ".join(lines[max(0, index - 5) : min(len(lines), index + 6)]).upper()
    writes = any(token in window for token in ("INSERT INTO", "UPDATE ", "DELETE FROM"))
    reads = any(token in window for token in ("SELECT ", " FROM ", " JOIN "))
    if writes and reads:
        return "heuristic_read_write_sql_context"
    if writes:
        return "heuristic_write_sql_context"
    if reads:
        return "heuristic_read_sql_context"
    return "unclassified_lexical_reference"


def lexical_hits(repo: Path, object_names: list[str]) -> dict[str, list[dict[str, Any]]]:
    result: dict[str, list[dict[str, Any]]] = {name: [] for name in object_names}
    files = sorted((repo / "core/src").rglob("*.rs")) + sorted((repo / "src-tauri/src").rglob("*.rs"))
    patterns = {name: re.compile(rf"(?<![A-Za-z0-9_]){re.escape(name)}(?![A-Za-z0-9_])", re.IGNORECASE) for name in object_names}
    for file in files:
        relative = file.relative_to(repo).as_posix()
        lines = file.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines):
            for name, pattern in patterns.items():
                if pattern.search(line):
                    result[name].append({
                        "path": relative,
                        "line": index + 1,
                        "kind": line_kind(lines, index, relative),
                        "text": line.strip()[:500],
                    })
    return result


def load_issue_1666(issue_path: Path) -> dict[str, Any]:
    issue = json.loads(issue_path.read_text())
    body = issue.get("body") or ""
    selected = []
    for number, line in enumerate(body.splitlines(), 1):
        lower = line.lower()
        if any(term in lower for term in ("baseline", "7 to 14 days", "moving average", "similar usage", "fan speeds", "cooling performance")):
            selected.append({"line": number, "text": line})
    return {
        "number": issue["number"],
        "title": issue["title"],
        "url": issue["html_url"],
        "state": issue["state"],
        "updated_at": issue["updated_at"],
        "body_sha256": sha256_text(body),
        "selected_scope_lines": selected,
        "scope_interpretation": "The live parent Issue remains a product-level Cooling Insight outcome. Current table/rollup details are owned by merged code and migrations and must be refreshed before conversion.",
    }



def object_decision(object_: dict[str, Any]) -> dict[str, Any]:
    kind = object_["type"]
    name = object_["name"]
    if kind == "table":
        contract = TABLE_CONTRACTS.get(name)
        return {
            "direct_copy_required": contract["direct_copy_required"] if contract else None,
            "candidate_action": "copy stored rows exactly" if contract and contract["direct_copy_required"] else "translate metadata into engine-specific state",
            "basis": contract["copy_reason"] if contract else "No verified table contract recorded.",
        }
    if kind == "index":
        automatic = name.startswith("sqlite_autoindex_")
        return {
            "direct_copy_required": False,
            "candidate_action": "recreate through native PRIMARY KEY/UNIQUE constraint" if automatic else "recreate and validate a native index against owning queries",
            "basis": "Index rows are derived structures; preserve uniqueness and query behavior rather than copying SQLite B-tree pages.",
            "definition_owner": "src-tauri/src/infrastructure/database/migration.rs",
        }
    if kind in {"trigger", "view"}:
        return {
            "direct_copy_required": False,
            "candidate_action": "translate SQL and verify native semantics before adoption",
            "basis": "Executable schema SQL cannot be copied as stored data.",
        }
    return {"direct_copy_required": None, "candidate_action": "unclassified", "basis": "Unexpected runtime object kind."}

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--issue-1666-json", type=Path, required=True)
    parser.add_argument("--schema-snapshot", type=Path)
    parser.add_argument("--cargo-target-dir", type=Path)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[3])
    args = parser.parse_args()
    repo = args.repo.resolve()
    output = args.output.resolve()

    with tempfile.TemporaryDirectory(prefix="hardviz-schema-inventory-") as temporary:
        temporary_path = Path(temporary)
        schema_path = args.schema_snapshot or temporary_path / "schema.json"
        if args.schema_snapshot is None:
            database = temporary_path / "synthetic.sqlite3"
            cargo_environment = None
            if args.cargo_target_dir:
                import os
                cargo_environment = {**os.environ, "CARGO_TARGET_DIR": str(args.cargo_target_dir.resolve())}
            subprocess.run(
                [
                    "cargo", "run", "-q", "-p", "hardviz-core", "--example", "duckdb_schema_inventory", "--",
                    "--database", str(database), "--output", str(schema_path),
                ],
                cwd=repo,
                check=True,
                env=cargo_environment,
            )
        schema = json.loads(schema_path.read_text())

    source_head = git(repo, "rev-parse", "HEAD")
    freshness = annotation_freshness(reviewed_source_hashes(repo))
    object_names = [object_["name"] for object_ in schema["schema_objects"]]
    hits = lexical_hits(repo, object_names)
    tables = []
    missing_contracts = []
    for runtime_table in schema["tables"]:
        name = runtime_table["name"]
        contract = TABLE_CONTRACTS.get(name)
        if contract is None:
            missing_contracts.append(name)
        tables.append({
            **runtime_table,
            "verified_contract": contract,
            "lexical_hits": hits.get(name, []),
            "lexical_hit_count": len(hits.get(name, [])),
        })

    total_lexical_hits = sum(len(object_hits) for object_hits in hits.values())
    unclassified_lexical_hits = sum(1 for object_hits in hits.values() for hit in object_hits if hit["kind"] == "unclassified_lexical_reference")
    runtime_types: dict[str, int] = {"table": 0, "index": 0, "trigger": 0, "view": 0}
    for object_ in schema["schema_objects"]:
        runtime_types[object_["type"]] = runtime_types.get(object_["type"], 0) + 1
    objects = [{**object_, "migration_decision": object_decision(object_)} for object_ in schema["schema_objects"]]
    objects_without_decision = [object_["name"] for object_ in objects if object_["migration_decision"]["direct_copy_required"] is None]
    missing_source_paths = []
    for table, contract in TABLE_CONTRACTS.items():
        for key in ("write_owners", "read_query_consumers", "rollup_consumers"):
            for source in contract[key]:
                if source.startswith("SQLite engine"):
                    continue
                if not (repo / source).is_file():
                    missing_source_paths.append({"table": table, "role": key, "path": source})
    for boundary in TRANSACTION_BOUNDARIES:
        if not (repo / boundary["source"]).is_file():
            missing_source_paths.append({"boundary": boundary["name"], "role": "transaction_source", "path": boundary["source"]})
    assertions = {
        "all_runtime_tables_have_verified_contract": not missing_contracts,
        "runtime_tables_without_verified_contract": missing_contracts,
        "all_declared_migrations_applied_successfully": schema["migration_count"] == len(schema["declared_migrations"]) and all(row["success"] for row in schema["applied_migrations"]),
        "migration_versions_contiguous": [row["version"] for row in schema["applied_migrations"]] == list(range(1, schema["max_successful_migration_version"] + 1)),
        "declared_and_applied_max_versions_match": schema["declared_max_migration_version"] == schema["max_successful_migration_version"],
        "all_runtime_objects_have_migration_decisions": not objects_without_decision,
        "runtime_objects_without_migration_decisions": objects_without_decision,
        "manual_annotations_match_reviewed_production_sources": freshness["current"],
        "all_verified_source_paths_exist": not missing_source_paths,
        "missing_verified_source_paths": missing_source_paths,
    }
    complete = all(value is True or value == [] for value in assertions.values())
    report = {
        "schema_version": SCHEMA_VERSION,
        "invocation": sys.argv,
        "completed": complete,
        "source": {
            "repository": "shm11C3/HardwareVisualizer",
            "head": source_head,
            "manual_annotations_reviewed_at_head": MANUAL_ANNOTATIONS_REVIEWED_AT_HEAD,
            "manual_annotation_freshness": "current" if freshness["current"] else "stale_requires_manual_review",
            "manual_annotation_policy": "The embedded owner/consumer/transaction annotations were reviewed at the provenance head. Freshness compares SHA-256 for the production source files supporting those annotations, so commits that only change this research probe do not invalidate them. Any reviewed production source mismatch fails completion until a maintainer reviews and repins its hash.",
            "manual_annotation_source_fingerprints": freshness,
            "branch": git(repo, "branch", "--show-current"),
            "migration_source": "src-tauri/src/infrastructure/database/migration.rs",
            "migration_runner": "core/src/infrastructure/database/migrate.rs::run_on_pool",
            "scanned_source_roots": ["core/src", "src-tauri/src"],
        },
        "scope_checkpoint_1666": load_issue_1666(args.issue_1666_json),
        "runtime_schema": {
            "sqlite_version": schema["sqlite_version"],
            "migration_count": schema["migration_count"],
            "declared_max_migration_version": schema["declared_max_migration_version"],
            "max_successful_migration_version": schema["max_successful_migration_version"],
            "declared_migrations": schema["declared_migrations"],
            "applied_migrations": schema["applied_migrations"],
            "object_counts": runtime_types,
            "schema_objects": objects,
            "tables": tables,
            "sqlite_sequence_rows_after_empty_migration": schema["sqlite_sequence"],
            "sqlite_sequence_interpretation": "The synthetic migrated database contains no user rows, so sqlite_sequence is empty. PROCESS_STATS and AMBIENT_ARCHIVE still require production high-water import/translation; no production database was measured.",
        },
        "verified_transaction_boundaries": TRANSACTION_BOUNDARIES,
        "cross_table_authority_findings": [
            "The daily Cooling projections are one real SQLite transaction and must stay in one authoritative transaction domain.",
            "Storage device mutations and daily records are one real refresh transaction and must not be split across independent engines without a new coordination protocol.",
            "Archive families share an application cycle timestamp but current DATA/GPU/PROCESS/AMBIENT/FAN writers use separate database calls/transactions; candidate snapshot semantics still require lifecycle qualification.",
            "Scheduled raw-archive cleanup intentionally handles table failures independently, and Cooling/Storage retention has separate policies; conversion must preserve those lifetimes and define partial-failure/retry behavior rather than assuming one global delete transaction.",
            "The two write-once baselines establish outside the six-table daily projection transaction; their pinned source/window and retention exemptions remain independently authoritative state.",
            "Summaries and baselines can outlive raw archives, so direct copy is required even where a rollup algorithm exists.",
            "SQLite migration history cannot become DuckDB migration authority by copying _sqlx_migrations; engine-specific DDL/version metadata and durable source/candidate selection remain blockers.",
        ],
        "remaining_blockers": [
            "Measure a real production-like copy fixture, including nonempty sqlite_sequence and deleted-highest-ID cases; this inventory intentionally never opens a user database.",
            "Define and test native equivalents for SQLite affinity/storage classes, original timestamp bytes, collations, constraints, INSERT OR IGNORE, and ON CONFLICT updates.",
            "Inventory dynamic SQL or consumers outside core/src and src-tauri/src separately before claiming repository-wide query-consumer completeness.",
            "Specify native schema metadata, conversion reconciliation, durable authority selection, cancellation, and recovery before enabling writes.",
            "Verify supported-platform packaging and full-app resource behavior independently.",
        ],
        "coverage_and_limits": {
            "runtime_object_enumeration": "Complete for objects created by all current App migrations in the synthetic SQLite database.",
            "query_consumer_inventory": "Bounded evidence only. Exact-name lexical hits are retained and manual owners were checked, but this is not proof that every dynamic, generated, frontend, or future consumer is covered.",
            "lexical_scanner": "Heuristic classifications can be false positives, especially tests, comments, migration DDL, formatted SQL, and nearby statements. Every hit is retained for inspection; unclassified hits are not silently promoted to verified consumers.",
            "lexical_hit_count": total_lexical_hits,
            "unclassified_lexical_hit_count": unclassified_lexical_hits,
            "data_scope": "No application or user database was opened. Runtime values, cardinalities, production high-water state, and application query frequency are unmeasured.",
            "conversion_scope": "No DuckDB converter or production schema is implemented or accepted by this inventory.",
        },
        "assertions": assertions,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({"completed": complete, "output": str(output), "migration_count": schema["migration_count"], "tables": len(tables), "objects": len(schema["schema_objects"])}))
    return 0 if complete else 1


if __name__ == "__main__":
    raise SystemExit(main())
