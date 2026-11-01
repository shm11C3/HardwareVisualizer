#!/usr/bin/env pwsh
# Exercise the real wrapper with a local extension-sensitive signer; no cloud
# credentials, certificate, installer execution, or external download is needed.
$ErrorActionPreference = 'Stop'
$wrapper = Join-Path $PSScriptRoot 'sign-codesigntool.ps1'
$tempBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$testRoot = Join-Path $tempBase ('codesign-wrapper-test-' + [Guid]::NewGuid().ToString('N'))
$toolDir = Join-Path $testRoot 'mock tool'
$staging = Join-Path $testRoot 'staging'
$environmentNames = @('CODESIGNTOOL_DIR', 'ES_USERNAME', 'ES_PASSWORD', 'ES_TOTP_SECRET', 'CREDENTIAL_ID', 'RUNNER_TEMP', 'SIGN_TEST_MODE', 'SIGN_TEST_TRACE')
$savedEnvironment = @{}
foreach ($name in $environmentNames) {
  $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name)
}

try {
  New-Item -ItemType Directory -Path $toolDir, $staging | Out-Null
  @'
@echo off
:args
if "%~1"=="" goto run
set "arg=%~1"
if "%arg:~0,17%"=="-input_file_path=" set "SIGN_MOCK_INPUT=%arg:~17%"
if "%arg:~0,17%"=="-output_dir_path=" set "SIGN_MOCK_OUTPUT=%arg:~17%"
if "%~1"=="-input_file_path" set "SIGN_MOCK_INPUT=%~2"
if "%~1"=="-output_dir_path" set "SIGN_MOCK_OUTPUT=%~2"
shift
goto args
:run
pwsh -NoProfile -File "%~dp0mock-codesign.ps1"
exit /b %ERRORLEVEL%
'@ | Set-Content -LiteralPath (Join-Path $toolDir 'CodeSignTool.bat') -Encoding ascii
  @'
$ErrorActionPreference = 'Stop'
$inputPath = $env:SIGN_MOCK_INPUT
$outputDir = $env:SIGN_MOCK_OUTPUT
if (-not $inputPath -or -not $outputDir) { throw 'Missing CodeSignTool path arguments' }
@{ inputPath = $inputPath; cwd = (Get-Location).Path } | ConvertTo-Json | Set-Content -LiteralPath $env:SIGN_TEST_TRACE
if ([IO.Path]::GetExtension($inputPath) -notin @('.exe', '.dll', '.msi')) {
  Write-Host "Error: Unsupported file format for signing: $inputPath"
  exit 0
}
if ($env:SIGN_TEST_MODE -eq 'missing-output') { exit 0 }
$signedPath = Join-Path $outputDir ([IO.Path]::GetFileName($inputPath))
[IO.File]::WriteAllText($signedPath, [IO.File]::ReadAllText($inputPath) + ':signed')
if ($env:SIGN_TEST_MODE -eq 'nonzero-with-output') { exit 23 }
exit 0
'@ | Set-Content -LiteralPath (Join-Path $toolDir 'mock-codesign.ps1') -Encoding utf8

  $env:CODESIGNTOOL_DIR = $toolDir
  $env:ES_USERNAME = 'test-user'
  $env:ES_PASSWORD = 'test-password'
  $env:ES_TOTP_SECRET = 'test-totp'
  $env:CREDENTIAL_ID = 'test-credential'
  $env:RUNNER_TEMP = $staging

  $cases = @(
    @{ file = 'setup [test].exe'; mode = 'success'; extension = '.exe'; succeeds = $true },
    @{ file = 'plugin.dll'; mode = 'success'; extension = '.dll'; succeeds = $true },
    @{ file = 'setup.msi'; mode = 'success'; extension = '.msi'; succeeds = $true },
    @{ file = 'nst82B9.tmp'; mode = 'success'; extension = '.exe'; succeeds = $true },
    @{ file = 'nst82B9.TMP'; mode = 'success'; extension = '.exe'; succeeds = $true },
    @{ file = 'missing.tmp'; mode = 'missing-output'; extension = '.exe'; succeeds = $false },
    @{ file = 'failed.tmp'; mode = 'nonzero-with-output'; extension = '.exe'; succeeds = $false }
  )
  foreach ($case in $cases) {
    $inputPath = Join-Path $testRoot $case.file
    $logPath = Join-Path $testRoot 'wrapper.log'
    $env:SIGN_TEST_TRACE = Join-Path $testRoot 'trace.json'
    $env:SIGN_TEST_MODE = $case.mode
    [IO.File]::WriteAllText($inputPath, 'fixture')
    & pwsh -NoProfile -File $wrapper -FilePath $inputPath *> $logPath
    $code = $LASTEXITCODE
    if (($code -eq 0) -ne $case.succeeds) {
      throw "Unexpected wrapper exit $code for $($case.file): $(Get-Content -LiteralPath $logPath -Raw)"
    }
    $trace = Get-Content -LiteralPath $env:SIGN_TEST_TRACE -Raw | ConvertFrom-Json
    if ([IO.Path]::GetExtension($trace.inputPath) -ne $case.extension -or $trace.cwd -ne $toolDir) {
      throw "Unexpected signer input or working directory for $($case.file)"
    }
    $expected = if ($case.succeeds) { 'fixture:signed' } else { 'fixture' }
    if ([IO.File]::ReadAllText($inputPath) -ne $expected) {
      throw "Original path was not restored or was changed on failure: $($case.file)"
    }
    if (@(Get-ChildItem -LiteralPath $staging -Force).Count -ne 0) {
      throw "Signing workspace leaked for $($case.file)"
    }
    Write-Output "PASS: $($case.file) / $($case.mode)"
  }
}
finally {
  foreach ($name in $environmentNames) {
    [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name])
  }
  if ([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($testRoot)).TrimEnd('\', '/') -ne $tempBase.TrimEnd('\', '/')) {
    throw 'Refusing to remove a test directory outside the temporary directory'
  }
  if (Test-Path -LiteralPath $testRoot) { Remove-Item -LiteralPath $testRoot -Recurse -Force }
}

# GitHub Actions propagates LASTEXITCODE after invoking a PowerShell script.
# Negative cases intentionally leave it nonzero; report success only after
# every assertion and cleanup above has completed without throwing.
exit 0
