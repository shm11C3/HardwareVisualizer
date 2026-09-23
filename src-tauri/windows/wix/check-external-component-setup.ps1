#!/usr/bin/env pwsh
# Assert that a built MSI carries the External Component Setup option (#2118)
# with the consent rules from ADR 0024:
#
# - the setup custom action is deferred, runs without impersonation, ignores
#   its exit code, and runs only when EXTERNAL_COMPONENT_PAWNIO = "1" and the
#   install directory is under Program Files;
# - EXTERNAL_COMPONENT_PAWNIO has no default in the Property table and is only
#   defaulted by the UI sequence at full UI, so /qn, /qr, /passive, and winget
#   run no setup;
# - the options dialog is inserted between InstallDirDlg and VerifyReadyDlg,
#   keeps the template's path validation on the way in, links back into the
#   chain on both sides, and its checkbox is the enabled control bound to
#   EXTERNAL_COMPONENT_PAWNIO with value 1;
# - an interactive, non-upgrade uninstall runs the notice mode before removal
#   and after costing, so its INSTALLDIR condition is evaluated on a value.
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
# The LocalSystem action must only run the copy under Program Files. MSI
# property names are case-sensitive, so string comparisons use -ceq.
$expectedActionCondition = "$property = `"1`" AND NOT REMOVE AND ((INSTALLDIR ~<< ProgramFiles64Folder OR INSTALLDIR ~<< ProgramFilesFolder) AND NOT (INSTALLDIR >< `"..`"))"
# Type 51 (set a property from formatted text); only a fresh install without
# an explicit value on the command line gets the default, and only at full UI
# (UILevel 5): the UI sequence also runs at reduced UI (/qr, UILevel 4), where
# the dialog that carries the consent is suppressed.
$expectedDefaultType = 51
$expectedDefaultCondition = "NOT Installed AND NOT $property AND UILevel = 5"

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
  Assert ($row[1] -ceq "Path") "CustomAction $action runs '$($row[1])', expected the main binary (File Id 'Path')"
  Assert ($row[2] -ceq "--external-component-setup pawnio") "CustomAction $action has arguments '$($row[2])'"
}

$sequence = @{}
foreach ($row in (Get-Rows "SELECT ``Action``, ``Condition``, ``Sequence`` FROM ``InstallExecuteSequence``" 3)) {
  $sequence[$row[0]] = $row
}
Assert ($sequence.ContainsKey($action)) "$action is not scheduled in InstallExecuteSequence"
if ($sequence.ContainsKey($action)) {
  $row = $sequence[$action]
  Assert ($row[1] -ceq $expectedActionCondition) "$action has condition '$($row[1])'"
  Assert ([int]$row[2] -gt [int]$sequence["InstallFiles"][2]) "$action runs before InstallFiles"
  Assert ([int]$row[2] -lt [int]$sequence["InstallFinalize"][2]) "$action runs after InstallFinalize"
}

# Uninstall notice (#2119): immediate, so it runs as the user and before any
# file is removed; never on silent uninstalls, upgrades, or outside Program Files.
$noticeAction = "ShowExternalComponentUninstallNotice"
$locationCondition = $expectedActionCondition.Substring("$property = `"1`" AND NOT REMOVE AND ".Length)
$noticeRow = Get-Rows "SELECT ``Type``, ``Source``, ``Target`` FROM ``CustomAction`` WHERE ``Action`` = '$noticeAction'" 3
Assert (($noticeRow.Count -eq 1) -and ([int]$noticeRow[0][0] -eq (18 + 0x40)) -and ($noticeRow[0][1] -ceq "Path") -and ($noticeRow[0][2] -ceq "--external-component-notice uninstall")) "CustomAction $noticeAction must be an immediate, exit-code-ignoring run of the main binary with --external-component-notice uninstall"
Assert ($sequence.ContainsKey($noticeAction)) "$noticeAction is not scheduled in InstallExecuteSequence"
if ($sequence.ContainsKey($noticeAction)) {
  $row = $sequence[$noticeAction]
  Assert ($row[1] -ceq "REMOVE = `"ALL`" AND NOT UPGRADINGPRODUCTCODE AND UILevel > 2 AND NOT (UILevel = 3 AND REBOOTPROMPT = `"S`") AND $locationCondition") "$noticeAction has condition '$($row[1])'"
  Assert ([int]$row[2] -lt [int]$sequence["InstallInitialize"][2]) "$noticeAction must run before InstallInitialize, while the executable still exists"
  # Its condition reads INSTALLDIR, which is empty before costing resolves it.
  Assert ([int]$row[2] -gt [int]$sequence["CostFinalize"][2]) "$noticeAction must run after CostFinalize, so INSTALLDIR is resolved when its condition is evaluated"
}

$defaultValue = Get-Rows "SELECT ``Value`` FROM ``Property`` WHERE ``Property`` = '$property'" 1
Assert ($defaultValue.Count -eq 0) "$property has a default value, so silent installs would run the setup"

$secure = Get-Rows "SELECT ``Value`` FROM ``Property`` WHERE ``Property`` = 'SecureCustomProperties'" 1
Assert (($secure.Count -eq 1) -and (($secure[0][0] -split ";") -ccontains $property)) "$property is not a secure custom property"

$setDefault = "Set$property"
$uiSequence = Get-Rows "SELECT ``Action``, ``Condition`` FROM ``InstallUISequence`` WHERE ``Action`` = '$setDefault'" 2
Assert ($uiSequence.Count -eq 1) "$setDefault is not in InstallUISequence"
if ($uiSequence.Count -eq 1) {
  Assert ($uiSequence[0][1] -ceq $expectedDefaultCondition) "$setDefault has condition '$($uiSequence[0][1])', so an explicit value may be overwritten"
}
Assert (-not $sequence.ContainsKey($setDefault)) "$setDefault also runs in InstallExecuteSequence"

$defaultAction = Get-Rows "SELECT ``Type``, ``Source``, ``Target`` FROM ``CustomAction`` WHERE ``Action`` = '$setDefault'" 3
Assert ($defaultAction.Count -eq 1) "CustomAction $setDefault is missing"
if ($defaultAction.Count -eq 1) {
  $row = $defaultAction[0]
  Assert (([int]$row[0] -eq $expectedDefaultType) -and ($row[1] -ceq $property) -and ($row[2] -ceq "1")) "CustomAction $setDefault sets '$($row[1])' to '$($row[2])' (type $($row[0])), expected $property = 1"
}

$dialogRow = Get-Rows "SELECT ``Dialog`` FROM ``Dialog`` WHERE ``Dialog`` = '$dialog'" 1
Assert ($dialogRow.Count -eq 1) "Dialog $dialog is missing"

# NSIS-inherited install folder (#2215): a fresh install that only inherited
# the exact NSIS default folder starts from Program Files; a folder the user
# chose for the NSIS build is kept. The UI copy runs before the folder page;
# the execute copy only without the UI sequence.
$installDirValue = "[ProgramFiles64Folder]HardwareVisualizer\"
$installDirBase = "NOT HV_MSI_INSTALLDIR AND (INSTALLDIR ~= HV_NSIS_DEFAULT_INSTALLDIR OR INSTALLDIR ~= HV_NSIS_DEFAULT_INSTALLDIR_DIR)"
$uiSearchSequence = @{}
foreach ($row in (Get-Rows "SELECT ``Action``, ``Condition``, ``Sequence`` FROM ``InstallUISequence``" 3)) {
  $uiSearchSequence[$row[0]] = $row
}
foreach ($default in @(
  @{ property = "HV_NSIS_DEFAULT_INSTALLDIR"; value = "[LocalAppDataFolder]HardwareVisualizer" },
  @{ property = "HV_NSIS_DEFAULT_INSTALLDIR_DIR"; value = "[LocalAppDataFolder]HardwareVisualizer\" })) {
  $action = "Set$($default.property)"
  $ca = Get-Rows "SELECT ``Type``, ``Source``, ``Target`` FROM ``CustomAction`` WHERE ``Action`` = '$action'" 3
  Assert (($ca.Count -eq 1) -and ([int]$ca[0][0] -eq 51) -and ($ca[0][1] -ceq $default.property) -and ($ca[0][2] -ceq $default.value)) "CustomAction $action must set $($default.property) to $($default.value)"
  foreach ($table in @($uiSearchSequence, $sequence)) {
    Assert ($table.ContainsKey($action) -and [int]$table[$action][2] -gt [int]$table["AppSearch"][2]) "$action must run after AppSearch in both sequences"
  }
}
foreach ($case in @(
  @{ action = "SetInstallDirFromNsisDefaultUi"; table = $uiSearchSequence; condition = "NOT Installed AND $installDirBase" },
  @{ action = "SetInstallDirFromNsisDefaultExecute"; table = $sequence; condition = "NOT Installed AND UILevel < 4 AND $installDirBase" })) {
  $ca = Get-Rows "SELECT ``Type``, ``Source``, ``Target`` FROM ``CustomAction`` WHERE ``Action`` = '$($case.action)'" 3
  Assert (($ca.Count -eq 1) -and ([int]$ca[0][0] -eq 51) -and ($ca[0][1] -ceq "INSTALLDIR") -and ($ca[0][2] -ceq $installDirValue)) "CustomAction $($case.action) must set INSTALLDIR to $installDirValue"
  $row = $case.table[$case.action]
  Assert ($null -ne $row) "$($case.action) is not scheduled"
  if ($null -ne $row) {
    Assert ($row[1] -ceq $case.condition) "$($case.action) has condition '$($row[1])'"
    Assert (([int]$row[2] -gt [int]$case.table["AppSearch"][2]) -and ([int]$row[2] -lt [int]$case.table["CostInitialize"][2])) "$($case.action) must run after AppSearch and before CostInitialize"
    # Its condition reads the comparison properties, so they must be set first.
    foreach ($propertyAction in @("SetHV_NSIS_DEFAULT_INSTALLDIR", "SetHV_NSIS_DEFAULT_INSTALLDIR_DIR")) {
      $propertyRow = $case.table[$propertyAction]
      Assert (($null -ne $propertyRow) -and ([int]$propertyRow[2] -lt [int]$row[2])) "$propertyAction must run before $($case.action)"
    }
  }
}
Assert (-not $uiSearchSequence.ContainsKey("SetInstallDirFromNsisDefaultExecute")) "The execute copy must not run in the UI sequence"
Assert (-not $sequence.ContainsKey("SetInstallDirFromNsisDefaultUi")) "The UI copy must not run in the execute sequence"

# Outside Program Files the real checkbox must be hidden and replaced by an
# unchecked placeholder, so the dialog never shows a selection that will not run.
$controlConditions = @{}
foreach ($row in (Get-Rows "SELECT ``Control_``, ``Action``, ``Condition`` FROM ``ControlCondition`` WHERE ``Dialog_`` = '$dialog'" 3)) {
  $controlConditions["$($row[0])/$($row[1])"] = $row[2]
}
Assert ($controlConditions["PawnioCheckBox/Hide"] -ceq "NOT $locationCondition") "PawnioCheckBox is not hidden outside Program Files"
Assert ($controlConditions["PawnioUnavailableCheckBox/Show"] -ceq "NOT $locationCondition") "PawnioUnavailableCheckBox is not shown outside Program Files"
$placeholder = Get-Rows "SELECT ``Property``, ``Attributes`` FROM ``Control`` WHERE ``Dialog_`` = '$dialog' AND ``Control`` = 'PawnioUnavailableCheckBox'" 2
# Attribute 0x2 is "enabled", so a disabled control has it cleared; the bound
# property must not be the real one.
Assert (($placeholder.Count -eq 1) -and ($placeholder[0][0] -cne $property) -and (([int]$placeholder[0][1] -band 0x2) -eq 0)) "PawnioUnavailableCheckBox must be disabled and bound to a property other than $property"

# The consent itself: the real checkbox is enabled and bound to the property
# the deferred action reads, and ticking it stores the value the action
# compares against.
$checkBox = Get-Rows "SELECT ``Property``, ``Attributes`` FROM ``Control`` WHERE ``Dialog_`` = '$dialog' AND ``Control`` = 'PawnioCheckBox'" 2
Assert (($checkBox.Count -eq 1) -and ($checkBox[0][0] -ceq $property) -and (([int]$checkBox[0][1] -band 0x2) -ne 0)) "PawnioCheckBox must be enabled and bound to $property"
$checkBoxValue = Get-Rows "SELECT ``Value`` FROM ``CheckBox`` WHERE ``Property`` = '$property'" 1
Assert (($checkBoxValue.Count -eq 1) -and ($checkBoxValue[0][0] -ceq "1")) "CheckBox table must map $property to 1, the value the setup action is conditioned on"

# The last NewDialog event wins, so the inserted dialog must have the highest
# order on InstallDirDlg Next, and it must keep the template's path validation
# condition (currently WIXUI_DONTVALIDATEPATH OR WIXUI_INSTALLDIR_VALID="1"):
# a bare condition would skip the invalid-path check the template's own
# NewDialog carries.
$nextEvents = Get-Rows "SELECT ``Argument``, ``Ordering``, ``Condition`` FROM ``ControlEvent`` WHERE ``Dialog_`` = 'InstallDirDlg' AND ``Control_`` = 'Next' AND ``Event`` = 'NewDialog'" 3
$sortedNext = @($nextEvents | Sort-Object { [int]$_[1] })
$lastNext = $sortedNext | Select-Object -Last 1
Assert ($null -ne $lastNext -and $lastNext[0] -ceq $dialog) "InstallDirDlg Next does not end on $dialog"
$templateNext = $sortedNext | Where-Object { $_[0] -cne $dialog } | Select-Object -Last 1
Assert (($null -ne $lastNext) -and ($null -ne $templateNext) -and ($lastNext[2] -ceq $templateNext[2])) "InstallDirDlg Next to $dialog has condition '$($lastNext[2])', expected the template's '$($templateNext[2])'"

$backEvents = Get-Rows "SELECT ``Argument`` FROM ``ControlEvent`` WHERE ``Dialog_`` = 'VerifyReadyDlg' AND ``Control_`` = 'Back' AND ``Event`` = 'NewDialog' AND ``Argument`` = '$dialog'" 1
Assert ($backEvents.Count -eq 1) "VerifyReadyDlg Back does not return to $dialog"

# The inserted dialog must lead back into the template chain on both sides.
foreach ($link in @(
  @{ control = "Next"; target = "VerifyReadyDlg" },
  @{ control = "Back"; target = "InstallDirDlg" })) {
  $events = Get-Rows "SELECT ``Argument``, ``Condition`` FROM ``ControlEvent`` WHERE ``Dialog_`` = '$dialog' AND ``Control_`` = '$($link.control)' AND ``Event`` = 'NewDialog'" 2
  Assert (($events.Count -eq 1) -and ($events[0][0] -ceq $link.target) -and ($events[0][1] -ceq "1")) "$dialog $($link.control) must always go to $($link.target)"
}

if ($failures.Count -gt 0) {
  foreach ($failure in $failures) { Write-Host "::error title=MSI External Component Setup::$failure" }
  exit 1
}
Write-Host "MSI External Component Setup checks passed: $MsiPath"
