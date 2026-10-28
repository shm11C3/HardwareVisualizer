#!/usr/bin/env pwsh
# Sign a single file with SSL.com eSigner CodeSignTool (cloud signing).
#
# This wrapper is invoked by Tauri's `bundle.windows.signCommand` with the file
# to sign passed as the only argument (the `%1` placeholder). CodeSignTool does
# not sign in place: it writes the signed file into an output *directory* and it
# must be run from inside its own install directory. So we sign into a temporary
# directory and then move the signed file back over the original path, which is
# what Tauri expects `%1` to be after signing.
#
# Required environment variables (provided by the GitHub Actions workflow):
#   CODESIGNTOOL_DIR - directory that contains CodeSignTool.bat
#   ES_USERNAME      - SSL.com / eSigner account username
#   ES_PASSWORD      - SSL.com / eSigner account password
#   ES_TOTP_SECRET   - eSigner TOTP secret, enables non-interactive OTP generation
#   CREDENTIAL_ID    - eSigner credential id (required when the account has >1 certificate)
#
# NOTE: secrets are passed to CodeSignTool.bat as CLI flags. Avoid using cmd.exe
# special characters (& | < > ^ %) in the eSigner password to prevent quoting issues.

param(
  [Parameter(Mandatory = $true, Position = 0)]
  [string]$FilePath
)

$ErrorActionPreference = "Stop"

$toolDir = $env:CODESIGNTOOL_DIR
if (-not $toolDir) { throw "CODESIGNTOOL_DIR is not set" }
if (-not $env:ES_USERNAME) { throw "ES_USERNAME is not set" }
if (-not $env:ES_PASSWORD) { throw "ES_PASSWORD is not set" }
# Required for non-interactive (headless CI) signing: without it CodeSignTool
# blocks on a manual OTP prompt instead of generating the OTP from the secret.
if (-not $env:ES_TOTP_SECRET) { throw "ES_TOTP_SECRET is not set" }

$absInput = (Resolve-Path -LiteralPath $FilePath).Path

$tempBase = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
$tempBase = [System.IO.Path]::GetFullPath($tempBase)
$workDir = Join-Path $tempBase ("codesign-" + [System.Guid]::NewGuid().ToString("N"))
$outDir = Join-Path $workDir "signed"
try {
  New-Item -ItemType Directory -Path $outDir -Force | Out-Null
  $signInput = $absInput
  if ([System.IO.Path]::GetExtension($absInput) -eq ".tmp") {
    # NSIS !uninstfinalize supplies a PE uninstaller named nst*.tmp, but
    # CodeSignTool rejects that extension. Sign a copy as .exe, then restore
    # the signed bytes to NSIS's original path. Keep input outside the output
    # directory so an exit-0 signing error cannot pass the artifact check.
    $inputDir = Join-Path $workDir "input"
    New-Item -ItemType Directory -Path $inputDir | Out-Null
    $signInput = Join-Path $inputDir ([System.IO.Path]::ChangeExtension([System.IO.Path]::GetFileName($absInput), ".exe"))
    Copy-Item -LiteralPath $absInput -Destination $signInput
  }

  $signArgs = @(
    "sign",
    "-username=$($env:ES_USERNAME)",
    "-password=$($env:ES_PASSWORD)",
    "-input_file_path=$signInput",
    "-output_dir_path=$outDir"
  )
  $signArgs += "-totp_secret=$($env:ES_TOTP_SECRET)"
  # credential_id is only required when the eSigner account has more than one certificate.
  if ($env:CREDENTIAL_ID) { $signArgs += "-credential_id=$($env:CREDENTIAL_ID)" }

  Write-Host "Signing with CodeSignTool: $absInput"

  # CodeSignTool.bat can only run from inside its own directory.
  Push-Location $toolDir
  try {
    & ".\CodeSignTool.bat" @signArgs
    $code = $LASTEXITCODE
  }
  finally {
    Pop-Location
  }

  $signedFile = Join-Path $outDir ([System.IO.Path]::GetFileName($signInput))

  # CodeSignTool sometimes exits 0 while printing an error and producing no output,
  # so verify the signed artifact actually exists before trusting the result.
  if ($code -ne 0 -or -not (Test-Path -LiteralPath $signedFile -PathType Leaf)) {
    throw "CodeSignTool failed (exit code $code) for: $absInput"
  }

  Move-Item -LiteralPath $signedFile -Destination $absInput -Force
  Write-Host "Signed: $absInput"
}
finally {
  if ([System.IO.Path]::GetDirectoryName([System.IO.Path]::GetFullPath($workDir)).TrimEnd('\', '/') -ne $tempBase.TrimEnd('\', '/')) {
    throw "Refusing to remove a signing workspace outside its temporary directory"
  }
  Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
}
