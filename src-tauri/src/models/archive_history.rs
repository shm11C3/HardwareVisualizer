use hardviz_core::infrastructure::database::archive_queries::{
  ArchiveRecord as CoreArchiveRecord, DataArchiveColumn, GpuArchiveColumn,
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
pub enum DataArchiveHardwareType {
  Cpu,
  Memory,
}

impl DataArchiveHardwareType {
  pub fn column(self, stats: ArchiveDataStats) -> DataArchiveColumn {
    match (self, stats) {
      (Self::Cpu, ArchiveDataStats::Avg) => DataArchiveColumn::CpuAvg,
      (Self::Cpu, ArchiveDataStats::Max) => DataArchiveColumn::CpuMax,
      (Self::Cpu, ArchiveDataStats::Min) => DataArchiveColumn::CpuMin,
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
pub struct ArchiveRecord {
  pub id: i64,
  pub value: Option<f64>,
  pub timestamp: String,
}

impl From<CoreArchiveRecord> for ArchiveRecord {
  fn from(record: CoreArchiveRecord) -> Self {
    Self {
      id: record.id,
      value: record.value,
      timestamp: record.timestamp,
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
