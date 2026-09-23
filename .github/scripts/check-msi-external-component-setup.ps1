#!/usr/bin/env pwsh
# Assert that a built MSI carries the External Component Setup option (#2118)
# with the consent rules from ADR 0024:
#
# - the setup custom action is deferred, runs without impersonation, ignores
#   its exit code, and runs only when EXTERNAL_COMPONENT_PAWNIO = "1" and the
#   install directory is under Program Files;
# - EXTERNAL_COMPONENT_PAWNIO has no default in the Property table and is only
#   defaulted by the UI sequence, so /qn, /passive, and winget run no setup;
# - the options dialog is inserted between InstallDirDlg and VerifyReadyDlg.
#
# The interactive behaviour still needs a manual run on Windows; this check
# catches a fragment that silently stopped linking or a template change that
# broke the dialog chain.

param(
  [Parameter(Mandatory = $true, Position = 0)]
  [string]$MsiPath
)

$ErrorActionPreference = "Stop"

$property = "EXTERNAL_COMPONENT_PAWNIO"
$action = "RunExternalComponentSetupPawnio"
$dialog = "ExternalComponentsDlg"
# Type 18 (exe from an installed file) + 0x40 ignore exit code
# + 0x400 deferred + 0x800 no impersonation.
$expectedActionType = 18 + 0x40 + 0x400 + 0x800
# The LocalSystem action must only run the copy under Program Files.
$expectedActionCondition = "$property = `"1`" AND NOT REMOVE AND (INSTALLDIR ~<< ProgramFiles64Folder OR INSTALLDIR ~<< ProgramFilesFolder)"
# Type 51 (set a property from formatted text); only a fresh install without
# an explicit value on the command line gets the default.
$expectedDefaultType = 51
$expectedDefaultCondition = "NOT Installed AND NOT $property"

$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase((Resolve-Path $MsiPath).Path, 0)

function Get-Rows([string]$query, [int]$columns) {
  $view = $database.OpenView($query)
  # COM methods without a result still emit $null into the function output.
  [void]$view.Execute()
  $rows = @()
  while ($null -ne ($record = $view.Fetch())) {
    $rows += , @(1..$columns | ForEach-Object { $record.StringData($_) })
  }
  [void]$view.Close()
  return , $rows
}

$failures = [System.Collections.Generic.List[string]]::new()
function Assert([bool]$condition, [string]$message) {
  if (-not $condition) { $failures.Add($message) }
}

$customAction = Get-Rows "SELECT ``Type``, ``Source``, ``Target`` FROM ``CustomAction`` WHERE ``Action`` = '$action'" 3
Assert ($customAction.Count -eq 1) "CustomAction $action is missing"
if ($customAction.Count -eq 1) {
  $row = $customAction[0]
  Assert ([int]$row[0] -eq $expectedActionType) "CustomAction $action has type $($row[0]), expected $expectedActionType"
  Assert ($row[1] -eq "Path") "CustomAction $action runs '$($row[1])', expected the main binary (File Id 'Path')"
  Assert ($row[2] -eq "--external-component-setup pawnio") "CustomAction $action has arguments '$($row[2])'"
}

$sequence = @{}
foreach ($row in (Get-Rows "SELECT ``Action``, ``Condition``, ``Sequence`` FROM ``InstallExecuteSequence``" 3)) {
  $sequence[$row[0]] = $row
}
Assert ($sequence.ContainsKey($action)) "$action is not scheduled in InstallExecuteSequence"
if ($sequence.ContainsKey($action)) {
  $row = $sequence[$action]
  Assert ($row[1] -eq $expectedActionCondition) "$action has condition '$($row[1])'"
  Assert ([int]$row[2] -gt [int]$sequence["InstallFiles"][2]) "$action runs before InstallFiles"
  Assert ([int]$row[2] -lt [int]$sequence["InstallFinalize"][2]) "$action runs after InstallFinalize"
}

$defaultValue = Get-Rows "SELECT ``Value`` FROM ``Property`` WHERE ``Property`` = '$property'" 1
Assert ($defaultValue.Count -eq 0) "$property has a default value, so silent installs would run the setup"

$secure = Get-Rows "SELECT ``Value`` FROM ``Property`` WHERE ``Property`` = 'SecureCustomProperties'" 1
Assert (($secure.Count -eq 1) -and (($secure[0][0] -split ";") -contains $property)) "$property is not a secure custom property"

$setDefault = "Set$property"
$uiSequence = Get-Rows "SELECT ``Action``, ``Condition`` FROM ``InstallUISequence`` WHERE ``Action`` = '$setDefault'" 2
Assert ($uiSequence.Count -eq 1) "$setDefault is not in InstallUISequence"
if ($uiSequence.Count -eq 1) {
  Assert ($uiSequence[0][1] -eq $expectedDefaultCondition) "$setDefault has condition '$($uiSequence[0][1])', so an explicit value may be overwritten"
}
Assert (-not $sequence.ContainsKey($setDefault)) "$setDefault also runs in InstallExecuteSequence"

$defaultAction = Get-Rows "SELECT ``Type``, ``Source``, ``Target`` FROM ``CustomAction`` WHERE ``Action`` = '$setDefault'" 3
Assert ($defaultAction.Count -eq 1) "CustomAction $setDefault is missing"
if ($defaultAction.Count -eq 1) {
  $row = $defaultAction[0]
  Assert (([int]$row[0] -eq $expectedDefaultType) -and ($row[1] -eq $property) -and ($row[2] -eq "1")) "CustomAction $setDefault sets '$($row[1])' to '$($row[2])' (type $($row[0])), expected $property = 1"
}

$dialogRow = Get-Rows "SELECT ``Dialog`` FROM ``Dialog`` WHERE ``Dialog`` = '$dialog'" 1
Assert ($dialogRow.Count -eq 1) "Dialog $dialog is missing"

# The last NewDialog event wins, so the inserted dialog must have the highest
# order on InstallDirDlg Next.
$nextEvents = Get-Rows "SELECT ``Argument``, ``Ordering`` FROM ``ControlEvent`` WHERE ``Dialog_`` = 'InstallDirDlg' AND ``Control_`` = 'Next' AND ``Event`` = 'NewDialog'" 2
$lastNext = $nextEvents | Sort-Object { [int]$_[1] } | Select-Object -Last 1
Assert ($null -ne $lastNext -and $lastNext[0] -eq $dialog) "InstallDirDlg Next does not end on $dialog"

$backEvents = Get-Rows "SELECT ``Argument`` FROM ``ControlEvent`` WHERE ``Dialog_`` = 'VerifyReadyDlg' AND ``Control_`` = 'Back' AND ``Event`` = 'NewDialog' AND ``Argument`` = '$dialog'" 1
Assert ($backEvents.Count -eq 1) "VerifyReadyDlg Back does not return to $dialog"

if ($failures.Count -gt 0) {
  foreach ($failure in $failures) { Write-Host "::error title=MSI External Component Setup::$failure" }
  exit 1
}
Write-Host "MSI External Component Setup checks passed: $MsiPath"
