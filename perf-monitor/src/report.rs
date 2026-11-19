use serde::Serialize;

use crate::config::Thresholds;
use crate::monitor::{MonitorResult, MonitorRun, Phase};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
  Text,
  Json,
}

struct CheckResult {
  cpu_avg_ok: bool,
  cpu_p95_ok: bool,
  app_mem_avg_ok: bool,
  app_mem_p95_ok: bool,
  app_mem_growth_ok: bool,
  tree_mem_avg_ok: bool,
  tree_mem_p95_ok: bool,
  tree_mem_growth_ok: bool,
  all_passed: bool,
}

#[derive(Debug, Serialize)]
struct JsonReport {
  desktop: PhaseReport,
  /// `null` when this platform cannot move the app to the background.
  background: Option<PhaseReport>,
  passed: bool,
}

#[derive(Debug, Serialize)]
struct PhaseReport {
  duration_seconds: u64,
  warmup_seconds: u64,
  sample_count: usize,
  cpu: CpuReport,
  memory: MemoryReport,
  passed: bool,
}

#[derive(Debug, Serialize)]
struct CpuReport {
  avg_percent: f32,
  max_percent: f32,
  p95_percent: f32,
  threshold_avg: f32,
  threshold_p95: f32,
  avg_passed: bool,
  p95_passed: bool,
}

#[derive(Debug, Serialize)]
struct MemoryReport {
  app: MemoryBudgetReport,
  process_tree: MemoryBudgetReport,
  breakdown: MemoryBreakdownReport,
  process_count: ProcessCountReport,
}

#[derive(Debug, Serialize)]
struct MemoryBudgetReport {
  avg_mb: f64,
  max_mb: f64,
  p95_mb: f64,
  growth_mb: f64,
  threshold_avg_mb: f64,
  threshold_p95_mb: f64,
  threshold_growth_mb: f64,
  avg_passed: bool,
  p95_passed: bool,
  growth_passed: bool,
}

#[derive(Debug, Serialize)]
struct MemoryBreakdownReport {
  parent: MemoryComponentReport,
  webview: MemoryComponentReport,
  other: MemoryComponentReport,
}

#[derive(Debug, Serialize)]
struct MemoryComponentReport {
  avg_mb: f64,
  max_mb: f64,
  p95_mb: f64,
  growth_mb: f64,
}

#[derive(Debug, Serialize)]
struct ProcessCountReport {
  max_total: usize,
  max_webview: usize,
}

/// One measured phase checked against its thresholds.
struct PhaseCheck<'a> {
  phase: Phase,
  result: &'a MonitorResult,
  thresholds: &'a Thresholds,
  check: CheckResult,
}

impl<'a> PhaseCheck<'a> {
  fn new(phase: Phase, result: &'a MonitorResult, thresholds: &'a Thresholds) -> Self {
    Self {
      phase,
      result,
      thresholds,
      check: CheckResult::new(result, thresholds),
    }
  }
}

/// Formats and prints the report. Returns `true` if every measured phase
/// passed its thresholds.
pub fn format_report(
  run: &MonitorRun,
  desktop_thresholds: &Thresholds,
  background_thresholds: &Thresholds,
  format: OutputFormat,
) -> bool {
  let desktop = PhaseCheck::new(Phase::Desktop, &run.desktop, desktop_thresholds);
  let background = run
    .background
    .as_ref()
    .map(|result| PhaseCheck::new(Phase::Background, result, background_thresholds));
  let passed = desktop.check.all_passed
    && background
      .as_ref()
      .is_none_or(|background| background.check.all_passed);
  let text = text_report(&desktop, background.as_ref(), passed);

  match format {
    OutputFormat::Text => print!("{text}"),
    OutputFormat::Json => {
      print_json_report(&desktop, background.as_ref(), passed);
      // Keep the human-readable report in CI logs while stdout stays JSON.
      eprint!("{text}");
    }
  }

  passed
}

impl CheckResult {
  fn new(result: &MonitorResult, thresholds: &Thresholds) -> Self {
    let cpu_avg_ok = result.avg_cpu <= thresholds.max_avg_cpu_percent;
    let cpu_p95_ok = result.p95_cpu <= thresholds.max_p95_cpu_percent;
    let app_mem_avg_ok = result.parent_memory.avg_mb <= thresholds.max_avg_app_memory_mb;
    let app_mem_p95_ok = result.parent_memory.p95_mb <= thresholds.max_p95_app_memory_mb;
    let app_mem_growth_ok =
      result.parent_memory.growth_mb <= thresholds.max_app_memory_growth_mb;
    let tree_mem_avg_ok = result.avg_memory_mb <= thresholds.max_avg_memory_mb;
    let tree_mem_p95_ok = result.p95_memory_mb <= thresholds.max_p95_memory_mb;
    let tree_mem_growth_ok = result.memory_growth_mb <= thresholds.max_memory_growth_mb;

    Self {
      cpu_avg_ok,
      cpu_p95_ok,
      app_mem_avg_ok,
      app_mem_p95_ok,
      app_mem_growth_ok,
      tree_mem_avg_ok,
      tree_mem_p95_ok,
      tree_mem_growth_ok,
      all_passed: cpu_avg_ok
        && cpu_p95_ok
        && app_mem_avg_ok
        && app_mem_p95_ok
        && app_mem_growth_ok
        && tree_mem_avg_ok
        && tree_mem_p95_ok
        && tree_mem_growth_ok,
    }
  }
}

fn status_label(passed: bool) -> &'static str {
  if passed { "PASS" } else { "FAIL" }
}

fn text_report(
  desktop: &PhaseCheck<'_>,
  background: Option<&PhaseCheck<'_>>,
  passed: bool,
) -> String {
  use std::fmt::Write as _;

  let mut report = phase_text_report(desktop);
  match background {
    Some(background) => report.push_str(&phase_text_report(background)),
    None => {
      let _ = writeln!(report);
      let _ = writeln!(
        report,
        "=== Performance Test Results ({} phase) ===",
        Phase::Background
      );
      let _ = writeln!(
        report,
        "Not measured: closing the main window is implemented on Windows only."
      );
    }
  }
  let _ = writeln!(report);
  let _ = writeln!(report, "Result: {}", status_label(passed));

  report
}

fn phase_text_report(phase: &PhaseCheck<'_>) -> String {
  use std::fmt::Write as _;

  let PhaseCheck {
    phase,
    result,
    thresholds,
    check,
  } = phase;
  let warmup_label = match phase {
    Phase::Desktop => "warmup",
    Phase::Background => "settling after the window closed",
  };

  let mut report = String::new();
  let _ = writeln!(report);
  let _ = writeln!(report, "=== Performance Test Results ({phase} phase) ===");
  let _ = writeln!(
    report,
    "Duration: {}s (after {}s {warmup_label})",
    result.duration_seconds, result.warmup_seconds
  );
  let _ = writeln!(report, "Samples: {}", result.samples.len());
  let _ = writeln!(report);
  let _ = writeln!(report, "CPU Usage:");
  let _ = writeln!(
    report,
    "  Average: {:>6.1}%  (threshold: {:>5.1}%)  {}",
    result.avg_cpu,
    thresholds.max_avg_cpu_percent,
    status_label(check.cpu_avg_ok)
  );
  let _ = writeln!(
    report,
    "  P95:     {:>6.1}%  (threshold: {:>5.1}%)  {}",
    result.p95_cpu,
    thresholds.max_p95_cpu_percent,
    status_label(check.cpu_p95_ok)
  );
  let _ = writeln!(report);
  let _ = writeln!(report, "Memory (RSS, app process):");
  let _ = writeln!(
    report,
    "  Average: {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.parent_memory.avg_mb,
    thresholds.max_avg_app_memory_mb,
    status_label(check.app_mem_avg_ok)
  );
  let _ = writeln!(
    report,
    "  P95:     {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.parent_memory.p95_mb,
    thresholds.max_p95_app_memory_mb,
    status_label(check.app_mem_p95_ok)
  );
  let _ = writeln!(
    report,
    "  Growth:  {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.parent_memory.growth_mb,
    thresholds.max_app_memory_growth_mb,
    status_label(check.app_mem_growth_ok)
  );
  let _ = writeln!(report);
  let _ = writeln!(report, "Memory (RSS, process tree incl. WebView):");
  let _ = writeln!(
    report,
    "  Average: {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.avg_memory_mb,
    thresholds.max_avg_memory_mb,
    status_label(check.tree_mem_avg_ok)
  );
  let _ = writeln!(
    report,
    "  P95:     {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.p95_memory_mb,
    thresholds.max_p95_memory_mb,
    status_label(check.tree_mem_p95_ok)
  );
  let _ = writeln!(
    report,
    "  Growth:  {:>7.1} MB  (threshold: {:>6.1} MB)  {}",
    result.memory_growth_mb,
    thresholds.max_memory_growth_mb,
    status_label(check.tree_mem_growth_ok)
  );
  let _ = writeln!(
    report,
    "  Processes: total max {}, WebView max {}",
    result.max_process_count, result.max_webview_process_count
  );
  let _ = writeln!(report, "  Breakdown:");
  let _ = writeln!(
    report,
    "    Parent:  avg {:>7.1} MB, p95 {:>7.1} MB, growth {:>7.1} MB",
    result.parent_memory.avg_mb,
    result.parent_memory.p95_mb,
    result.parent_memory.growth_mb
  );
  let _ = writeln!(
    report,
    "    WebView: avg {:>7.1} MB, p95 {:>7.1} MB, growth {:>7.1} MB",
    result.webview_memory.avg_mb,
    result.webview_memory.p95_mb,
    result.webview_memory.growth_mb
  );
  let _ = writeln!(
    report,
    "    Other:   avg {:>7.1} MB, p95 {:>7.1} MB, growth {:>7.1} MB",
    result.other_process_memory.avg_mb,
    result.other_process_memory.p95_mb,
    result.other_process_memory.growth_mb
  );
  let _ = writeln!(report, "Phase result: {}", status_label(check.all_passed));

  report
}

fn print_json_report(
  desktop: &PhaseCheck<'_>,
  background: Option<&PhaseCheck<'_>>,
  passed: bool,
) {
  let report = JsonReport {
    desktop: phase_json_report(desktop),
    background: background.map(phase_json_report),
    passed,
  };

  match serde_json::to_string_pretty(&report) {
    Ok(json) => println!("{json}"),
    Err(e) => {
      eprintln!("Error: failed to serialize report: {e}");
      std::process::exit(1);
    }
  }
}

fn phase_json_report(phase: &PhaseCheck<'_>) -> PhaseReport {
  let PhaseCheck {
    result,
    thresholds,
    check,
    ..
  } = phase;

  PhaseReport {
    duration_seconds: result.duration_seconds,
    warmup_seconds: result.warmup_seconds,
    sample_count: result.samples.len(),
    cpu: CpuReport {
      avg_percent: result.avg_cpu,
      max_percent: result.max_cpu,
      p95_percent: result.p95_cpu,
      threshold_avg: thresholds.max_avg_cpu_percent,
      threshold_p95: thresholds.max_p95_cpu_percent,
      avg_passed: check.cpu_avg_ok,
      p95_passed: check.cpu_p95_ok,
    },
    memory: MemoryReport {
      app: MemoryBudgetReport {
        avg_mb: result.parent_memory.avg_mb,
        max_mb: result.parent_memory.max_mb,
        p95_mb: result.parent_memory.p95_mb,
        growth_mb: result.parent_memory.growth_mb,
        threshold_avg_mb: thresholds.max_avg_app_memory_mb,
        threshold_p95_mb: thresholds.max_p95_app_memory_mb,
        threshold_growth_mb: thresholds.max_app_memory_growth_mb,
        avg_passed: check.app_mem_avg_ok,
        p95_passed: check.app_mem_p95_ok,
        growth_passed: check.app_mem_growth_ok,
      },
      process_tree: MemoryBudgetReport {
        avg_mb: result.avg_memory_mb,
        max_mb: result.max_memory_mb,
        p95_mb: result.p95_memory_mb,
        growth_mb: result.memory_growth_mb,
        threshold_avg_mb: thresholds.max_avg_memory_mb,
        threshold_p95_mb: thresholds.max_p95_memory_mb,
        threshold_growth_mb: thresholds.max_memory_growth_mb,
        avg_passed: check.tree_mem_avg_ok,
        p95_passed: check.tree_mem_p95_ok,
        growth_passed: check.tree_mem_growth_ok,
      },
      breakdown: MemoryBreakdownReport {
        parent: MemoryComponentReport {
          avg_mb: result.parent_memory.avg_mb,
          max_mb: result.parent_memory.max_mb,
          p95_mb: result.parent_memory.p95_mb,
          growth_mb: result.parent_memory.growth_mb,
        },
        webview: MemoryComponentReport {
          avg_mb: result.webview_memory.avg_mb,
          max_mb: result.webview_memory.max_mb,
          p95_mb: result.webview_memory.p95_mb,
          growth_mb: result.webview_memory.growth_mb,
        },
        other: MemoryComponentReport {
          avg_mb: result.other_process_memory.avg_mb,
          max_mb: result.other_process_memory.max_mb,
          p95_mb: result.other_process_memory.p95_mb,
          growth_mb: result.other_process_memory.growth_mb,
        },
      },
      process_count: ProcessCountReport {
        max_total: result.max_process_count,
        max_webview: result.max_webview_process_count,
      },
    },
    passed: check.all_passed,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::monitor::{MemoryStats, ProcessMetrics};

  #[test]
  fn text_report_contains_threshold_status_and_memory_breakdown() {
    let result = monitor_result(1.0, 50.0, 200.0, 5);
    let thresholds = thresholds();
    let desktop = PhaseCheck::new(Phase::Desktop, &result, &thresholds);

    let report = text_report(&desktop, None, desktop.check.all_passed);

    assert!(report.contains("=== Performance Test Results (desktop phase) ==="));
    assert!(report.contains("Duration: 10s (after 5s warmup)"));
    assert!(report.contains("CPU Usage:"));
    assert!(report.contains("Average:    1.0%  (threshold:   5.0%)  PASS"));
    assert!(report.contains("Memory (RSS, app process):"));
    assert!(report.contains("Average:    50.0 MB  (threshold:  100.0 MB)  PASS"));
    assert!(report.contains("Memory (RSS, process tree incl. WebView):"));
    assert!(report.contains("Average:   275.0 MB  (threshold:  450.0 MB)  PASS"));
    assert!(report.contains("Processes: total max 3, WebView max 1"));
    assert!(report.contains("Parent:  avg    50.0 MB"));
    assert!(report.contains("WebView: avg   200.0 MB"));
    assert!(report.contains("Phase result: PASS"));
  }

  #[test]
  fn text_report_marks_an_unsupported_background_phase_as_not_measured() {
    let result = monitor_result(1.0, 50.0, 200.0, 5);
    let thresholds = thresholds();
    let desktop = PhaseCheck::new(Phase::Desktop, &result, &thresholds);

    let report = text_report(&desktop, None, true);

    assert!(report.contains("=== Performance Test Results (background phase) ==="));
    assert!(report.contains("Not measured:"));
    assert!(report.ends_with("Result: PASS\n"));
  }

  #[test]
  fn background_phase_reports_its_own_thresholds_and_settling_time() {
    let desktop_result = monitor_result(1.0, 50.0, 200.0, 5);
    let background_result = monitor_result(0.5, 50.0, 80.0, 12);
    let desktop_thresholds = thresholds();
    let background_thresholds = Thresholds {
      max_avg_memory_mb: 150.0,
      ..thresholds()
    };
    let desktop = PhaseCheck::new(Phase::Desktop, &desktop_result, &desktop_thresholds);
    let background = PhaseCheck::new(
      Phase::Background,
      &background_result,
      &background_thresholds,
    );

    let report = text_report(&desktop, Some(&background), false);
    let background_section = report
      .split("=== Performance Test Results (background phase) ===")
      .nth(1)
      .expect("background section");

    assert!(
      background_section
        .contains("Duration: 10s (after 12s settling after the window closed)")
    );
    assert!(
      background_section.contains("Average:   155.0 MB  (threshold:  150.0 MB)  FAIL")
    );
    assert!(background_section.contains("Phase result: FAIL"));
    assert!(report.ends_with("Result: FAIL\n"));
  }

  #[test]
  fn json_report_keeps_each_phase_check_separate() {
    let desktop_result = monitor_result(1.0, 50.0, 200.0, 5);
    let background_result = monitor_result(7.0, 50.0, 80.0, 12);
    let thresholds = thresholds();
    let desktop = PhaseCheck::new(Phase::Desktop, &desktop_result, &thresholds);
    let background = PhaseCheck::new(Phase::Background, &background_result, &thresholds);

    let desktop_json = phase_json_report(&desktop);
    let background_json = phase_json_report(&background);

    assert!(desktop_json.passed);
    assert!(!background_json.passed);
    assert!(!background_json.cpu.avg_passed);
    assert_eq!(background_json.warmup_seconds, 12);
  }

  fn monitor_result(
    cpu: f32,
    parent_mb: f64,
    webview_mb: f64,
    warmup_seconds: u64,
  ) -> MonitorResult {
    let other_mb = 25.0;
    let total_mb = parent_mb + webview_mb + other_mb;
    MonitorResult {
      samples: vec![ProcessMetrics {
        cpu_usage: cpu,
        memory_rss_mb: total_mb,
        parent_memory_rss_mb: parent_mb,
        webview_memory_rss_mb: webview_mb,
        other_process_memory_rss_mb: other_mb,
        process_count: 3,
        webview_process_count: 1,
      }],
      avg_cpu: cpu,
      max_cpu: cpu,
      p95_cpu: cpu,
      avg_memory_mb: total_mb,
      max_memory_mb: total_mb,
      p95_memory_mb: total_mb,
      memory_growth_mb: 0.0,
      parent_memory: memory_stats(parent_mb),
      webview_memory: memory_stats(webview_mb),
      other_process_memory: memory_stats(other_mb),
      max_process_count: 3,
      max_webview_process_count: 1,
      duration_seconds: 10,
      warmup_seconds,
    }
  }

  fn thresholds() -> Thresholds {
    Thresholds {
      max_avg_cpu_percent: 5.0,
      max_p95_cpu_percent: 10.0,
      max_avg_app_memory_mb: 100.0,
      max_p95_app_memory_mb: 100.0,
      max_app_memory_growth_mb: 50.0,
      max_avg_memory_mb: 450.0,
      max_p95_memory_mb: 500.0,
      max_memory_growth_mb: 50.0,
    }
  }

  fn memory_stats(value: f64) -> MemoryStats {
    MemoryStats {
      avg_mb: value,
      max_mb: value,
      p95_mb: value,
      growth_mb: 0.0,
    }
  }
}
