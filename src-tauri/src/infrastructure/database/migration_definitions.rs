use hardviz_core::infrastructure::database::migrate::SchemaMigration;

#[allow(dead_code)]
pub fn get_max_migration_version() -> i64 {
  get_migrations()
    .iter()
    .map(|m| m.version)
    .max()
    .unwrap_or(0)
}

/// Ordered forward migrations applied at startup by Core's migrator
/// ([`hardviz_core::infrastructure::database::migrate::run`]). Append-only:
/// the SQL of an already-released version must never change, or `sqlx`
/// will reject it as a checksum mismatch on existing databases.
pub fn get_migrations() -> Vec<SchemaMigration> {
  vec![
    SchemaMigration {
      version: 1,
      description: "create_initial_tables",
      sql: "CREATE TABLE DATA_ARCHIVE (id INTEGER PRIMARY KEY, cpu_avg INTEGER, cpu_max INTEGER, cpu_min INTEGER, ram_avg INTEGER, ram_max INTEGER, ram_min INTEGER, timestamp DATETIME);",
    },
    SchemaMigration {
      version: 2,
      description: "create_gpu_tables",
      sql: "CREATE TABLE GPU_DATA_ARCHIVE (id INTEGER PRIMARY KEY, gpu_name TEXT, usage_avg INTEGER, usage_max INTEGER, usage_min INTEGER, temperature_avg INTEGER, temperature_max INTEGER, temperature_min INTEGER, timestamp DATETIME);",
    },
    SchemaMigration {
      version: 3,
      description: "add_gpu_memory_usage_columns",
      sql: r#"
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_avg INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_max INTEGER;
        ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN dedicated_memory_min INTEGER;
      "#,
    },
    SchemaMigration {
      version: 4,
      description: "create_process_stats",
      sql: "CREATE TABLE PROCESS_STATS (id INTEGER PRIMARY KEY AUTOINCREMENT, pid INTEGER NOT NULL, process_name TEXT NOT NULL,  cpu_usage REAL NOT NULL,  memory_usage INTEGER NOT NULL, execution_sec INTEGER NOT NULL, timestamp DATETIME NOT NULL);",
    },
    SchemaMigration {
      version: 5,
      description: "add_gpu_id_column",
      sql: "ALTER TABLE GPU_DATA_ARCHIVE ADD COLUMN gpu_id TEXT;",
    },
    SchemaMigration {
      version: 6,
      description: "create_storage_smart_daily_snapshots",
      sql: r#"
        CREATE TABLE storage_devices (
          id TEXT PRIMARY KEY,
          display_name TEXT NOT NULL,
          model TEXT,
          serial_hash TEXT,
          protocol TEXT,
          capacity_bytes INTEGER,
          first_seen_at TEXT NOT NULL,
          last_seen_at TEXT NOT NULL,
          is_active INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE storage_smart_daily_snapshots (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          device_id TEXT NOT NULL,
          date TEXT NOT NULL,
          health_status TEXT NOT NULL,
          warning_level TEXT NOT NULL DEFAULT 'none',
          warning_reasons TEXT,
          temperature_celsius REAL,
          power_on_hours INTEGER,
          percentage_used REAL,
          available_spare_percent REAL,
          reallocated_sector_count INTEGER,
          current_pending_sector_count INTEGER,
          offline_uncorrectable_count INTEGER,
          media_errors INTEGER,
          error_log_entries INTEGER,
          unsafe_shutdown_count INTEGER,
          collected_at TEXT NOT NULL,
          UNIQUE(device_id, date),
          FOREIGN KEY(device_id) REFERENCES storage_devices(id)
        );
      "#,
    },
    SchemaMigration {
      version: 7,
      description: "rename_storage_smart_daily_snapshots_to_storage_health_daily_records",
      sql: r#"
        ALTER TABLE storage_smart_daily_snapshots
        RENAME TO storage_health_daily_records;
      "#,
    },
    SchemaMigration {
      version: 8,
      description: "add_process_stats_timestamp_index",
      sql: "CREATE INDEX IF NOT EXISTS idx_process_stats_timestamp ON PROCESS_STATS(timestamp);",
    },
    SchemaMigration {
      version: 9,
      description: "add_cpu_temperature_archive_columns",
      sql: r#"
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_temperature_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_temperature_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_temperature_min REAL;
      "#,
    },
    SchemaMigration {
      version: 10,
      description: "add_power_archive_columns",
      sql: r#"
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_power_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_power_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN cpu_power_min REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN gpu_power_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN gpu_power_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN gpu_power_min REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN ane_power_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN ane_power_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN ane_power_min REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN package_power_avg REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN package_power_max REAL;
        ALTER TABLE DATA_ARCHIVE ADD COLUMN package_power_min REAL;
      "#,
    },
    SchemaMigration {
      version: 11,
      description: "create_cooling_daily_summary",
      sql: r#"
        CREATE TABLE cooling_daily_summary (
          date TEXT PRIMARY KEY,
          idle_cpu_temperature_avg REAL,
          idle_cpu_temperature_max REAL,
          idle_cpu_temperature_min REAL,
          idle_sample_minutes INTEGER NOT NULL DEFAULT 0,
          low_cpu_temperature_avg REAL,
          low_cpu_temperature_max REAL,
          low_cpu_temperature_min REAL,
          low_sample_minutes INTEGER NOT NULL DEFAULT 0,
          mid_cpu_temperature_avg REAL,
          mid_cpu_temperature_max REAL,
          mid_cpu_temperature_min REAL,
          mid_sample_minutes INTEGER NOT NULL DEFAULT 0,
          high_cpu_temperature_avg REAL,
          high_cpu_temperature_max REAL,
          high_cpu_temperature_min REAL,
          high_sample_minutes INTEGER NOT NULL DEFAULT 0,
          coverage_minutes INTEGER NOT NULL
        );
      "#,
    },
    SchemaMigration {
      version: 12,
      description: "create_cooling_baseline",
      sql: r#"
        CREATE TABLE cooling_baseline (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          window_start_date TEXT NOT NULL,
          window_end_date TEXT NOT NULL,
          idle_temperature_avg REAL NOT NULL,
          sample_minutes INTEGER NOT NULL,
          established_at TEXT NOT NULL
        );
      "#,
    },
    SchemaMigration {
      version: 13,
      description: "create_cooling_hourly_summary",
      sql: r#"
        CREATE TABLE cooling_hourly_summary (
          hour_start TEXT PRIMARY KEY,
          cpu_usage_avg REAL,
          cpu_temperature_avg REAL,
          sample_minutes INTEGER NOT NULL
        );
      "#,
    },
    SchemaMigration {
      version: 14,
      description: "add_cooling_daily_summary_power_columns",
      // The CPU package power the timeline's power lane reads for 90d/1y
      // (#2021). Nullable so a machine with no power sampler keeps
      // reporting absent power rather than 0 W; `power_sample_minutes`
      // defaults to 0 so rows written before this migration read back as
      // "no powered minutes" instead of failing the NOT NULL constraint.
      sql: r#"
        ALTER TABLE cooling_daily_summary ADD COLUMN cpu_power_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN cpu_power_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN cpu_power_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN power_sample_minutes INTEGER NOT NULL DEFAULT 0;
      "#,
    },
    SchemaMigration {
      version: 15,
      description: "create_ambient_archive",
      // Row-per-source (#2043): more than one ambient sensor in a room is
      // plausible, and each one is a distinct Sensor Source Label rather
      // than a column. `temperature` is NOT NULL because a row only
      // exists when a fresh reading backs it - a minute with no usable
      // ambient sample has no row at all, never a zeroed one (DP-02).
      // `humidity` is nullable: temperature-only sensors are common.
      sql: r#"
        CREATE TABLE AMBIENT_ARCHIVE (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          source TEXT NOT NULL,
          temperature REAL NOT NULL,
          humidity REAL,
          timestamp DATETIME NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_ambient_archive_timestamp ON AMBIENT_ARCHIVE(timestamp);
      "#,
    },
    SchemaMigration {
      version: 16,
      description: "add_cooling_daily_summary_ambient_delta_columns",
      // The per-band thermal delta (CPU package temperature minus ambient,
      // #2045) plus how many of the day's archived minutes carried an
      // ambient pair at all. Nullable delta columns and a defaulted
      // `*_delta_sample_minutes` so every row written before this
      // migration - and every row on a machine with no ambient sensor -
      // reads back as absent rather than 0 K (DP-02).
      //
      // `ambient_coverage_minutes` is counted outside the load-band gate,
      // the same way `power_sample_minutes` is: ambient availability is a
      // separate capability from CPU temperature, and the backfill cursor
      // needs a fact that a machine without a CPU temperature sensor can
      // still record.
      sql: r#"
        ALTER TABLE cooling_daily_summary ADD COLUMN idle_delta_temperature_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN idle_delta_temperature_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN idle_delta_temperature_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN idle_delta_sample_minutes INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE cooling_daily_summary ADD COLUMN low_delta_temperature_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN low_delta_temperature_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN low_delta_temperature_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN low_delta_sample_minutes INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE cooling_daily_summary ADD COLUMN mid_delta_temperature_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN mid_delta_temperature_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN mid_delta_temperature_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN mid_delta_sample_minutes INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE cooling_daily_summary ADD COLUMN high_delta_temperature_avg REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN high_delta_temperature_max REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN high_delta_temperature_min REAL;
        ALTER TABLE cooling_daily_summary ADD COLUMN high_delta_sample_minutes INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE cooling_daily_summary ADD COLUMN ambient_coverage_minutes INTEGER NOT NULL DEFAULT 0;
      "#,
    },
    SchemaMigration {
      version: 17,
      description: "create_fan_archive",
      // The one-minute fan-speed archive behind the Cooling Insight fan
      // lane (#2022). Row-per-fan rather than fixed columns because how
      // many fans a machine exposes is configuration-dependent, and both
      // value columns are NOT NULL because a row is only written for a
      // reading that was actually taken: an unreadable fan is absent,
      // never 0 RPM (which is a real Inactive Fan Reading).
      sql: r#"
        CREATE TABLE FAN_ARCHIVE (
          id INTEGER PRIMARY KEY,
          source TEXT NOT NULL,
          rpm INTEGER NOT NULL,
          timestamp DATETIME NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_fan_archive_timestamp ON FAN_ARCHIVE(timestamp);
      "#,
    },
    SchemaMigration {
      version: 18,
      description: "create_cooling_fan_daily_summary",
      // The long-lived per-fan daily rollup the 90d/1y fan lane reads
      // (#2022). Keyed by (date, source) for the same row-per-fan reason
      // as `FAN_ARCHIVE`; a fan with no archived reading that day simply
      // has no row.
      sql: r#"
        CREATE TABLE cooling_fan_daily_summary (
          date TEXT NOT NULL,
          source TEXT NOT NULL,
          rpm_avg REAL NOT NULL,
          rpm_max INTEGER NOT NULL,
          rpm_min INTEGER NOT NULL,
          sample_minutes INTEGER NOT NULL,
          PRIMARY KEY (date, source)
        );
      "#,
    },
    SchemaMigration {
      version: 19,
      description: "add_data_archive_timestamp_index",
      // `DATA_ARCHIVE` is the one archive table that never had a
      // timestamp index, and it is also the largest: at one row per
      // minute a year of history is over half a million rows.
      //
      // Every read of it is bounded by a time range - the cooling
      // rollup's per-day fetch, the ambient pairing join (#2045), the
      // retention delete - and without this index each of those is a
      // full table scan. The pairing join made that acute, because it
      // pairs `AMBIENT_ARCHIVE` against this table per archived minute.
      //
      // `IF NOT EXISTS` matches the other index migrations, so this is a
      // no-op on a database that somehow already has it.
      sql: r#"
        CREATE INDEX IF NOT EXISTS idx_data_archive_timestamp ON DATA_ARCHIVE(timestamp);
      "#,
    },
    SchemaMigration {
      version: 20,
      description: "create_cooling_delta_baseline",
      // The pinned ambient-normalized (ΔT) cooling baseline (#2045).
      //
      // Its own table rather than columns on `cooling_baseline` because
      // the two establish at different times: ambient collection
      // commonly begins long after the absolute baseline was pinned.
      // Pinning is write-once against a `CHECK (id = 1)` row, and that
      // is exactly what makes an established baseline undriftable - two
      // baselines sharing one such row would force the later one to
      // arrive as an UPDATE, a weaker rule that has to be got right
      // rather than being impossible to get wrong.
      sql: r#"
        CREATE TABLE cooling_delta_baseline (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          window_start_date TEXT NOT NULL,
          window_end_date TEXT NOT NULL,
          delta_temperature_avg REAL NOT NULL,
          sample_minutes INTEGER NOT NULL,
          established_at TEXT NOT NULL
        );
      "#,
    },
    SchemaMigration {
      version: 21,
      description: "create_cooling_thermal_delta_daily_summary",
      // The per-band Thermal Delta rollup moves off `cooling_daily_summary`
      // into its own table keyed by `(date, source)`, the same shape as
      // `cooling_fan_daily_summary` (#2062). `AMBIENT_ARCHIVE` has been
      // row-per-source since #2043 and the user now chooses which sensor
      // is read; the v16 columns collapsed every source into one
      // per-minute mean, so a later sensor change would have blended two
      // placements into one ΔT - and into the pinned baseline - with no
      // way to tell afterwards. Nullable delta columns and defaulted
      // `*_delta_sample_minutes` for the same reason as v16: a band with
      // no paired minute reads back absent, never 0 K (DP-02).
      // `coverage_minutes` is NOT NULL without a default because a row
      // exists only for a source that paired at least one minute.
      //
      // The v16 columns are dropped rather than left unused: no released
      // build ever wrote them (v16 and v20 both post-date v1.10.1), so
      // there is nothing to migrate, and a source-blind ΔT could not be
      // attributed to a row here anyway. Days the one-minute archives
      // still hold are re-rolled into the new table by the catch-up
      // cursor (`cooling_rollup::ambient_rollup_is_behind`).
      sql: r#"
        CREATE TABLE cooling_thermal_delta_daily_summary (
          date TEXT NOT NULL,
          source TEXT NOT NULL,
          coverage_minutes INTEGER NOT NULL,
          idle_delta_temperature_avg REAL,
          idle_delta_temperature_max REAL,
          idle_delta_temperature_min REAL,
          idle_delta_sample_minutes INTEGER NOT NULL DEFAULT 0,
          low_delta_temperature_avg REAL,
          low_delta_temperature_max REAL,
          low_delta_temperature_min REAL,
          low_delta_sample_minutes INTEGER NOT NULL DEFAULT 0,
          mid_delta_temperature_avg REAL,
          mid_delta_temperature_max REAL,
          mid_delta_temperature_min REAL,
          mid_delta_sample_minutes INTEGER NOT NULL DEFAULT 0,
          high_delta_temperature_avg REAL,
          high_delta_temperature_max REAL,
          high_delta_temperature_min REAL,
          high_delta_sample_minutes INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (date, source)
        );
        ALTER TABLE cooling_daily_summary DROP COLUMN idle_delta_temperature_avg;
        ALTER TABLE cooling_daily_summary DROP COLUMN idle_delta_temperature_max;
        ALTER TABLE cooling_daily_summary DROP COLUMN idle_delta_temperature_min;
        ALTER TABLE cooling_daily_summary DROP COLUMN idle_delta_sample_minutes;
        ALTER TABLE cooling_daily_summary DROP COLUMN low_delta_temperature_avg;
        ALTER TABLE cooling_daily_summary DROP COLUMN low_delta_temperature_max;
        ALTER TABLE cooling_daily_summary DROP COLUMN low_delta_temperature_min;
        ALTER TABLE cooling_daily_summary DROP COLUMN low_delta_sample_minutes;
        ALTER TABLE cooling_daily_summary DROP COLUMN mid_delta_temperature_avg;
        ALTER TABLE cooling_daily_summary DROP COLUMN mid_delta_temperature_max;
        ALTER TABLE cooling_daily_summary DROP COLUMN mid_delta_temperature_min;
        ALTER TABLE cooling_daily_summary DROP COLUMN mid_delta_sample_minutes;
        ALTER TABLE cooling_daily_summary DROP COLUMN high_delta_temperature_avg;
        ALTER TABLE cooling_daily_summary DROP COLUMN high_delta_temperature_max;
        ALTER TABLE cooling_daily_summary DROP COLUMN high_delta_temperature_min;
        ALTER TABLE cooling_daily_summary DROP COLUMN high_delta_sample_minutes;
        ALTER TABLE cooling_daily_summary DROP COLUMN ambient_coverage_minutes;
      "#,
    },
    SchemaMigration {
      version: 22,
      description: "add_cooling_delta_baseline_source",
      // The pinned ΔT baseline records which ambient source it was
      // established from (#2062), so every later comparison can refuse a
      // recent window measured against a different sensor. Recreated
      // rather than altered: a row pinned by v20 was derived from the
      // source-blind v16 columns and cannot say which placement it
      // describes - it is exactly the mixture this change forbids - so it
      // is discarded and re-established from the row-per-source rollup.
      // No released build ever pinned one, so nothing is lost. The
      // write-once `CHECK (id = 1)` rule is unchanged.
      sql: r#"
        DROP TABLE cooling_delta_baseline;
        CREATE TABLE cooling_delta_baseline (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          source TEXT NOT NULL,
          window_start_date TEXT NOT NULL,
          window_end_date TEXT NOT NULL,
          delta_temperature_avg REAL NOT NULL,
          sample_minutes INTEGER NOT NULL,
          established_at TEXT NOT NULL
        );
      "#,
    },
    SchemaMigration {
      version: 23,
      description: "create_cooling_covariate_daily_summaries",
      // The paired co-variate rollup (#2068): per ambient source and
      // CPU-load band, the sufficient statistics of a least-squares fit
      // of the Thermal Delta against CPU package power (`n, Σx, Σy, Σxy,
      // Σx², Σy²`), the day's medians of power, ΔT and ambient, and the
      // band's share of the day; and per fan beside each of those, the
      // same six sums for ΔT against fan speed with the day's median rpm.
      // Slope, intercept and Pearson r are derived at query time - the
      // sums add across days, so a window's fit needs no minute past the
      // archive's own retention.
      //
      // Keyed by `(date, source, band)` and `(date, source, fan_source,
      // band)`: the ambient source is on every row for the reason
      // `cooling_thermal_delta_daily_summary` carries it, so a sensor
      // change can never mix two placements into one fit. Nullable
      // medians and defaulted counts for the reason v21 has them: a
      // reading the day never carried reads back absent, never zero.
      // `band_share` and `ambient_temperature_median` are NOT NULL because
      // a row exists only for a band that saw a paired minute, and every
      // paired minute carries an ambient reading. The fit sums default
      // to 0 with `*_n` as the presence flag - `n = 0` is the empty fit.
      //
      // Days the one-minute archives still hold are back-filled by the
      // catch-up cursor (`cooling_rollup::covariate_rollup_is_behind`).
      sql: r#"
        CREATE TABLE cooling_covariate_daily_summary (
          date TEXT NOT NULL,
          source TEXT NOT NULL,
          band TEXT NOT NULL,
          sample_minutes INTEGER NOT NULL,
          band_share REAL NOT NULL,
          ambient_temperature_median REAL NOT NULL,
          delta_minutes INTEGER NOT NULL DEFAULT 0,
          delta_temperature_median REAL,
          power_minutes INTEGER NOT NULL DEFAULT 0,
          cpu_power_median REAL,
          power_fit_n INTEGER NOT NULL DEFAULT 0,
          power_fit_sum_x REAL NOT NULL DEFAULT 0,
          power_fit_sum_y REAL NOT NULL DEFAULT 0,
          power_fit_sum_xy REAL NOT NULL DEFAULT 0,
          power_fit_sum_xx REAL NOT NULL DEFAULT 0,
          power_fit_sum_yy REAL NOT NULL DEFAULT 0,
          PRIMARY KEY (date, source, band)
        );
        CREATE TABLE cooling_fan_covariate_daily_summary (
          date TEXT NOT NULL,
          source TEXT NOT NULL,
          fan_source TEXT NOT NULL,
          band TEXT NOT NULL,
          rpm_minutes INTEGER NOT NULL,
          rpm_median REAL NOT NULL,
          fit_n INTEGER NOT NULL DEFAULT 0,
          fit_sum_x REAL NOT NULL DEFAULT 0,
          fit_sum_y REAL NOT NULL DEFAULT 0,
          fit_sum_xy REAL NOT NULL DEFAULT 0,
          fit_sum_xx REAL NOT NULL DEFAULT 0,
          fit_sum_yy REAL NOT NULL DEFAULT 0,
          PRIMARY KEY (date, source, fan_source, band)
        );
      "#,
    },
  ]
}
