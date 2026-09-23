//! Command-line modes of the application binary that run without the Tauri
//! runtime.
//!
//! External Component Setup (ADR 0024): the Settings action launches the
//! executable elevated in this mode and the Windows installer's custom action
//! invokes it from its elevated context, so one Core code path serves both
//! entry points. The mode reports through its exit code only; it never writes
//! a result anywhere the caller could have redirected.
//!
//! External component uninstall notice (#2119): both Windows uninstallers run
//! the executable in this mode, unelevated, before removing it. It tells the
//! user which installed components the uninstall keeps, using the same Core
//! detection as Settings, and always exits 0 so it can never block removal.
//!
//! Elevated relaunch handoff (follow-up to #2216): "restart as administrator"
//! and Elevated Startup Mode launch the elevated child while the current
//! process is still fully running, so a declined UAC prompt leaves it
//! untouched. The child is told the parent's process id and waits for that
//! process to exit before the Tauri runtime starts: until then the parent
//! holds the single-instance lock and the database, so a child that ran
//! ahead would exit as a second instance or race the writers the parent is
//! still draining.

use hardviz_core::external_component_setup::{
  ExternalComponentSetupOutcome, ExternalComponentSetupResult, RuntimeInstallState,
  SetupFailureStage, setup_plan,
};
use hardviz_core::models::ExternalComponent;
use hardviz_core::platform::factory::PlatformFactory;
use hardviz_core::platform::traits::ProcessExitWait;
use std::time::Duration;

pub const EXTERNAL_COMPONENT_SETUP_FLAG: &str = "--external-component-setup";
pub const EXTERNAL_COMPONENT_NOTICE_FLAG: &str = "--external-component-notice";
pub const WAIT_FOR_PARENT_FLAG: &str = "--wait-for-parent";
const UNINSTALL_NOTICE: &str = "uninstall";
/// How long the elevated child waits for its parent. The parent only has to
/// drain its workers and close the database, which takes seconds; the bound
/// exists so a parent that hangs, or a process id that was reused after the
/// parent exited, cannot keep the child from ever starting.
const PARENT_EXIT_TIMEOUT: Duration = Duration::from_secs(60);
/// Components External Component Setup can install, in notice order.
const SETUP_COMPONENTS: [ExternalComponent; 2] =
  [ExternalComponent::Pawnio, ExternalComponent::Smartctl];

/// Stable command-line identifiers for components with a setup plan.
pub fn component_cli_id(component: ExternalComponent) -> &'static str {
  match component {
    ExternalComponent::Pawnio => "pawnio",
    ExternalComponent::Smartctl => "smartctl",
  }
}

fn component_from_cli_id(id: &str) -> Option<ExternalComponent> {
  match id {
    "pawnio" => Some(ExternalComponent::Pawnio),
    "smartctl" => Some(ExternalComponent::Smartctl),
    _ => None,
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliMode {
  ExternalComponentSetup { component: ExternalComponent },
  ExternalComponentUninstallNotice,
}

/// What the process was started with, as far as the App itself decides.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CliArgs {
  /// A mode that runs instead of the app, or `None` for a normal launch.
  pub mode: Option<CliMode>,
  /// The process that launched this one elevated and that must exit before
  /// the app starts (see [`WAIT_FOR_PARENT_FLAG`]).
  pub wait_for_parent: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliParseError {
  UnknownComponent(String),
  UnknownNotice(String),
  MissingValue(&'static str),
  InvalidParentPid(String),
}

/// Recognize the App's own command-line arguments; anything else is left to
/// Tauri.
pub fn parse_cli_args<I, S>(args: I) -> Result<CliArgs, CliParseError>
where
  I: IntoIterator<Item = S>,
  S: AsRef<str>,
{
  let mut args = args.into_iter().map(|arg| arg.as_ref().to_string());
  let mut parsed = CliArgs::default();

  while let Some(arg) = args.next() {
    if arg == EXTERNAL_COMPONENT_SETUP_FLAG {
      let id = args
        .next()
        .ok_or(CliParseError::MissingValue(EXTERNAL_COMPONENT_SETUP_FLAG))?;
      let component =
        component_from_cli_id(&id).ok_or(CliParseError::UnknownComponent(id))?;
      parsed.mode = Some(CliMode::ExternalComponentSetup { component });
    } else if arg == EXTERNAL_COMPONENT_NOTICE_FLAG {
      let notice = args
        .next()
        .ok_or(CliParseError::MissingValue(EXTERNAL_COMPONENT_NOTICE_FLAG))?;
      if notice != UNINSTALL_NOTICE {
        return Err(CliParseError::UnknownNotice(notice));
      }
      parsed.mode = Some(CliMode::ExternalComponentUninstallNotice);
    } else if arg == WAIT_FOR_PARENT_FLAG {
      let pid = args
        .next()
        .ok_or(CliParseError::MissingValue(WAIT_FOR_PARENT_FLAG))?;
      parsed.wait_for_parent = Some(
        pid
          .parse()
          .map_err(|_| CliParseError::InvalidParentPid(pid))?,
      );
    }
  }

  Ok(parsed)
}

/// The arguments for a relaunch of this process: its own arguments without
/// the program name and without a handoff flag from its own launch, so a
/// relaunch never waits on a parent that is long gone and whose id may
/// belong to another process by now.
pub fn relaunch_args() -> Vec<String> {
  without_wait_for_parent(std::env::args().skip(1))
}

/// The arguments for the elevated child of this process: [`relaunch_args`]
/// plus the handoff flag naming this process.
pub fn elevated_handoff_args() -> Vec<String> {
  with_wait_for_parent(relaunch_args(), std::process::id())
}

fn without_wait_for_parent<I: IntoIterator<Item = String>>(args: I) -> Vec<String> {
  let mut args = args.into_iter();
  let mut kept = Vec::new();
  while let Some(arg) = args.next() {
    if arg == WAIT_FOR_PARENT_FLAG {
      args.next();
      continue;
    }
    kept.push(arg);
  }
  kept
}

fn with_wait_for_parent(mut args: Vec<String>, parent_pid: u32) -> Vec<String> {
  args.push(WAIT_FOR_PARENT_FLAG.to_string());
  args.push(parent_pid.to_string());
  args
}

/// Wait for the parent named by [`WAIT_FOR_PARENT_FLAG`] to exit. Returns
/// the warning to log when the wait did not end with the parent's exit; the
/// caller logs it once the logger exists, because this runs before it does.
pub fn wait_for_parent_exit(parent_pid: u32) -> Result<(), String> {
  let waited = PlatformFactory::shared()
    .and_then(|platform| platform.wait_for_process_exit(parent_pid, PARENT_EXIT_TIMEOUT));
  match waited {
    Ok(ProcessExitWait::Exited) => Ok(()),
    Ok(ProcessExitWait::TimedOut) => Err(format!(
      "Elevated relaunch handoff: the parent process {parent_pid} did not exit within \
       {} s; starting anyway",
      PARENT_EXIT_TIMEOUT.as_secs()
    )),
    Err(e) => Err(format!(
      "Elevated relaunch handoff: could not wait for the parent process {parent_pid}: \
       {e}; starting anyway"
    )),
  }
}

/// Run a command-line mode to completion and return the process exit code.
pub fn run_cli_mode(mode: CliMode) -> i32 {
  match mode {
    CliMode::ExternalComponentSetup { component } => {
      // A panic must still become a meaningful exit code: the elevated child
      // has no console, so the default exit status 101 would be the only
      // trace. The default hook still prints the message to stderr for a
      // caller that redirected it.
      let outcome =
        match std::panic::catch_unwind(|| run_external_component_setup(component)) {
          Ok(result) => result.outcome,
          Err(payload) => ExternalComponentSetupOutcome::failed(
            SetupFailureStage::Panicked,
            panic_message(payload.as_ref()),
          ),
        };
      if let ExternalComponentSetupOutcome::Failed { stage, detail } = &outcome {
        eprintln!("external component setup failed at {stage:?}: {detail}");
      }
      outcome.exit_code()
    }
    CliMode::ExternalComponentUninstallNotice => {
      // Detection or UI trouble must never fail an uninstall.
      let kept = std::panic::catch_unwind(installed_setup_components).unwrap_or_default();
      if let Some(text) = uninstall_notice_text(&kept) {
        show_notice(&text);
      }
      0
    }
  }
}

fn component_display_name(component: ExternalComponent) -> &'static str {
  match component {
    ExternalComponent::Pawnio => "PawnIO",
    ExternalComponent::Smartctl => "smartctl",
  }
}

/// Components with a setup plan whose runtime Core reports as installed.
/// Unknown state is not reported as installed.
fn installed_setup_components() -> Vec<ExternalComponent> {
  let Ok(platform) = PlatformFactory::shared() else {
    return Vec::new();
  };
  SETUP_COMPONENTS
    .into_iter()
    .filter(|component| {
      setup_plan(*component).is_some_and(|plan| {
        matches!(
          platform.external_component_setup_status(plan).runtime,
          RuntimeInstallState::Installed { .. }
        )
      })
    })
    .collect()
}

fn uninstall_notice_text(kept: &[ExternalComponent]) -> Option<String> {
  if kept.is_empty() {
    return None;
  }
  let names = kept
    .iter()
    .map(|component| component_display_name(*component))
    .collect::<Vec<_>>()
    .join(", ");
  Some(format!(
    "{names} will stay installed.\n\n\
     Uninstalling HardwareVisualizer does not remove {names}, because other \
     applications can use it. To remove it, uninstall {names} from Settings > \
     Apps > Installed apps."
  ))
}

#[cfg(windows)]
fn show_notice(text: &str) {
  use windows::Win32::UI::WindowsAndMessaging::{
    MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MessageBoxW,
  };
  use windows::core::PCWSTR;

  let text = text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
  let caption = "HardwareVisualizer"
    .encode_utf16()
    .chain(Some(0))
    .collect::<Vec<_>>();
  unsafe {
    MessageBoxW(
      None,
      PCWSTR(text.as_ptr()),
      PCWSTR(caption.as_ptr()),
      MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST,
    );
  }
}

#[cfg(not(windows))]
fn show_notice(_text: &str) {}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
  payload
    .downcast_ref::<&str>()
    .map(|message| (*message).to_string())
    .or_else(|| payload.downcast_ref::<String>().cloned())
    .unwrap_or_else(|| "panic without a message".to_string())
}

fn run_external_component_setup(
  component: ExternalComponent,
) -> ExternalComponentSetupResult {
  let Some(plan) = setup_plan(component) else {
    return ExternalComponentSetupResult::failed(
      component,
      SetupFailureStage::Other,
      "this component has no External Component Setup plan",
    );
  };
  match PlatformFactory::shared() {
    Ok(platform) => platform.run_external_component_setup(plan),
    Err(e) => ExternalComponentSetupResult::failed(
      component,
      SetupFailureStage::Other,
      e.to_string(),
    ),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parse_cli_mode<const N: usize>(
    args: [&str; N],
  ) -> Result<Option<CliMode>, CliParseError> {
    parse_cli_args(args).map(|parsed| parsed.mode)
  }

  #[test]
  fn plain_launch_is_not_a_cli_mode() {
    assert_eq!(parse_cli_mode(["hardware-visualizer.exe"]), Ok(None));
    assert_eq!(
      parse_cli_mode(["hardware-visualizer.exe", "--some-tauri-flag"]),
      Ok(None)
    );
  }

  #[test]
  fn parses_external_component_setup() {
    let mode = parse_cli_mode([
      "hardware-visualizer.exe",
      "--external-component-setup",
      "pawnio",
    ])
    .unwrap();

    assert_eq!(
      mode,
      Some(CliMode::ExternalComponentSetup {
        component: ExternalComponent::Pawnio,
      })
    );
  }

  #[test]
  fn rejects_unknown_components_and_missing_values() {
    assert_eq!(
      parse_cli_mode(["exe", "--external-component-setup", "winring0"]),
      Err(CliParseError::UnknownComponent("winring0".to_string()))
    );
    assert_eq!(
      parse_cli_mode(["exe", "--external-component-setup"]),
      Err(CliParseError::MissingValue(EXTERNAL_COMPONENT_SETUP_FLAG))
    );
  }

  #[test]
  fn parses_the_uninstall_notice() {
    assert_eq!(
      parse_cli_mode(["exe", "--external-component-notice", "uninstall"]),
      Ok(Some(CliMode::ExternalComponentUninstallNotice))
    );
    assert_eq!(
      parse_cli_mode(["exe", "--external-component-notice", "install"]),
      Err(CliParseError::UnknownNotice("install".to_string()))
    );
    assert_eq!(
      parse_cli_mode(["exe", "--external-component-notice"]),
      Err(CliParseError::MissingValue(EXTERNAL_COMPONENT_NOTICE_FLAG))
    );
  }

  #[test]
  fn parses_the_parent_to_wait_for_beside_a_normal_launch() {
    assert_eq!(
      parse_cli_args(["exe", "--wait-for-parent", "4242"]),
      Ok(CliArgs {
        mode: None,
        wait_for_parent: Some(4242),
      })
    );
    assert_eq!(
      parse_cli_args(["exe", "--some-tauri-flag", "--wait-for-parent", "7"]),
      Ok(CliArgs {
        mode: None,
        wait_for_parent: Some(7),
      })
    );
  }

  #[test]
  fn rejects_a_missing_or_invalid_parent_pid() {
    assert_eq!(
      parse_cli_args(["exe", "--wait-for-parent"]),
      Err(CliParseError::MissingValue(WAIT_FOR_PARENT_FLAG))
    );
    assert_eq!(
      parse_cli_args(["exe", "--wait-for-parent", "parent"]),
      Err(CliParseError::InvalidParentPid("parent".to_string()))
    );
    assert_eq!(
      parse_cli_args(["exe", "--wait-for-parent", "-1"]),
      Err(CliParseError::InvalidParentPid("-1".to_string()))
    );
  }

  #[test]
  fn handoff_args_replace_the_parent_of_the_current_launch() {
    let launched_with = vec![
      "--some-tauri-flag".to_string(),
      "--wait-for-parent".to_string(),
      "100".to_string(),
    ];

    let relaunch = without_wait_for_parent(launched_with.clone());
    assert_eq!(relaunch, vec!["--some-tauri-flag".to_string()]);

    assert_eq!(
      with_wait_for_parent(relaunch, 200),
      vec![
        "--some-tauri-flag".to_string(),
        "--wait-for-parent".to_string(),
        "200".to_string(),
      ]
    );
    assert_eq!(
      without_wait_for_parent(vec!["--wait-for-parent".to_string()]),
      Vec::<String>::new()
    );
  }

  #[test]
  fn uninstall_notice_names_only_installed_components() {
    assert_eq!(uninstall_notice_text(&[]), None);

    let text = uninstall_notice_text(&[ExternalComponent::Pawnio])
      .expect("an installed component produces a notice");
    assert!(text.starts_with("PawnIO will stay installed."));
    assert!(text.contains("uninstall PawnIO from Settings"));
  }

  #[test]
  fn cli_ids_round_trip() {
    for component in [ExternalComponent::Pawnio, ExternalComponent::Smartctl] {
      assert_eq!(
        component_from_cli_id(component_cli_id(component)),
        Some(component)
      );
    }
  }

  #[test]
  fn panic_messages_are_extracted_from_str_and_string_payloads() {
    let str_payload: Box<dyn std::any::Any + Send> = Box::new("boom");
    assert_eq!(panic_message(str_payload.as_ref()), "boom");
    let string_payload: Box<dyn std::any::Any + Send> = Box::new("bang".to_string());
    assert_eq!(panic_message(string_payload.as_ref()), "bang");
    let other: Box<dyn std::any::Any + Send> = Box::new(7u8);
    assert_eq!(panic_message(other.as_ref()), "panic without a message");
  }

  #[test]
  fn a_component_without_a_plan_exits_with_the_generic_failure_code() {
    assert_eq!(
      run_cli_mode(CliMode::ExternalComponentSetup {
        component: ExternalComponent::Smartctl,
      }),
      SetupFailureStage::Other.exit_code()
    );
  }
}
