use hardviz_core::infrastructure::database::archive_queries::{
  AmbientArchiveBucket as CoreAmbientArchiveBucket,
  AmbientArchiveSeries as CoreAmbientArchiveSeries,
  ArchiveBucketTimestamp as CoreArchiveBucketTimestamp,
  ArchiveSeriesPoint as CoreArchiveSeriesPoint, DataArchiveColumn,
  FanArchiveSeries as CoreFanArchiveSeries, GpuArchiveColumn,
  ProcessStatRecord as CoreProcessStatRecord,
};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveDataStats {
  Avg,
  Max,
  Min,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveBucketTimestamp {
  Start,
  End,
}

impl From<ArchiveBucketTimestamp> for CoreArchiveBucketTimestamp {
  fn from(value: ArchiveBucketTimestamp) -> Self {
    match value {
      ArchiveBucketTimestamp::Start => Self::Start,
      ArchiveBucketTimestamp::End => Self::End,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DataArchiveHardwareType {
  Cpu,
  CpuTemperature,
  CpuPower,
  GpuPower,
  AnePower,
  PackagePower,
  Memory,
}

impl DataArchiveHardwareType {
  pub fn column(self, stats: ArchiveDataStats) -> DataArchiveColumn {
    match (self, stats) {
      (Self::Cpu, ArchiveDataStats::Avg) => DataArchiveColumn::CpuAvg,
      (Self::Cpu, ArchiveDataStats::Max) => DataArchiveColumn::CpuMax,
      (Self::Cpu, ArchiveDataStats::Min) => DataArchiveColumn::CpuMin,
      (Self::CpuTemperature, ArchiveDataStats::Avg) => {
        DataArchiveColumn::CpuTemperatureAvg
      }
      (Self::CpuTemperature, ArchiveDataStats::Max) => {
        DataArchiveColumn::CpuTemperatureMax
      }
      (Self::CpuTemperature, ArchiveDataStats::Min) => {
        DataArchiveColumn::CpuTemperatureMin
      }
      (Self::CpuPower, ArchiveDataStats::Avg) => DataArchiveColumn::CpuPowerAvg,
      (Self::CpuPower, ArchiveDataStats::Max) => DataArchiveColumn::CpuPowerMax,
      (Self::CpuPower, ArchiveDataStats::Min) => DataArchiveColumn::CpuPowerMin,
      (Self::GpuPower, ArchiveDataStats::Avg) => DataArchiveColumn::GpuPowerAvg,
      (Self::GpuPower, ArchiveDataStats::Max) => DataArchiveColumn::GpuPowerMax,
      (Self::GpuPower, ArchiveDataStats::Min) => DataArchiveColumn::GpuPowerMin,
      (Self::AnePower, ArchiveDataStats::Avg) => DataArchiveColumn::AnePowerAvg,
      (Self::AnePower, ArchiveDataStats::Max) => DataArchiveColumn::AnePowerMax,
      (Self::AnePower, ArchiveDataStats::Min) => DataArchiveColumn::AnePowerMin,
      (Self::PackagePower, ArchiveDataStats::Avg) => DataArchiveColumn::PackagePowerAvg,
      (Self::PackagePower, ArchiveDataStats::Max) => DataArchiveColumn::PackagePowerMax,
      (Self::PackagePower, ArchiveDataStats::Min) => DataArchiveColumn::PackagePowerMin,
      (Self::Memory, ArchiveDataStats::Avg) => DataArchiveColumn::RamAvg,
      (Self::Memory, ArchiveDataStats::Max) => DataArchiveColumn::RamMax,
      (Self::Memory, ArchiveDataStats::Min) => DataArchiveColumn::RamMin,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GpuArchiveDataType {
  Usage,
  Temp,
  DedicatedMemory,
}

impl GpuArchiveDataType {
  pub fn column(self, stats: ArchiveDataStats) -> GpuArchiveColumn {
    match (self, stats) {
      (Self::Usage, ArchiveDataStats::Avg) => GpuArchiveColumn::UsageAvg,
      (Self::Usage, ArchiveDataStats::Max) => GpuArchiveColumn::UsageMax,
      (Self::Usage, ArchiveDataStats::Min) => GpuArchiveColumn::UsageMin,
      (Self::Temp, ArchiveDataStats::Avg) => GpuArchiveColumn::TemperatureAvg,
      (Self::Temp, ArchiveDataStats::Max) => GpuArchiveColumn::TemperatureMax,
      (Self::Temp, ArchiveDataStats::Min) => GpuArchiveColumn::TemperatureMin,
      (Self::DedicatedMemory, ArchiveDataStats::Avg) => {
        GpuArchiveColumn::DedicatedMemoryAvg
      }
      (Self::DedicatedMemory, ArchiveDataStats::Max) => {
        GpuArchiveColumn::DedicatedMemoryMax
      }
      (Self::DedicatedMemory, ArchiveDataStats::Min) => {
        GpuArchiveColumn::DedicatedMemoryMin
      }
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ArchiveSeriesPoint {
  pub timestamp: i64,
  pub value: Option<f64>,
}

impl From<CoreArchiveSeriesPoint> for ArchiveSeriesPoint {
  fn from(point: CoreArchiveSeriesPoint) -> Self {
    Self {
      timestamp: point.timestamp,
      value: point.value,
    }
  }
}

/// One archived fan's bucketed RPM series (#2022). Row-per-fan on disk, so
/// how many series come back depends on the machine's configuration rather
/// than on a fixed set the caller names; an empty response is exactly how a
/// machine with no readable fan reports itself.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FanArchiveSeries {
  /// The fan's stable channel-derived identifier, as archived.
  pub source: String,
  pub points: Vec<ArchiveSeriesPoint>,
}

impl From<CoreFanArchiveSeries> for FanArchiveSeries {
  fn from(series: CoreFanArchiveSeries) -> Self {
    Self {
      source: series.source,
      points: series.points.into_iter().map(Into::into).collect(),
    }
  }
}

/// One bucket of the Cooling Insight ambient lane (#2046). `ambientAvg`
/// null means no minute in the bucket carried an ambient row, and
/// `deltaAvg` null means none carried both an ambient row and a CPU
/// package temperature - neither is a measured zero, and the lane must
/// break there rather than interpolate.
//
// Kept to a single paragraph deliberately: tauri-specta renders a blank
// `///` line as `" * "` in `bindings.ts`, whose trailing space fails CI's
// `git diff --check`. The generated file must not be hand-edited, so the
// paragraph break has to go here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AmbientArchiveBucket {
  pub timestamp: i64,
  pub ambient_avg: Option<f64>,
  pub delta_avg: Option<f64>,
}

impl From<CoreAmbientArchiveBucket> for AmbientArchiveBucket {
  fn from(bucket: CoreAmbientArchiveBucket) -> Self {
    Self {
      timestamp: bucket.timestamp,
      ambient_avg: bucket.ambient_avg,
      delta_avg: bucket.delta_avg,
    }
  }
}

/// The ambient lane of one Cooling Insight timeline range (#2046).
/// `AMBIENT_ARCHIVE` is row-per-source, so `sources` names the labels that
/// actually contributed to this window; an empty list is exactly how a
/// machine with no environmental sensor reports itself.
//
// Same single-paragraph constraint as [`AmbientArchiveBucket`].
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AmbientArchiveSeries {
  pub sources: Vec<String>,
  pub buckets: Vec<AmbientArchiveBucket>,
}

impl From<CoreAmbientArchiveSeries> for AmbientArchiveSeries {
  fn from(series: CoreAmbientArchiveSeries) -> Self {
    Self {
      sources: series.sources,
      buckets: series.buckets.into_iter().map(Into::into).collect(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct ProcessStatRecord {
  pub pid: i64,
  pub process_name: String,
  pub avg_cpu_usage: f64,
  pub avg_memory_usage: f64,
  pub total_execution_sec: i64,
  pub latest_timestamp: String,
}

impl From<CoreProcessStatRecord> for ProcessStatRecord {
  fn from(record: CoreProcessStatRecord) -> Self {
    Self {
      pid: record.pid,
      process_name: record.process_name,
      avg_cpu_usage: record.avg_cpu_usage,
      avg_memory_usage: record.avg_memory_usage,
      total_execution_sec: record.total_execution_sec,
      latest_timestamp: record.latest_timestamp,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cpu_temperature_stats_map_to_temperature_columns() {
    assert_eq!(
      DataArchiveHardwareType::CpuTemperature.column(ArchiveDataStats::Avg),
      DataArchiveColumn::CpuTemperatureAvg
    );
    assert_eq!(
      DataArchiveHardwareType::CpuTemperature.column(ArchiveDataStats::Max),
      DataArchiveColumn::CpuTemperatureMax
    );
    assert_eq!(
      DataArchiveHardwareType::CpuTemperature.column(ArchiveDataStats::Min),
      DataArchiveColumn::CpuTemperatureMin
    );
  }

  #[test]
  fn power_stats_map_to_component_columns() {
    let cases = [
      (
        DataArchiveHardwareType::CpuPower,
        DataArchiveColumn::CpuPowerAvg,
        DataArchiveColumn::CpuPowerMax,
        DataArchiveColumn::CpuPowerMin,
      ),
      (
        DataArchiveHardwareType::GpuPower,
        DataArchiveColumn::GpuPowerAvg,
        DataArchiveColumn::GpuPowerMax,
        DataArchiveColumn::GpuPowerMin,
      ),
      (
        DataArchiveHardwareType::AnePower,
        DataArchiveColumn::AnePowerAvg,
        DataArchiveColumn::AnePowerMax,
        DataArchiveColumn::AnePowerMin,
      ),
      (
        DataArchiveHardwareType::PackagePower,
        DataArchiveColumn::PackagePowerAvg,
        DataArchiveColumn::PackagePowerMax,
        DataArchiveColumn::PackagePowerMin,
      ),
    ];
    for (hardware_type, avg, max, min) in cases {
      assert_eq!(hardware_type.column(ArchiveDataStats::Avg), avg);
      assert_eq!(hardware_type.column(ArchiveDataStats::Max), max);
      assert_eq!(hardware_type.column(ArchiveDataStats::Min), min);
    }
  }
}
