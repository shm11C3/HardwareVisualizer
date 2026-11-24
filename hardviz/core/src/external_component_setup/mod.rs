//! External Component Setup: the explicit, user-initiated installation of an
//! optional external component from its pinned upstream release.
//!
//! Core owns the component catalog (pinned artifact URLs, sizes, digests,
//! module file names, installer switches), the verification rules, and the
//! OS-level setup steps behind the platform boundary. App owns the entry
//! points (command-line mode, IPC, installer custom actions) and the UI.
//!
//! The setup process reports back through its exit code only. A result file
//! or pipe in a user-writable location would let a same-user medium-integrity
//! process redirect an elevated write or forge the result; the exit code of a
//! process handle the caller owns cannot be forged.
//!
//! See `docs/adr/0024-external-component-setup.md`,
//! `docs/adr/0026-refresh-outdated-external-component-files.md`, and
//! `docs/design/external-component-setup.md`.

use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::models::ExternalComponent;

#[cfg(target_os = "windows")]
pub mod windows;

/// A release asset pinned by version, size, and SHA-256 digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinnedArtifact {
  pub version: &'static str,
  pub file_name: &'static str,
  pub url: &'static str,
  pub size: u64,
  pub sha256_hex: &'static str,
}

/// The runtime installer step of a setup plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallerStep {
  pub artifact: PinnedArtifact,
  /// Arguments for an unattended install. The installer elevates itself, so
  /// the process running it must already be elevated to avoid a second prompt.
  pub unattended_args: &'static [&'static str],
  /// Exit codes that mean "installed, restart Windows to finish".
  pub reboot_required_exit_codes: &'static [i32],
}

/// One file of a [`FileBundleStep`], with the digest of its contents in the
/// pinned release and in every earlier upstream release that shipped it
/// (ADR 0026). A later release is deliberately not listed, so a newer file
/// classifies as unrecognized and is never rolled back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleFile {
  pub file_name: &'static str,
  pub sha256_hex: &'static str,
  pub earlier_sha256_hex: &'static [&'static str],
}

impl ModuleFile {
  pub fn classify(&self, contents: &[u8]) -> ModuleFileCondition {
    self.condition_of_digest(&sha256_hex(contents))
  }

  pub fn condition_of_digest(&self, digest_hex: &str) -> ModuleFileCondition {
    if digest_hex.eq_ignore_ascii_case(self.sha256_hex) {
      ModuleFileCondition::Current
    } else if self
      .earlier_sha256_hex
      .iter()
      .any(|earlier| digest_hex.eq_ignore_ascii_case(earlier))
    {
      ModuleFileCondition::Outdated
    } else {
      ModuleFileCondition::Unrecognized
    }
  }
}

/// The module-file step of a setup plan: a zip archive whose listed entries
/// are placed under the component install location when missing, and which
/// replace a present file whose contents match an earlier release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileBundleStep {
  pub artifact: PinnedArtifact,
  pub files: &'static [ModuleFile],
}

impl FileBundleStep {
  pub fn file_names(&self) -> impl Iterator<Item = &'static str> {
    self.files.iter().map(|file| file.file_name)
  }

  pub fn file(&self, file_name: &str) -> Option<&'static ModuleFile> {
    self.files.iter().find(|file| file.file_name == file_name)
  }
}

/// Everything Core needs to set up one component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalComponentSetupPlan {
  pub component: ExternalComponent,
  pub installer: InstallerStep,
  pub file_bundle: FileBundleStep,
}

/// PawnIO runtime 2.2.0 from the PawnIO.Setup release. The digest was
/// computed from the downloaded asset on 2026-09-13 and matches the winget
/// `namazso.PawnIO` 2.2.0 manifest.
const PAWNIO_RUNTIME: PinnedArtifact = PinnedArtifact {
  version: "2.2.0",
  file_name: "PawnIO_setup.exe",
  url: "https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe",
  size: 3_410_960,
  sha256_hex: "1f519a22e47187f70a1379a48ca604981c4fcf694f4e65b734aaa74a9fba3032",
};

/// PawnIO.Modules 0.2.11: the tag the sensor specification
/// (`docs/specs/sensors/pawnio-interface.md`) verified its IOCTL facts
/// against. Move this pin together with the specification, not ahead of it.
/// The digest was computed from the downloaded asset on 2026-09-26 and
/// matches the digest GitHub reports for the release asset.
const PAWNIO_MODULES: PinnedArtifact = PinnedArtifact {
  version: "0.2.11",
  file_name: "release_0_2_11.zip",
  url: "https://github.com/namazso/PawnIO.Modules/releases/download/0.2.11/release_0_2_11.zip",
  size: 69_582,
  sha256_hex: "43608cb89bc84247fef1368a139013f7d043e17db6d6c8dfc9b46bf0905a81f4",
};

/// Signed module blobs the Windows providers can load. Names match the
/// provider's production file names; unsigned `.amx` fallbacks are not
/// installed because the production driver rejects them.
///
/// Digests were computed on 2026-09-26 from the files in the
/// `release_<version>.zip` asset of every PawnIO.Modules release from 0.1.0
/// to the pinned one. Earlier digests are listed oldest first with the
/// releases that shipped them; a release that left a file unchanged adds no
/// entry. When the pin moves, the old current digest joins the earlier list.
const PAWNIO_MODULE_FILES: &[ModuleFile] = &[
  ModuleFile {
    file_name: "IntelMSR.bin",
    // 0.2.10-0.2.11
    sha256_hex: "d6ed85d65ab17a22f813ef98207d6d537155ee2ded5976a21cb48413c9b92e5f",
    earlier_sha256_hex: &[
      "3a12e321e219e27c646e12c294a0ca26fc815166816889933c1063f88e437594", // 0.1.0-0.1.2
      "f3ab69f0a2686813de2f47efcda6271e176c4ee50fab5f107254f7d6a4e77361", // 0.1.3-0.1.6
      "0dba915f95b5c6a084bf6c75c535233c90e79c154d49a0da1afb4d64215dcf7d", // 0.2.0
      "d09fa2d4232f92d9902fc90b058adc55ae5469b9b6f2f3f1441184796945bad1", // 0.2.1
      "ed41a0d0de082f4668a9caccb9681ebbad97a8ed3452c41918dda1eb454b77a9", // 0.2.2-0.2.3
      "74508721ede84765e53dc3533ac23283b89fa328275e0079d9508b22b6677ea4", // 0.2.4
      "344cb99053883e42876ac3194fb0336b0c00795e78c46dd651f8b75625f17e78", // 0.2.5-0.2.6
      "5bfda87500160076158befc77300f81a388d72b11df2c794a3939ebe66777098", // 0.2.7-0.2.9
    ],
  },
  ModuleFile {
    file_name: "RyzenSMU.bin",
    // 0.2.11
    sha256_hex: "301d9ca397108e09f31bfbd5ac4c9bb4f352a5de68532c32db3ba7ddcde93450",
    earlier_sha256_hex: &[
      "8642d4b11287f4968f7a9186339af5c381f681289778dc7893e7915023f76d8b", // 0.1.0-0.1.2
      "5f4c157935b70f542c653e4e54bed4f419b1268eee5a65b1966a977c57fac773", // 0.1.3-0.1.6
      "1d29404b02b4247ddb27544638e38586ce19f52c0774dd6e4a17ed7a2fd1eca8", // 0.2.0
      "8cec3a2d03b19d585fd75e36be2875fde8825834968509582d4351201dc2871a", // 0.2.1
      "0cf0fe1296c5c38f4bee0f96352b35f14d32ab97cb58fd17600646d98507d8aa", // 0.2.2
      "c505fdaf67d3dccca1c39c91cc69ccab4b4a99bebe8b13d3e7632bf7876df965", // 0.2.3
      "b84eca7f32c63b3d8c14b2c6d45482706df8683aa6f43eb8bead9dc62181d38f", // 0.2.4-0.2.6
      "dad38b36a08e2da982d4397619aee7264e32be088ea6bcb781b066adfb00efc0", // 0.2.7-0.2.9
      "54da61c2653ed0afabc20d1349636023cb90e7582c4ee4ab93fa77d673e33f26", // 0.2.10
    ],
  },
  ModuleFile {
    file_name: "AMDFamily17.bin",
    // 0.2.10-0.2.11
    sha256_hex: "dae74615761b78bdf064dfb3e136252ddcc6fc727d88f14738d0e5800d427a91",
    earlier_sha256_hex: &[
      "cd59598344b54a23178ccf9223239f60a3111bd86ff2c7bea2c9d1cef89e095b", // 0.1.3-0.1.6
      "18084c329a9b674571ad20a1844b0d3032b044261a43750e5c1dec18075bb514", // 0.2.0
      "374d4bc3e88284d08f2c65e292df5340c6a034affc30b614db6e780d7094d117", // 0.2.1
      "099dc01d6db97ea997fec4a461e191cc64b9d7ce47c9d2153c451c56c2adcf50", // 0.2.2-0.2.9
    ],
  },
  ModuleFile {
    file_name: "LpcIO.bin",
    // 0.2.10-0.2.11
    sha256_hex: "b3896a1cab0d808fca31fe2ebcae045d59dac690da87b17c858bb8da357eb45e",
    earlier_sha256_hex: &[
      "e872948accba4287b7f231860f9a7569eb530238e02f89dc01347b1247dbd388", // 0.1.0-0.1.2
      "b7c9ecd2a4c044b2c55d10e5a2a02b1fcac5322804fd4aee697e8d393c892d49", // 0.1.3-0.1.5
      "3dcf8b2bc80ff642d97c4608511a818642b5bf315ff53df3df393d043e71d101", // 0.1.6-0.2.7
      "4247d588b9da9c598a65c6f5b8255a90bedee510ee1c354f9a909a2f959cd4df", // 0.2.8-0.2.9
    ],
  },
];

/// `ERROR_SUCCESS_REBOOT_REQUIRED`: the PawnIO installer returns it in silent
/// mode when the driver install needs a restart (PawnIO.Setup 2.2.0 notes).
/// The setup process reuses it as its own exit code for the same meaning.
const ERROR_SUCCESS_REBOOT_REQUIRED: i32 = 3010;

const PAWNIO_SETUP_PLAN: ExternalComponentSetupPlan = ExternalComponentSetupPlan {
  component: ExternalComponent::Pawnio,
  installer: InstallerStep {
    artifact: PAWNIO_RUNTIME,
    unattended_args: &["-install", "-silent"],
    reboot_required_exit_codes: &[ERROR_SUCCESS_REBOOT_REQUIRED],
  },
  file_bundle: FileBundleStep {
    artifact: PAWNIO_MODULES,
    files: PAWNIO_MODULE_FILES,
  },
};

/// The setup plan for a component, or `None` when HardwareVisualizer does not
/// offer setup for it (for example `smartctl`, which is a system package).
pub fn setup_plan(
  component: ExternalComponent,
) -> Option<&'static ExternalComponentSetupPlan> {
  match component {
    ExternalComponent::Pawnio => Some(&PAWNIO_SETUP_PLAN),
    ExternalComponent::Smartctl => None,
  }
}

/// Components that have a setup plan, in display order.
pub fn components_with_setup() -> Vec<ExternalComponent> {
  [ExternalComponent::Pawnio, ExternalComponent::Smartctl]
    .into_iter()
    .filter(|component| setup_plan(*component).is_some())
    .collect()
}

/// Whether the component runtime is installed on this machine.
///
/// `Unknown` preserves enumeration uncertainty: a registry or file-system
/// failure is not evidence of absence and must not start an install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeInstallState {
  NotInstalled,
  Installed {
    version: Option<String>,
    install_location: Option<PathBuf>,
  },
  Unknown {
    detail: String,
  },
}

/// What a module file on disk is, compared with the pinned catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleFileCondition {
  Missing,
  /// Matches the pinned release.
  Current,
  /// Matches only an earlier release; setup replaces it.
  Outdated,
  /// Any other contents, including a release newer than the pin. Setup
  /// leaves it alone and counts it as present.
  Unrecognized,
}

/// One module file the plan can place, as found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleFileState {
  pub file_name: String,
  pub condition: ModuleFileCondition,
}

impl ModuleFileState {
  pub fn is_present(&self) -> bool {
    self.condition != ModuleFileCondition::Missing
  }
}

/// Why setup is not offered on this platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalComponentSetupSupport {
  Supported,
  UnsupportedPlatform,
}

/// The current state of one component as seen by the setup plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalComponentSetupStatus {
  pub component: ExternalComponent,
  pub support: ExternalComponentSetupSupport,
  pub runtime: RuntimeInstallState,
  pub module_files: Vec<ModuleFileState>,
  /// Set when module-file enumeration failed somewhere, so `module_files`
  /// may under-report presence. Setup refuses to run while this is set.
  pub enumeration_error: Option<String>,
  pub pinned_runtime_version: String,
  pub pinned_modules_version: String,
}

impl ExternalComponentSetupStatus {
  pub fn unsupported_platform(plan: &ExternalComponentSetupPlan) -> Self {
    Self {
      component: plan.component,
      support: ExternalComponentSetupSupport::UnsupportedPlatform,
      runtime: RuntimeInstallState::NotInstalled,
      module_files: plan
        .file_bundle
        .file_names()
        .map(|file_name| ModuleFileState {
          file_name: file_name.to_string(),
          condition: ModuleFileCondition::Missing,
        })
        .collect(),
      enumeration_error: None,
      pinned_runtime_version: plan.installer.artifact.version.to_string(),
      pinned_modules_version: plan.file_bundle.artifact.version.to_string(),
    }
  }

  /// True when the runtime is installed and every module file is present and
  /// not outdated, so setup has nothing left to do. Uncertain enumeration is
  /// never complete.
  pub fn is_complete(&self) -> bool {
    matches!(self.runtime, RuntimeInstallState::Installed { .. })
      && self.enumeration_error.is_none()
      && self.module_files.iter().all(|file| {
        matches!(
          file.condition,
          ModuleFileCondition::Current | ModuleFileCondition::Unrecognized
        )
      })
  }

  /// Why setup must not run right now, if the state is not trustworthy.
  pub fn setup_blocker(&self) -> Option<String> {
    if self.support != ExternalComponentSetupSupport::Supported {
      return Some("External Component Setup is available on Windows only".to_string());
    }
    if let RuntimeInstallState::Unknown { detail } = &self.runtime {
      return Some(format!("runtime state is unknown: {detail}"));
    }
    self
      .enumeration_error
      .as_ref()
      .map(|detail| format!("module file state is unknown: {detail}"))
  }

  pub fn missing_module_files(&self) -> Vec<&str> {
    self.module_files_in(ModuleFileCondition::Missing)
  }

  pub fn outdated_module_files(&self) -> Vec<&str> {
    self.module_files_in(ModuleFileCondition::Outdated)
  }

  fn module_files_in(&self, condition: ModuleFileCondition) -> Vec<&str> {
    self
      .module_files
      .iter()
      .filter(|file| file.condition == condition)
      .map(|file| file.file_name.as_str())
      .collect()
  }
}

/// Where a setup run stopped. Carried in the setup process exit code so the
/// caller can explain the failure without a writable result channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupFailureStage {
  /// The current state could not be determined, so nothing was attempted.
  StateUnknown,
  /// The administrator-only staging directory could not be created.
  StagingDirectory,
  DownloadRuntime,
  VerifyRuntime,
  StartInstaller,
  InstallerExit,
  DownloadModules,
  VerifyModules,
  /// The verified archive does not contain a required module file.
  ArchiveContents,
  PlaceModules,
  /// Every step ran, but the component is still not complete.
  Incomplete,
  UnsupportedPlatform,
  /// The setup process panicked; the message went to its stderr only.
  Panicked,
  /// The runtime installer did not exit within its time limit and was
  /// terminated; the runtime may be partially installed.
  InstallerTimedOut,
  /// The runtime installer did not exit within its time limit and could not
  /// be confirmed terminated, so it may still be running; the staging
  /// directory was left in place for it.
  InstallerStillRunning,
  /// A failure without a more specific stage, or an unrecognized exit code.
  Other,
}

impl SetupFailureStage {
  const ALL: [Self; 16] = [
    Self::StateUnknown,
    Self::StagingDirectory,
    Self::DownloadRuntime,
    Self::VerifyRuntime,
    Self::StartInstaller,
    Self::InstallerExit,
    Self::DownloadModules,
    Self::VerifyModules,
    Self::ArchiveContents,
    Self::PlaceModules,
    Self::Incomplete,
    Self::UnsupportedPlatform,
    Self::Panicked,
    Self::InstallerTimedOut,
    Self::InstallerStillRunning,
    Self::Other,
  ];

  /// Exit code of the setup process for this stage. `Other` is the generic
  /// failure code; the specific stages start at 10 so they never collide with
  /// conventional codes or `ERROR_SUCCESS_REBOOT_REQUIRED`.
  pub fn exit_code(self) -> i32 {
    match self {
      Self::Other => 1,
      Self::StateUnknown => 10,
      Self::StagingDirectory => 11,
      Self::DownloadRuntime => 12,
      Self::VerifyRuntime => 13,
      Self::StartInstaller => 14,
      Self::InstallerExit => 15,
      Self::DownloadModules => 16,
      Self::VerifyModules => 17,
      Self::ArchiveContents => 18,
      Self::PlaceModules => 19,
      Self::Incomplete => 20,
      Self::UnsupportedPlatform => 21,
      Self::Panicked => 22,
      Self::InstallerTimedOut => 23,
      Self::InstallerStillRunning => 24,
    }
  }

  pub fn from_exit_code(code: i32) -> Option<Self> {
    Self::ALL
      .into_iter()
      .find(|stage| stage.exit_code() == code)
  }
}

/// The outcome of running a setup plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalComponentSetupOutcome {
  /// Nothing was missing, nothing was changed.
  AlreadyInstalled,
  /// The plan completed and the component is usable after the app restarts.
  Installed,
  /// The plan completed but Windows must restart before the driver is usable.
  RebootRequired,
  /// The plan stopped; the component may be partially set up.
  Failed {
    stage: SetupFailureStage,
    detail: String,
  },
}

impl ExternalComponentSetupOutcome {
  pub fn failed(stage: SetupFailureStage, detail: impl Into<String>) -> Self {
    Self::Failed {
      stage,
      detail: detail.into(),
    }
  }

  /// Process exit code of the setup command-line mode.
  pub fn exit_code(&self) -> i32 {
    match self {
      Self::AlreadyInstalled | Self::Installed => 0,
      Self::RebootRequired => ERROR_SUCCESS_REBOOT_REQUIRED,
      Self::Failed { stage, .. } => stage.exit_code(),
    }
  }

  /// Reconstruct the outcome the caller can trust from the exit code of the
  /// setup process it launched. `0` is reported as `Installed`; the caller
  /// short-circuits an already complete component before launching.
  pub fn from_exit_code(exit_code: Option<i32>) -> Self {
    match exit_code {
      Some(0) => Self::Installed,
      Some(ERROR_SUCCESS_REBOOT_REQUIRED) => Self::RebootRequired,
      Some(code) => Self::Failed {
        stage: SetupFailureStage::from_exit_code(code)
          .unwrap_or(SetupFailureStage::Other),
        detail: format!("the setup process exited with code {code}"),
      },
      None => Self::Failed {
        stage: SetupFailureStage::Other,
        detail: "the setup process exited without an exit code".to_string(),
      },
    }
  }
}

/// What a setup run did, as known inside the setup process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalComponentSetupResult {
  pub component: ExternalComponent,
  pub outcome: ExternalComponentSetupOutcome,
  pub runtime_installed: bool,
  pub module_files_placed: Vec<String>,
  /// Outdated module files replaced with the pinned release (ADR 0026).
  pub module_files_replaced: Vec<String>,
}

impl ExternalComponentSetupResult {
  pub fn failed(
    component: ExternalComponent,
    stage: SetupFailureStage,
    detail: impl Into<String>,
  ) -> Self {
    Self {
      component,
      outcome: ExternalComponentSetupOutcome::failed(stage, detail),
      runtime_installed: false,
      module_files_placed: Vec::new(),
      module_files_replaced: Vec::new(),
    }
  }

  pub fn exit_code(&self) -> i32 {
    self.outcome.exit_code()
  }
}

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
  Sha256::digest(bytes)
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect()
}

/// Check a downloaded artifact against its pinned size and digest.
pub fn verify_artifact(bytes: &[u8], artifact: &PinnedArtifact) -> Result<(), String> {
  if bytes.len() as u64 != artifact.size {
    return Err(format!(
      "{} size mismatch: expected {} bytes, downloaded {} bytes",
      artifact.file_name,
      artifact.size,
      bytes.len()
    ));
  }

  let digest_hex = sha256_hex(bytes);
  if !digest_hex.eq_ignore_ascii_case(artifact.sha256_hex) {
    return Err(format!(
      "{} SHA-256 mismatch: expected {}, downloaded {}",
      artifact.file_name, artifact.sha256_hex, digest_hex
    ));
  }

  Ok(())
}

/// What an installer exit status means for the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerExitOutcome {
  Installed,
  RebootRequired,
  Failed(Option<i32>),
}

pub fn interpret_installer_exit_code(
  step: &InstallerStep,
  exit_code: Option<i32>,
) -> InstallerExitOutcome {
  match exit_code {
    Some(0) => InstallerExitOutcome::Installed,
    Some(code) if step.reboot_required_exit_codes.contains(&code) => {
      InstallerExitOutcome::RebootRequired
    }
    other => InstallerExitOutcome::Failed(other),
  }
}

/// Map archive entry names to the missing module files they satisfy.
///
/// Matching uses the entry's base name, case-insensitively, so an archive
/// that nests files in a directory still resolves. Returns
/// `(entry_name, file_name)` pairs in the order of `missing`.
pub fn select_bundle_entries<'a>(
  entry_names: &'a [String],
  missing: &[&'a str],
) -> Vec<(&'a str, &'a str)> {
  missing
    .iter()
    .filter_map(|file_name| {
      entry_names
        .iter()
        .find(|entry| {
          entry
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|base| base.eq_ignore_ascii_case(file_name))
        })
        .map(|entry| (entry.as_str(), *file_name))
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pawnio_has_a_setup_plan_and_smartctl_does_not() {
    assert!(setup_plan(ExternalComponent::Pawnio).is_some());
    assert!(setup_plan(ExternalComponent::Smartctl).is_none());
    assert_eq!(components_with_setup(), vec![ExternalComponent::Pawnio]);
  }

  #[test]
  fn pawnio_plan_pins_upstream_release_assets() {
    let plan = setup_plan(ExternalComponent::Pawnio).unwrap();

    assert_eq!(plan.installer.artifact.version, "2.2.0");
    assert!(
      plan
        .installer
        .artifact
        .url
        .starts_with("https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/")
    );
    assert_eq!(plan.installer.unattended_args, &["-install", "-silent"]);
    assert_eq!(plan.file_bundle.artifact.version, "0.2.11");
    assert_eq!(
      plan.file_bundle.file_names().collect::<Vec<_>>(),
      vec![
        "IntelMSR.bin",
        "RyzenSMU.bin",
        "AMDFamily17.bin",
        "LpcIO.bin"
      ]
    );
    assert_eq!(plan.installer.artifact.sha256_hex.len(), 64);
    assert_eq!(plan.file_bundle.artifact.sha256_hex.len(), 64);
  }

  #[test]
  fn verify_artifact_accepts_matching_size_and_digest() {
    let bytes = b"hello world";
    let artifact = PinnedArtifact {
      version: "1",
      file_name: "hello.txt",
      url: "https://example.invalid/hello.txt",
      size: 11,
      sha256_hex: "B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9",
    };

    assert_eq!(verify_artifact(bytes, &artifact), Ok(()));
  }

  #[test]
  fn verify_artifact_rejects_size_and_digest_mismatches() {
    let artifact = PinnedArtifact {
      version: "1",
      file_name: "hello.txt",
      url: "https://example.invalid/hello.txt",
      size: 11,
      sha256_hex: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
    };

    let size_error = verify_artifact(b"hello", &artifact).unwrap_err();
    assert!(size_error.contains("size mismatch"), "{size_error}");

    let digest_error = verify_artifact(b"hello worle", &artifact).unwrap_err();
    assert!(digest_error.contains("SHA-256 mismatch"), "{digest_error}");
  }

  #[test]
  fn installer_exit_codes_map_to_plan_outcomes() {
    let step = setup_plan(ExternalComponent::Pawnio).unwrap().installer;

    assert_eq!(
      interpret_installer_exit_code(&step, Some(0)),
      InstallerExitOutcome::Installed
    );
    assert_eq!(
      interpret_installer_exit_code(&step, Some(3010)),
      InstallerExitOutcome::RebootRequired
    );
    assert_eq!(
      interpret_installer_exit_code(&step, Some(5)),
      InstallerExitOutcome::Failed(Some(5))
    );
    assert_eq!(
      interpret_installer_exit_code(&step, None),
      InstallerExitOutcome::Failed(None)
    );
  }

  #[test]
  fn select_bundle_entries_matches_base_names_case_insensitively() {
    let entries = vec![
      "COPYING".to_string(),
      "nested/intelmsr.bin".to_string(),
      "LpcIO.bin".to_string(),
    ];

    let selected =
      select_bundle_entries(&entries, &["IntelMSR.bin", "RyzenSMU.bin", "LpcIO.bin"]);

    assert_eq!(
      selected,
      vec![
        ("nested/intelmsr.bin", "IntelMSR.bin"),
        ("LpcIO.bin", "LpcIO.bin")
      ]
    );
  }

  fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
      && value
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
  }

  #[test]
  fn module_catalog_pins_lowercase_digests_and_keeps_releases_apart() {
    let plan = setup_plan(ExternalComponent::Pawnio).unwrap();

    for file in plan.file_bundle.files {
      assert!(is_sha256_hex(file.sha256_hex), "{}", file.file_name);
      for (index, earlier) in file.earlier_sha256_hex.iter().enumerate() {
        assert!(is_sha256_hex(earlier), "{} {earlier}", file.file_name);
        assert_ne!(*earlier, file.sha256_hex, "{}", file.file_name);
        assert!(
          !file.earlier_sha256_hex[..index].contains(earlier),
          "{} lists {earlier} twice",
          file.file_name
        );
      }
    }
  }

  #[test]
  fn module_file_contents_classify_against_the_catalog() {
    let file = ModuleFile {
      file_name: "Example.bin",
      sha256_hex: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
      earlier_sha256_hex: &[
        "5eb63bbbe01eeed093cb22bb8f5acdc3c49b31f3e03c4f0e0e1cdc1bb1bb1d6b",
      ],
    };

    assert_eq!(file.classify(b"hello world"), ModuleFileCondition::Current);
    assert_eq!(
      file.classify(b"hello worle"),
      ModuleFileCondition::Unrecognized
    );
    assert_eq!(sha256_hex(b"hello world"), file.sha256_hex);
  }

  #[test]
  fn an_earlier_release_digest_is_outdated() {
    let plan = setup_plan(ExternalComponent::Pawnio).unwrap();
    let file = plan.file_bundle.files[0];
    let earlier = file.earlier_sha256_hex[0];

    assert_eq!(
      file.condition_of_digest(earlier),
      ModuleFileCondition::Outdated
    );
    assert_eq!(
      file.condition_of_digest(&earlier.to_uppercase()),
      ModuleFileCondition::Outdated
    );
    assert_eq!(
      file.condition_of_digest(file.sha256_hex),
      ModuleFileCondition::Current
    );
  }

  fn installed_status(
    conditions: [ModuleFileCondition; 4],
  ) -> ExternalComponentSetupStatus {
    let plan = setup_plan(ExternalComponent::Pawnio).unwrap();
    let mut status = ExternalComponentSetupStatus::unsupported_platform(plan);
    status.support = ExternalComponentSetupSupport::Supported;
    status.runtime = RuntimeInstallState::Installed {
      version: Some("2.2.0".to_string()),
      install_location: None,
    };
    for (file, condition) in status.module_files.iter_mut().zip(conditions) {
      file.condition = condition;
    }
    status
  }

  #[test]
  fn an_outdated_file_leaves_the_component_incomplete() {
    use ModuleFileCondition::{Current, Outdated, Unrecognized};

    let status = installed_status([Current, Outdated, Unrecognized, Current]);

    assert!(!status.is_complete());
    assert!(status.missing_module_files().is_empty());
    assert_eq!(status.outdated_module_files(), vec!["RyzenSMU.bin"]);
    assert_eq!(status.setup_blocker(), None);
  }

  #[test]
  fn an_unrecognized_file_counts_as_complete_and_is_never_outdated() {
    use ModuleFileCondition::{Current, Unrecognized};

    let status = installed_status([Current, Unrecognized, Unrecognized, Current]);

    assert!(status.is_complete());
    assert!(status.outdated_module_files().is_empty());
    assert!(status.module_files.iter().all(ModuleFileState::is_present));
  }

  #[test]
  fn status_reports_completeness_and_missing_files() {
    let plan = setup_plan(ExternalComponent::Pawnio).unwrap();
    let mut status = ExternalComponentSetupStatus::unsupported_platform(plan);
    assert!(!status.is_complete());
    assert_eq!(status.missing_module_files().len(), 4);
    assert!(status.setup_blocker().is_some());

    status.support = ExternalComponentSetupSupport::Supported;
    status.runtime = RuntimeInstallState::Installed {
      version: Some("2.2.0".to_string()),
      install_location: None,
    };
    for file in &mut status.module_files {
      file.condition = ModuleFileCondition::Current;
    }
    assert!(status.is_complete());
    assert!(status.missing_module_files().is_empty());
    assert_eq!(status.setup_blocker(), None);
  }

  #[test]
  fn uncertain_state_blocks_setup_and_is_never_complete() {
    let plan = setup_plan(ExternalComponent::Pawnio).unwrap();
    let mut status = ExternalComponentSetupStatus::unsupported_platform(plan);
    status.support = ExternalComponentSetupSupport::Supported;
    status.runtime = RuntimeInstallState::Installed {
      version: None,
      install_location: None,
    };
    for file in &mut status.module_files {
      file.condition = ModuleFileCondition::Current;
    }

    status.enumeration_error = Some("access denied".to_string());
    assert!(!status.is_complete());
    assert!(status.setup_blocker().unwrap().contains("access denied"));

    status.enumeration_error = None;
    status.runtime = RuntimeInstallState::Unknown {
      detail: "registry query failed".to_string(),
    };
    assert!(!status.is_complete());
    assert!(
      status
        .setup_blocker()
        .unwrap()
        .contains("registry query failed")
    );
  }

  #[test]
  fn failure_stages_round_trip_through_exit_codes() {
    for stage in SetupFailureStage::ALL {
      assert_eq!(
        SetupFailureStage::from_exit_code(stage.exit_code()),
        Some(stage)
      );
      assert_ne!(stage.exit_code(), 0);
      assert_ne!(stage.exit_code(), ERROR_SUCCESS_REBOOT_REQUIRED);
    }
    assert_eq!(SetupFailureStage::from_exit_code(99), None);
  }

  #[test]
  fn installer_timeout_has_its_own_exit_code() {
    assert_eq!(SetupFailureStage::InstallerTimedOut.exit_code(), 23);
    assert_eq!(
      SetupFailureStage::from_exit_code(23),
      Some(SetupFailureStage::InstallerTimedOut)
    );
  }

  #[test]
  fn an_unconfirmed_installer_stop_has_its_own_exit_code() {
    assert_eq!(SetupFailureStage::InstallerStillRunning.exit_code(), 24);
    assert_eq!(
      SetupFailureStage::from_exit_code(24),
      Some(SetupFailureStage::InstallerStillRunning)
    );
  }

  #[test]
  fn outcome_exit_codes_reconstruct_the_outcome() {
    let installed = ExternalComponentSetupOutcome::Installed;
    assert_eq!(
      ExternalComponentSetupOutcome::from_exit_code(Some(installed.exit_code())),
      installed
    );

    let reboot = ExternalComponentSetupOutcome::RebootRequired;
    assert_eq!(
      ExternalComponentSetupOutcome::from_exit_code(Some(reboot.exit_code())),
      reboot
    );

    let failed =
      ExternalComponentSetupOutcome::failed(SetupFailureStage::VerifyRuntime, "digest");
    match ExternalComponentSetupOutcome::from_exit_code(Some(failed.exit_code())) {
      ExternalComponentSetupOutcome::Failed { stage, detail } => {
        assert_eq!(stage, SetupFailureStage::VerifyRuntime);
        assert!(detail.contains("13"), "{detail}");
      }
      other => panic!("unexpected {other:?}"),
    }

    assert!(matches!(
      ExternalComponentSetupOutcome::from_exit_code(None),
      ExternalComponentSetupOutcome::Failed {
        stage: SetupFailureStage::Other,
        ..
      }
    ));
    assert!(matches!(
      ExternalComponentSetupOutcome::from_exit_code(Some(99)),
      ExternalComponentSetupOutcome::Failed {
        stage: SetupFailureStage::Other,
        ..
      }
    ));
  }

  #[test]
  fn result_exit_code_follows_the_outcome() {
    let result = ExternalComponentSetupResult::failed(
      ExternalComponent::Pawnio,
      SetupFailureStage::Incomplete,
      "x",
    );
    assert_eq!(result.exit_code(), 20);
  }
}
