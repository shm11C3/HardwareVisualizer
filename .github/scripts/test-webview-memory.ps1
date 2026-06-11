[CmdletBinding()]
param(
  [string]$Binary = "target/ci/hardware-visualizer.exe",
  [int]$WarmupSeconds = 60,
  [int]$DurationSeconds = 600,
  [int]$SampleIntervalSeconds = 5,
  [int]$TailWindowSeconds = 180,
  [double]$MaxGrowthMb = 150,
  [double]$MinTailSlopeMbPerMinute = 1,
  [string]$OutputDirectory = "test-results/webview-memory"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not $IsWindows) {
  throw "WebView2 memory performance checks are Windows-only."
}

if ($WarmupSeconds -lt 0) {
  throw "WarmupSeconds must be greater than or equal to 0."
}

if ($DurationSeconds -le 0) {
  throw "DurationSeconds must be greater than 0."
}

if ($SampleIntervalSeconds -le 0) {
  throw "SampleIntervalSeconds must be greater than 0."
}

if ($TailWindowSeconds -le 0) {
  throw "TailWindowSeconds must be greater than 0."
}

function Convert-BytesToMiB {
  param([double]$Bytes)

  [math]::Round($Bytes / 1MB, 2)
}

function New-ProcessIdSet {
  $set = [System.Collections.Generic.HashSet[int]]::new()
  $processes = @(
    Get-CimInstance Win32_Process `
      -Filter "Name = 'msedgewebview2.exe'" `
      -ErrorAction SilentlyContinue
  )
  foreach ($process in $processes) {
    [void]$set.Add([int]$process.ProcessId)
  }
  $set
}

function Get-DescendantProcessIds {
  param(
    [int]$RootPid,
    [object[]]$ProcessRows
  )

  $childrenByParent = @{}
  foreach ($row in $ProcessRows) {
    $parentId = [int]$row.ParentProcessId
    if (-not $childrenByParent.ContainsKey($parentId)) {
      $childrenByParent[$parentId] = [System.Collections.Generic.List[int]]::new()
    }
    $childrenByParent[$parentId].Add([int]$row.ProcessId)
  }

  $seen = [System.Collections.Generic.HashSet[int]]::new()
  $queue = [System.Collections.Generic.Queue[int]]::new()
  $queue.Enqueue($RootPid)

  while ($queue.Count -gt 0) {
    $current = $queue.Dequeue()
    if (-not $childrenByParent.ContainsKey($current)) {
      continue
    }

    foreach ($childPid in $childrenByParent[$current]) {
      if ($seen.Add($childPid)) {
        $queue.Enqueue($childPid)
      }
    }
  }

  @($seen | ForEach-Object { [int]$_ })
}

function Get-MemorySample {
  param(
    [int]$AppPid,
    [datetime]$StartedAt,
    [System.Collections.Generic.HashSet[int]]$BaselineWebViewPids
  )

  $now = Get-Date
  $processRows = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue)
  $descendantIds = @(Get-DescendantProcessIds -RootPid $AppPid -ProcessRows $processRows)

  $trackedIds = [System.Collections.Generic.HashSet[int]]::new()
  [void]$trackedIds.Add($AppPid)
  foreach ($pid in $descendantIds) {
    [void]$trackedIds.Add([int]$pid)
  }

  $webviewRows = @($processRows | Where-Object { $_.Name -ieq "msedgewebview2.exe" })
  $webviewIds = [System.Collections.Generic.HashSet[int]]::new()

  # WebView2 helpers are not always parented below the Tauri app, so the
  # process group includes child helpers plus WebView2 pids created after start.
  foreach ($row in $webviewRows) {
    $pid = [int]$row.ProcessId
    if ($trackedIds.Contains($pid) -or -not $BaselineWebViewPids.Contains($pid)) {
      [void]$webviewIds.Add($pid)
    }
  }

  $webviewProcesses = @()
  $webviewIdList = @($webviewIds | ForEach-Object { [int]$_ } | Sort-Object)
  if ($webviewIdList.Count -gt 0) {
    $webviewProcesses = @(Get-Process -Id $webviewIdList -ErrorAction SilentlyContinue)
  }

  $webviewPrivateBytes = 0.0
  foreach ($process in $webviewProcesses) {
    $webviewPrivateBytes += [double]$process.PrivateMemorySize64
  }

  $appPrivateBytes = 0.0
  $appProcess = Get-Process -Id $AppPid -ErrorAction SilentlyContinue
  if ($null -ne $appProcess) {
    $appPrivateBytes = [double]$appProcess.PrivateMemorySize64
  }

  [pscustomobject]@{
    timestamp = $now.ToUniversalTime().ToString("o")
    elapsedSeconds = [math]::Round(($now - $StartedAt).TotalSeconds, 3)
    appPid = $AppPid
    appPrivateBytes = [int64]$appPrivateBytes
    appPrivateMb = Convert-BytesToMiB $appPrivateBytes
    webviewPrivateBytes = [int64]$webviewPrivateBytes
    webviewPrivateMb = Convert-BytesToMiB $webviewPrivateBytes
    webviewProcessCount = $webviewProcesses.Count
    trackedProcessCount = $trackedIds.Count
    webviewPids = $webviewIdList
  }
}

function Get-TailSlopeMbPerMinute {
  param([object[]]$Samples)

  if ($Samples.Count -lt 2) {
    return 0.0
  }

  $sumX = 0.0
  $sumY = 0.0
  foreach ($sample in $Samples) {
    $sumX += [double]$sample.elapsedSeconds / 60.0
    $sumY += [double]$sample.webviewPrivateMb
  }

  $meanX = $sumX / $Samples.Count
  $meanY = $sumY / $Samples.Count
  $numerator = 0.0
  $denominator = 0.0

  foreach ($sample in $Samples) {
    $x = ([double]$sample.elapsedSeconds / 60.0) - $meanX
    $y = [double]$sample.webviewPrivateMb - $meanY
    $numerator += $x * $y
    $denominator += $x * $x
  }

  if ($denominator -eq 0.0) {
    return 0.0
  }

  [math]::Round($numerator / $denominator, 3)
}

function Write-Results {
  param(
    [object[]]$Samples,
    [object]$Analysis,
    [string]$ResolvedBinary,
    [datetime]$StartedAt,
    [datetime]$EndedAt,
    [string]$OutputDirectory
  )

  New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null

  $jsonPath = Join-Path $OutputDirectory "webview-memory-results.json"
  $csvPath = Join-Path $OutputDirectory "webview-memory-samples.csv"

  $result = [ordered]@{
    schemaVersion = 1
    metric = "webview2_private_memory"
    binary = $ResolvedBinary
    startedAt = $StartedAt.ToUniversalTime().ToString("o")
    endedAt = $EndedAt.ToUniversalTime().ToString("o")
    timing = [ordered]@{
      warmupSeconds = $WarmupSeconds
      durationSeconds = $DurationSeconds
      sampleIntervalSeconds = $SampleIntervalSeconds
      tailWindowSeconds = $TailWindowSeconds
    }
    thresholds = [ordered]@{
      maxGrowthMb = $MaxGrowthMb
      minTailSlopeMbPerMinute = $MinTailSlopeMbPerMinute
    }
    analysis = $Analysis
    samples = $Samples
  }

  $result | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 -Path $jsonPath
  $Samples |
    Select-Object timestamp, elapsedSeconds, appPid, appPrivateBytes, appPrivateMb, webviewPrivateBytes, webviewPrivateMb, webviewProcessCount, trackedProcessCount, @{Name = "webviewPids"; Expression = { $_.webviewPids -join ";" }} |
    Export-Csv -NoTypeInformation -Encoding utf8 -Path $csvPath

  Write-Host "Wrote WebView2 memory results to $jsonPath"
  Write-Host "Wrote WebView2 memory samples to $csvPath"
}

$resolvedBinary = (Resolve-Path $Binary).Path
$binaryName = [System.IO.Path]::GetFileNameWithoutExtension($resolvedBinary)
$existingApps = @(Get-Process -Name $binaryName -ErrorAction SilentlyContinue)
if ($existingApps.Count -gt 0) {
  Write-Warning "Existing $binaryName process(es) found: $($existingApps.Id -join ', '). Tauri single-instance behavior can invalidate this run."
}

$baselineWebViewPids = New-ProcessIdSet
$targetProcess = $null
$samples = @()
$startedAt = Get-Date

try {
  Write-Host "Starting $resolvedBinary"
  $targetProcess = Start-Process -FilePath $resolvedBinary -WorkingDirectory (Split-Path $resolvedBinary) -PassThru

  $totalSeconds = $WarmupSeconds + $DurationSeconds
  Write-Host "Sampling WebView2 private memory for $totalSeconds seconds (warmup: ${WarmupSeconds}s, duration: ${DurationSeconds}s, interval: ${SampleIntervalSeconds}s)."

  while ($true) {
    Start-Sleep -Seconds ([math]::Min($SampleIntervalSeconds, [math]::Max(1, $totalSeconds - [int]((Get-Date) - $startedAt).TotalSeconds)))

    if ($targetProcess.HasExited) {
      throw "Target process exited before measurement completed. ExitCode: $($targetProcess.ExitCode)"
    }

    $sample = Get-MemorySample -AppPid $targetProcess.Id -StartedAt $startedAt -BaselineWebViewPids $baselineWebViewPids
    $samples += $sample
    Write-Host ("t={0,7:N1}s webviewPrivate={1,8:N2} MiB webviewProcesses={2}" -f $sample.elapsedSeconds, $sample.webviewPrivateMb, $sample.webviewProcessCount)

    if ($sample.elapsedSeconds -ge $totalSeconds) {
      break
    }
  }
}
finally {
  if ($null -ne $targetProcess -and -not $targetProcess.HasExited) {
    Write-Host "Stopping $($targetProcess.ProcessName) ($($targetProcess.Id))"
    Stop-Process -Id $targetProcess.Id -Force -ErrorAction SilentlyContinue
    Wait-Process -Id $targetProcess.Id -Timeout 10 -ErrorAction SilentlyContinue
  }
}

$endedAt = Get-Date
$measurementSamples = @($samples | Where-Object { $_.elapsedSeconds -ge $WarmupSeconds })
$baselineSample = $measurementSamples | Select-Object -First 1
$finalSample = $measurementSamples | Select-Object -Last 1
$webviewSampleCount = @($measurementSamples | Where-Object { $_.webviewProcessCount -gt 0 }).Count

if ($null -eq $baselineSample -or $null -eq $finalSample) {
  $analysis = [ordered]@{
    status = "invalid"
    reason = "No samples were collected after warmup."
    sampleCount = $samples.Count
  }
  Write-Results -Samples $samples -Analysis $analysis -ResolvedBinary $resolvedBinary -StartedAt $startedAt -EndedAt $endedAt -OutputDirectory $OutputDirectory
  throw "No samples were collected after warmup."
}

$growthMb = [math]::Round(([double]$finalSample.webviewPrivateMb - [double]$baselineSample.webviewPrivateMb), 2)
$tailStartSeconds = [math]::Max([double]$WarmupSeconds, [double]$finalSample.elapsedSeconds - [double]$TailWindowSeconds)
$tailSamples = @($measurementSamples | Where-Object { $_.elapsedSeconds -ge $tailStartSeconds })
$tailSlope = Get-TailSlopeMbPerMinute -Samples $tailSamples
$maxWebViewPrivateMb = ($measurementSamples | Measure-Object -Property webviewPrivateMb -Maximum).Maximum
$thresholdExceeded = $growthMb -ge $MaxGrowthMb -and $tailSlope -gt $MinTailSlopeMbPerMinute

$status = "pass"
$reason = "WebView2 private memory stayed below the growth heuristic."
if ($webviewSampleCount -eq 0) {
  $status = "invalid"
  $reason = "No msedgewebview2.exe processes were observed after warmup."
} elseif ($thresholdExceeded) {
  $status = "fail"
  $reason = "WebView2 private memory exceeded growth and tail-slope thresholds."
}

$analysis = [ordered]@{
  status = $status
  reason = $reason
  sampleCount = $samples.Count
  measurementSampleCount = $measurementSamples.Count
  webviewSampleCount = $webviewSampleCount
  baselineElapsedSeconds = $baselineSample.elapsedSeconds
  baselineWebViewPrivateMb = $baselineSample.webviewPrivateMb
  finalElapsedSeconds = $finalSample.elapsedSeconds
  finalWebViewPrivateMb = $finalSample.webviewPrivateMb
  maxWebViewPrivateMb = [math]::Round([double]$maxWebViewPrivateMb, 2)
  growthMb = $growthMb
  tailSlopeMbPerMinute = $tailSlope
  tailSampleCount = $tailSamples.Count
}

Write-Results -Samples $samples -Analysis $analysis -ResolvedBinary $resolvedBinary -StartedAt $startedAt -EndedAt $endedAt -OutputDirectory $OutputDirectory

Write-Host ("WebView2 private memory: baseline={0:N2} MiB final={1:N2} MiB growth={2:N2} MiB tailSlope={3:N3} MiB/min status={4}" -f $analysis.baselineWebViewPrivateMb, $analysis.finalWebViewPrivateMb, $analysis.growthMb, $analysis.tailSlopeMbPerMinute, $analysis.status)

if ($status -eq "invalid") {
  Write-Host "::warning title=WebView2 memory perf invalid::$reason"
  exit 1
}

if ($status -eq "fail") {
  Write-Host "::warning title=WebView2 memory growth::$reason Growth ${growthMb}MiB, tail slope ${tailSlope}MiB/min."
  exit 1
}
