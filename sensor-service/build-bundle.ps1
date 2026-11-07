param(
  [ValidateSet('dev', 'ci', 'release')]
  [string]$BuildProfile = $(
    if ($env:HARDVIZ_SENSOR_SERVICE_PROFILE) {
      $env:HARDVIZ_SENSOR_SERVICE_PROFILE
    }
    else {
      'release'
    }
  )
)

$ErrorActionPreference = 'Stop'

if (-not $IsWindows) {
  throw 'The sensor service bundle is Windows-only'
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$previousVersion = $env:HARDVIZ_SENSOR_VERSION

Push-Location -LiteralPath $repoRoot

try {
  $config = Get-Content -LiteralPath (Join-Path $repoRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
  $env:HARDVIZ_SENSOR_VERSION = $config.version

  $cargoArgs = @('build', '--locked', '--profile', $BuildProfile, '-p', 'hardviz-sensor-service')
  $serviceTarget = $env:TAURI_ENV_TARGET_TRIPLE
  if ($serviceTarget) {
    $cargoArgs += @('--target', $serviceTarget)
  }

  & cargo @cargoArgs

  if ($LASTEXITCODE -ne 0) {
    throw "Sensor service build failed ($LASTEXITCODE)"
  }

  $metadataJson = & cargo metadata --offline --locked --no-deps --format-version 1
  if ($LASTEXITCODE -ne 0) {
    throw "Cannot resolve Cargo output directory ($LASTEXITCODE)"
  }

  $metadata = $metadataJson | ConvertFrom-Json
  $outputDir = $metadata.target_directory
  if ($serviceTarget) {
    $outputDir = Join-Path $outputDir $serviceTarget
  }

  $profileDir = if ($BuildProfile -eq 'dev') { 'debug' } else { $BuildProfile }
  $source = Join-Path (Join-Path $outputDir $profileDir) 'hardware-visualizer-sensors.exe'

  $destinationDir = Join-Path $repoRoot 'src-tauri/windows/wix/generated'
  [void](New-Item -ItemType Directory -Path $destinationDir -Force)
  $destination = Join-Path $destinationDir 'hardware-visualizer-sensors.exe'
  Copy-Item -LiteralPath $source -Destination $destination -Force

  # Sign the bytes used by both the MSI File and embedded validator before
  # bundling. Signing only the MSI or the app would leave this executable unsigned.
  if ($env:ENABLE_WINDOWS_SIGNING -eq 'true') {
    $global:LASTEXITCODE = 0
    & (Join-Path $repoRoot '.github/scripts/sign-codesigntool.ps1') -FilePath $destination
    if ($LASTEXITCODE -ne 0) {
      throw "Sensor service signing failed ($LASTEXITCODE)"
    }
  }
}
finally {
  $env:HARDVIZ_SENSOR_VERSION = $previousVersion
  Pop-Location
}
