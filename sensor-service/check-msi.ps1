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

$feature = Get-MsiRow 'SELECT `Level` FROM `Feature` WHERE `Feature` = ''WindowsSensorService''' 1
Assert ($feature.Count -eq 1 -and $feature[0][0] -eq '2') "The sensor service must be optional by default"

$service = Get-MsiRow 'SELECT `Name`, `ServiceType`, `StartType`, `StartName`, `Component_` FROM `ServiceInstall` WHERE `ServiceInstall` = ''InstallWindowsSensorService''' 5
Assert ($service.Count -eq 1) "Sensor service registration missing"
Assert ($service[0][0] -eq 'HardwareVisualizerSensors' -and $service[0][1] -eq '16' -and $service[0][2] -eq '2' -and $service[0][3] -eq 'LocalSystem') "Unexpected service identity or start policy"

$component = $service[0][4]
$owner = Get-MsiRow "SELECT ``KeyPath`` FROM ``Component`` WHERE ``Component`` = '$component'" 1
Assert ($owner.Count -eq 1 -and $owner[0][0] -eq 'WindowsSensorServiceExe') "Service must own its executable"
$association = Get-MsiRow "SELECT ``Component_`` FROM ``FeatureComponents`` WHERE ``Feature_`` = 'WindowsSensorService'" 1
Assert ($association.Count -eq 1 -and $association[0][0] -eq $component) "Feature must own exactly its service component"

$control = Get-MsiRow 'SELECT `Name`, `Event`, `Wait` FROM `ServiceControl` WHERE `ServiceControl` = ''ControlWindowsSensorService''' 3
Assert ($control.Count -eq 1 -and $control[0][0] -eq 'HardwareVisualizerSensors') "Service lifecycle missing"
Assert (([int]$control[0][1] -band 163) -eq 163 -and $control[0][2] -eq '1') "Service must start on install, stop for replacement/removal and be deleted on full uninstall"

$conditions = Get-MsiRow 'SELECT `Level`, `Condition` FROM `Condition` WHERE `Feature_` = ''WindowsSensorService''' 2
Assert (@($conditions | Where-Object { $_[0] -eq '1' -and $_[1] -eq 'INSTALL_SENSOR_SERVICE = "1"' }).Count -eq 1) "Explicit feature selection missing"
Assert (@($conditions | Where-Object { $_[0] -eq '0' -and $_[1] -match 'ProgramFiles64Folder' }).Count -eq 1) "Unprotected install location must disable the service"

$validator = Get-MsiRow 'SELECT `Type`, `Source`, `Target` FROM `CustomAction` WHERE `Action` = ''ValidateSensorServiceDirectory''' 3
Assert ($validator.Count -eq 1 -and [int]$validator[0][0] -eq 3074 -and $validator[0][1] -eq 'SensorServiceValidator') "Directory validation must run the MSI-embedded copy, deferred as SYSTEM, and fail closed"
Assert ($validator[0][2] -eq '--validate-install-dir "[INSTALLDIR]."') "Directory validation arguments changed"

$sequence = @{}
foreach ($row in (Get-MsiRow 'SELECT `Action`, `Condition`, `Sequence` FROM `InstallExecuteSequence`' 3)) { $sequence[$row[0]] = $row }
Assert ($sequence['ValidateSensorServiceDirectory'][1] -eq '&WindowsSensorService = 3 AND NOT REMOVE') "Validation must follow selected feature state"
Assert ([int]$sequence['ValidateSensorServiceDirectory'][2] -gt [int]$sequence['CreateFolders'][2] -and [int]$sequence['ValidateSensorServiceDirectory'][2] -lt [int]$sequence['InstallFiles'][2]) "Validate trusted directory ancestry before placing executable code"

$secure = Get-MsiRow 'SELECT `Value` FROM `Property` WHERE `Property` = ''SecureCustomProperties''' 1
Assert ($secure.Count -eq 1 -and ($secure[0][0] -split ';') -contains 'INSTALL_SENSOR_SERVICE') "Service consent property must cross the MSI elevation boundary"
Write-Output "MSI sensor-service ownership, consent, validation and lifecycle checks passed: $MsiPath"
