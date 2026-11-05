param([Parameter(Mandatory = $true)][string]$MsiPath)

$ErrorActionPreference = "Stop"
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase((Resolve-Path -LiteralPath $MsiPath).Path, 0)

function Get-MsiRow([string]$query, [int]$columns) {
  $view = $database.OpenView($query)
  [void]$view.Execute()
  $rows = @()

  while ($null -ne ($record = $view.Fetch())) {
    $rows += ,@(1..$columns | ForEach-Object { $record.StringData($_) })
  }

  [void]$view.Close()
  return ,$rows
}

function Assert([bool]$condition, [string]$message) {
  if (-not $condition) { throw $message }
}

$feature = Get-MsiRow 'SELECT `Level`, `Attributes`, `Feature_Parent` FROM `Feature` WHERE `Feature` = ''WindowsSensorService''' 3
Assert ($feature.Count -eq 1 -and $feature[0][0] -eq '1') "The sensor service must be standard in fresh and upgraded MSI installs"
Assert (([int]$feature[0][1] -band 18) -eq 18 -and $feature[0][2] -eq 'External') "Service selection must follow its required application parent and disallow independent removal"

$service = Get-MsiRow 'SELECT `Name`, `ServiceType`, `StartType`, `StartName`, `Component_` FROM `ServiceInstall` WHERE `ServiceInstall` = ''InstallWindowsSensorService''' 5
Assert ($service.Count -eq 1) "Sensor service registration missing"
Assert ($service[0][0] -eq 'HardwareVisualizerSensors' -and $service[0][1] -eq '16' -and $service[0][2] -eq '2' -and $service[0][3] -eq 'LocalSystem') "Unexpected service identity or start policy"

$component = $service[0][4]
$owner = Get-MsiRow "SELECT ``KeyPath``, ``Condition`` FROM ``Component`` WHERE ``Component`` = '$component'" 2
Assert ($owner.Count -eq 1 -and $owner[0][0] -eq 'WindowsSensorServiceExe') "Service must own its executable"
Assert ($owner[0][1] -match 'ProgramFiles64Folder' -and $owner[0][1] -match 'ProgramFilesFolder' -and $owner[0][1] -match 'NOT') "The component must enforce the protected install location independently of feature selection"
$association = Get-MsiRow "SELECT ``Component_`` FROM ``FeatureComponents`` WHERE ``Feature_`` = 'WindowsSensorService'" 1
Assert ($association.Count -eq 1 -and $association[0][0] -eq $component) "Feature must own exactly its service component"

$control = Get-MsiRow 'SELECT `Name`, `Event`, `Wait` FROM `ServiceControl` WHERE `ServiceControl` = ''ControlWindowsSensorService''' 3
Assert ($control.Count -eq 1 -and $control[0][0] -eq 'HardwareVisualizerSensors') "Service lifecycle missing"
Assert (([int]$control[0][1] -band 163) -eq 163 -and $control[0][2] -eq '1') "Service must start on install, stop for replacement/removal and be deleted on full uninstall"

$conditionTable = Get-MsiRow 'SELECT `Name` FROM `_Tables` WHERE `Name` = ''Condition''' 1
if ($conditionTable.Count -gt 0) {
  $conditions = Get-MsiRow 'SELECT `Level`, `Condition` FROM `Condition` WHERE `Feature_` = ''WindowsSensorService''' 2
  Assert ($conditions.Count -eq 0) "The bundled feature must not depend on opt-in or location conditions; its component owns location safety"
}

$validator = Get-MsiRow 'SELECT `Type`, `Source`, `Target` FROM `CustomAction` WHERE `Action` = ''ValidateSensorServiceDirectory''' 3
Assert ($validator.Count -eq 1 -and [int]$validator[0][0] -eq 3074 -and $validator[0][1] -eq 'SensorServiceValidator') "Directory validation must run the MSI-embedded copy, deferred as SYSTEM, and fail closed"
Assert ($validator[0][2] -eq '--validate-install-dir "[INSTALLDIR]."') "Directory validation arguments changed"

$sequence = @{}
foreach ($row in (Get-MsiRow 'SELECT `Action`, `Condition`, `Sequence` FROM `InstallExecuteSequence`' 3)) { $sequence[$row[0]] = $row }
Assert ($sequence['ValidateSensorServiceDirectory'][1] -eq '$WindowsSensorServiceComponent = 3 AND NOT REMOVE') "Validation must follow the component install action"
Assert ([int]$sequence['ValidateSensorServiceDirectory'][2] -gt [int]$sequence['CreateFolders'][2] -and [int]$sequence['ValidateSensorServiceDirectory'][2] -lt [int]$sequence['InstallFiles'][2]) "Validate trusted directory ancestry before placing executable code"

$choice = Get-MsiRow 'SELECT `Control` FROM `Control` WHERE `Control` = ''SensorServiceCheckBox''' 1
Assert ($choice.Count -eq 0) "The app-owned service must not require a separate installer checkbox"
$selection = Get-MsiRow 'SELECT `Property` FROM `Property` WHERE `Property` = ''INSTALL_SENSOR_SERVICE'' OR `Property` = ''HV_SENSOR_SERVICE_SELECTED''' 1
Assert ($selection.Count -eq 0 -and -not $sequence.ContainsKey('SetINSTALL_SENSOR_SERVICE')) "Obsolete service-selection state must be removed"

# Costing evaluates the built MSI at each UI level without executing an
# installation, registering a service or requesting elevation.
foreach ($case in @(
  @{ name = 'fresh full UI'; level = '5'; upgrade = ''; directory = "$env:ProgramFiles\HardwareVisualizer"; expected = 3 },
  @{ name = 'fresh silent'; level = '2'; upgrade = ''; directory = "$env:ProgramFiles\HardwareVisualizer"; expected = 3 },
  @{ name = 'upgrade silent'; level = '2'; upgrade = '{00000000-0000-0000-0000-000000000001}'; directory = "$env:ProgramFiles\HardwareVisualizer"; expected = 3 },
  # A component whose condition is false has no requested action (-1),
  # rather than an explicit uninstall request (2).
  @{ name = 'unprotected'; level = '2'; upgrade = ''; directory = "$env:LOCALAPPDATA\HardwareVisualizer"; expected = -1 }
)) {
  $session = $installer.OpenPackage((Resolve-Path -LiteralPath $MsiPath).Path, 1)
  $session.Property('UILevel') = $case.level
  $session.Property('INSTALLDIR') = $case.directory
  $session.Property('WIX_UPGRADE_DETECTED') = $case.upgrade

  foreach ($action in @('CostInitialize', 'FileCost', 'CostFinalize')) {
    Assert ($session.DoAction($action) -eq 1) "$($case.name): $action failed"
  }

  $featureState = $session.FeatureRequestState('WindowsSensorService')
  $componentState = $session.ComponentRequestState($component)
  Assert ($featureState -eq 3) "$($case.name): bundled feature must follow the application parent (got $featureState)"
  Assert ($componentState -eq $case.expected) "$($case.name): unexpected requested service component state ($componentState)"
  Write-Output "MSI plan '$($case.name)': service component action $componentState"
  [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($session)
}

Write-Output "MSI sensor-service default selection, ownership, validation and lifecycle checks passed: $MsiPath"
