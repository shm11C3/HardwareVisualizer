<#
.SYNOPSIS
Captures a clean-room Super I/O / PawnIO LpcIO diagnostic bundle.

.DESCRIPTION
This script is for #1635 hardware validation, not production sampling.
It loads the locally installed PawnIO LpcIO module, holds the shared
Access_ISABUS mutex, records raw chip-id bytes for both standard Super I/O
slots, and optionally probes the Nuvoton Hardware Monitor logical-device base
and raw Hardware Monitor bytes.

The optional Hardware Monitor byte read is intentionally behind -IncludeHmRead.
It writes only read-transaction plumbing values: configuration-mode entry/exit,
logical-device selection, hardware-monitor index selection, and bank selection.
It does not write fan-control, PWM, threshold, alarm, GPIO, or activation
registers.

Run from an Administrator PowerShell session for real PawnIO access:

  powershell -ExecutionPolicy Bypass -File .\scripts\diagnostics\capture-superio-hm-dump.ps1 -IncludeBaseDiscovery -IncludeHmRead

Use -DryRun to validate script discovery/JSON output without opening PawnIO.
#>

[CmdletBinding()]
param(
  [string]$OutputPath = "",
  [string]$PawnIoRoot = "",
  [int]$MutexTimeoutMs = 500,
  [switch]$IncludeBaseDiscovery,
  [switch]$IncludeHmRead,
  [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
  $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
  $OutputPath = Join-Path "tmp" "superio-hm-dump-$timestamp.json"
}

function Format-HResult {
  param([int]$Value)
  $unsigned = [BitConverter]::ToUInt32([BitConverter]::GetBytes($Value), 0)
  return ("0x{0:X8}" -f $unsigned)
}

function Test-HResultSucceeded {
  param([int]$Value)
  return $Value -ge 0
}

function Test-IsElevated {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($identity)
  return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-CimSummary {
  param([string]$ClassName)
  try {
    $instance = Get-CimInstance -ClassName $ClassName | Select-Object -First 1
    switch ($ClassName) {
      "Win32_BaseBoard" {
        return [ordered]@{
          manufacturer = $instance.Manufacturer
          product = $instance.Product
          version = $instance.Version
        }
      }
      "Win32_Processor" {
        return [ordered]@{
          manufacturer = $instance.Manufacturer
          name = $instance.Name
          cores = $instance.NumberOfCores
          logicalProcessors = $instance.NumberOfLogicalProcessors
        }
      }
      "Win32_OperatingSystem" {
        return [ordered]@{
          caption = $instance.Caption
          version = $instance.Version
          buildNumber = $instance.BuildNumber
          architecture = $instance.OSArchitecture
        }
      }
      default {
        return [ordered]@{ name = $instance.Name }
      }
    }
  } catch {
    return [pscustomobject]@{ error = $_.Exception.Message }
  }
}

function Convert-ByteToHex {
  param([AllowNull()]$Value)
  if ($null -eq $Value) {
    return $null
  }
  return ("0x{0:X2}" -f ([int]$Value -band 0xFF))
}

function Convert-WordToHex {
  param([AllowNull()]$Value)
  if ($null -eq $Value) {
    return $null
  }
  return ("0x{0:X4}" -f ([int]$Value -band 0xFFFF))
}

function Convert-CallResult {
  param([pscustomobject]$Call)
  if ($null -eq $Call) {
    return $null
  }

  return [ordered]@{
    function = $Call.function
    input = @($Call.input)
    hresult = $Call.hresult
    succeeded = $Call.succeeded
    returnSize = $Call.returnSize
    output = @($Call.output)
    error = $Call.error
  }
}

function Get-PawnIoInstallCandidates {
  $candidates = New-Object System.Collections.Generic.List[string]

  if (-not [string]::IsNullOrWhiteSpace($PawnIoRoot)) {
    $candidates.Add($PawnIoRoot)
  }

  try {
    $reg = Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO" -Name "InstallLocation" -ErrorAction Stop
    if (-not [string]::IsNullOrWhiteSpace($reg.InstallLocation)) {
      $candidates.Add($reg.InstallLocation)
    }
  } catch {
    # Registry install location is optional; keep probing standard paths.
  }

  foreach ($name in @("ProgramFiles", "ProgramW6432", "ProgramFiles(x86)")) {
    $root = [Environment]::GetEnvironmentVariable($name)
    if (-not [string]::IsNullOrWhiteSpace($root)) {
      $candidates.Add((Join-Path $root "PawnIO"))
    }
  }

  return @($candidates | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique)
}

function Find-FirstNamedFile {
  param(
    [string[]]$Roots,
    [string[]]$Names,
    [int]$MaxDepth = 4
  )

  foreach ($name in $Names) {
    foreach ($root in $Roots) {
      if (-not (Test-Path -LiteralPath $root)) {
        continue
      }

      $direct = Join-Path $root $name
      if (Test-Path -LiteralPath $direct -PathType Leaf) {
        return (Resolve-Path -LiteralPath $direct).Path
      }

      try {
        $match = Get-ChildItem -LiteralPath $root -Recurse -File -Depth $MaxDepth -ErrorAction SilentlyContinue |
          Where-Object { $_.Name -ieq $name } |
          Select-Object -First 1
        if ($null -ne $match) {
          return $match.FullName
        }
      } catch {
        # Some install subfolders may not be enumerable; keep looking.
      }
    }
  }

  return $null
}

Add-Type -TypeDefinition @"
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class PawnIoNative {
  [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  private static extern IntPtr LoadLibraryW(string lpFileName);

  [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Ansi, BestFitMapping = false)]
  private static extern IntPtr GetProcAddress(IntPtr hModule, string procName);

  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern bool FreeLibrary(IntPtr hModule);

  [UnmanagedFunctionPointer(CallingConvention.Winapi)]
  private delegate int PawnIoVersion(out UInt32 version);

  [UnmanagedFunctionPointer(CallingConvention.Winapi)]
  private delegate int PawnIoOpen(out IntPtr handle);

  [UnmanagedFunctionPointer(CallingConvention.Winapi)]
  private delegate int PawnIoLoad(IntPtr handle, byte[] blob, UIntPtr blobSize);

  [UnmanagedFunctionPointer(CallingConvention.Winapi)]
  private delegate int PawnIoExecute(
    IntPtr handle,
    [MarshalAs(UnmanagedType.LPStr)] string name,
    UInt64[] input,
    UIntPtr inputCells,
    UInt64[] output,
    UIntPtr outputCells,
    out UIntPtr returnSize);

  [UnmanagedFunctionPointer(CallingConvention.Winapi)]
  private delegate int PawnIoClose(IntPtr handle);

  public static IntPtr LoadLibrary(string path) {
    IntPtr library = LoadLibraryW(path);
    if (library == IntPtr.Zero) {
      throw new Win32Exception(Marshal.GetLastWin32Error(), "LoadLibraryW failed for " + path);
    }
    return library;
  }

  public static int Version(IntPtr library, out UInt32 version) {
    return GetDelegate<PawnIoVersion>(library, "pawnio_version")(out version);
  }

  public static int Open(IntPtr library, out IntPtr handle) {
    return GetDelegate<PawnIoOpen>(library, "pawnio_open")(out handle);
  }

  public static int Load(IntPtr library, IntPtr handle, byte[] blob) {
    return GetDelegate<PawnIoLoad>(library, "pawnio_load")(handle, blob, (UIntPtr)blob.Length);
  }

  public static int Execute(
    IntPtr library,
    IntPtr handle,
    string name,
    UInt64[] input,
    UInt64[] output,
    UInt64 outputCells,
    out UInt64 returnSize) {
    UInt64[] safeInput = input ?? new UInt64[0];
    UInt64[] safeOutput = output ?? new UInt64[1];
    UIntPtr nativeReturnSize;
    int hr = GetDelegate<PawnIoExecute>(library, "pawnio_execute")(
      handle,
      name,
      safeInput,
      (UIntPtr)safeInput.Length,
      safeOutput,
      (UIntPtr)outputCells,
      out nativeReturnSize);
    returnSize = nativeReturnSize.ToUInt64();
    return hr;
  }

  public static int Close(IntPtr library, IntPtr handle) {
    return GetDelegate<PawnIoClose>(library, "pawnio_close")(handle);
  }

  private static T GetDelegate<T>(IntPtr library, string name) where T : class {
    IntPtr proc = GetProcAddress(library, name);
    if (proc == IntPtr.Zero) {
      throw new MissingMethodException("Missing PawnIO export: " + name);
    }
    return Marshal.GetDelegateForFunctionPointer(proc, typeof(T)) as T;
  }
}
"@

$script:PawnIoLibrary = [IntPtr]::Zero
$script:PawnIoHandle = [IntPtr]::Zero

function Invoke-PawnIo {
  param(
    [string]$Name,
    [UInt64[]]$InputCells = @(),
    [int]$OutputCells = 0
  )

  $output = New-Object UInt64[] ([Math]::Max(1, $OutputCells))
  [UInt64]$returnSize = 0
  try {
    $hr = [PawnIoNative]::Execute($script:PawnIoLibrary, $script:PawnIoHandle, $Name, $InputCells, $output, ([UInt64]$OutputCells), [ref]$returnSize)
    $succeeded = Test-HResultSucceeded $hr
    $visibleOutput = @()
    if ($succeeded -and $OutputCells -gt 0) {
      $visibleOutput = @($output | Select-Object -First ([Math]::Min($OutputCells, [int]$returnSize)))
    }
    return [pscustomobject]@{
      function = $Name
      input = @($InputCells)
      hresult = Format-HResult $hr
      succeeded = $succeeded
      returnSize = $returnSize
      output = $visibleOutput
      error = $null
    }
  } catch {
    return [pscustomobject]@{
      function = $Name
      input = @($InputCells)
      hresult = $null
      succeeded = $false
      returnSize = 0
      output = @()
      error = $_.Exception.Message
    }
  }
}

function Invoke-PawnIoRequired {
  param(
    [string]$Name,
    [UInt64[]]$InputCells = @(),
    [int]$OutputCells = 0
  )
  $call = Invoke-PawnIo -Name $Name -InputCells $InputCells -OutputCells $OutputCells
  if (-not $call.succeeded) {
    $reason = if ($call.hresult) { $call.hresult } else { $call.error }
    throw "$Name failed: $reason"
  }
  return $call
}

function Read-SuperIoRegister {
  param([byte]$Register)
  $call = Invoke-PawnIoRequired -Name "ioctl_superio_inb" -InputCells @([UInt64]$Register) -OutputCells 1
  return [int]$call.output[0]
}

function Write-SuperIoRegister {
  param([byte]$Register, [byte]$Value)
  [void](Invoke-PawnIoRequired -Name "ioctl_superio_outb" -InputCells @([UInt64]$Register, [UInt64]$Value) -OutputCells 0)
}

function Write-PortByte {
  param([UInt16]$Port, [byte]$Value)
  return Invoke-PawnIoRequired -Name "ioctl_pio_outb" -InputCells @([UInt64]$Port, [UInt64]$Value) -OutputCells 0
}

function Read-PortByteCall {
  param([UInt16]$Port)
  return Invoke-PawnIo -Name "ioctl_pio_inb" -InputCells @([UInt64]$Port) -OutputCells 1
}

function Enter-NuvotonConfig {
  param([UInt16]$IndexPort)
  [void](Write-PortByte -Port $IndexPort -Value 0x87)
  [void](Write-PortByte -Port $IndexPort -Value 0x87)
}

function Exit-NuvotonConfig {
  param([UInt16]$IndexPort)
  return Invoke-PawnIo -Name "ioctl_pio_outb" -InputCells @([UInt64]$IndexPort, [UInt64]0xAA) -OutputCells 0
}

function Enter-IteConfig {
  param([int]$Slot, [UInt16]$IndexPort)
  $fourth = if ($Slot -eq 0) { 0x55 } else { 0xAA }
  foreach ($key in @(0x87, 0x01, 0x55, $fourth)) {
    [void](Write-PortByte -Port $IndexPort -Value ([byte]$key))
  }
}

function Exit-IteConfig {
  return Invoke-PawnIo -Name "ioctl_superio_outb" -InputCells @([UInt64]0x02, [UInt64]0x02) -OutputCells 0
}

function Get-ChipIdFields {
  param([int]$High, [int]$Low)
  $chipId = (($High -band 0xFF) -shl 8) -bor ($Low -band 0xFF)
  $absent = (($High -eq 0x00 -and $Low -eq 0x00) -or ($High -eq 0xFF -and $Low -eq 0xFF))
  return [ordered]@{
    idHigh = $High
    idHighHex = Convert-ByteToHex $High
    idLow = $Low
    idLowHex = Convert-ByteToHex $Low
    chipId = if ($absent) { $null } else { $chipId }
    chipIdHex = if ($absent) { $null } else { Convert-WordToHex $chipId }
    absent = $absent
  }
}

function Test-ValidIoBase {
  param([int]$Base)
  return $Base -ne 0x0000 -and $Base -ne 0xFFFF -and $Base -ge 0x0100 -and $Base -le 0x0FFE
}

function Read-HardwareMonitorRegister {
  param([UInt16]$IndexPort, [UInt16]$DataPort, [byte]$Register)
  $writeIndex = Invoke-PawnIo -Name "ioctl_pio_outb" -InputCells @([UInt64]$IndexPort, [UInt64]$Register) -OutputCells 0
  if (-not $writeIndex.succeeded) {
    return [ordered]@{
      register = [int]$Register
      registerHex = Convert-ByteToHex $Register
      writeIndex = Convert-CallResult $writeIndex
      readData = $null
      value = $null
      valueHex = $null
    }
  }

  $readData = Read-PortByteCall -Port $DataPort
  $value = if ($readData.succeeded -and $readData.output.Count -gt 0) { [int]$readData.output[0] } else { $null }
  return [ordered]@{
    register = [int]$Register
    registerHex = Convert-ByteToHex $Register
    writeIndex = Convert-CallResult $writeIndex
    readData = Convert-CallResult $readData
    value = $value
    valueHex = Convert-ByteToHex $value
  }
}

function Read-NuvotonHmCandidateDump {
  param([UInt16]$BaseAddress)

  $indexPort = [UInt16]($BaseAddress + 0x05)
  $dataPort = [UInt16]($BaseAddress + 0x06)

  $dump = [ordered]@{
    note = "Raw dump only. Do not decode or ship from this output until the matching spec is implementation-ready."
    baseAddress = [int]$BaseAddress
    baseAddressHex = Convert-WordToHex $BaseAddress
    indexPort = [int]$indexPort
    indexPortHex = Convert-WordToHex $indexPort
    dataPort = [int]$dataPort
    dataPortHex = Convert-WordToHex $dataPort
    findBarsBeforeRead = Convert-CallResult (Invoke-PawnIo -Name "ioctl_find_bars" -InputCells ([UInt64[]]@()) -OutputCells 0)
    bankSelect = $null
    bank4Reads = @()
  }

  $bankIndex = Invoke-PawnIo -Name "ioctl_pio_outb" -InputCells @([UInt64]$indexPort, [UInt64]0x4E) -OutputCells 0
  $bankData = $null
  if ($bankIndex.succeeded) {
    $bankData = Invoke-PawnIo -Name "ioctl_pio_outb" -InputCells @([UInt64]$dataPort, [UInt64]0x04) -OutputCells 0
  }
  $dump.bankSelect = [ordered]@{
    indexRegisterWrite = Convert-CallResult $bankIndex
    bankValueWrite = Convert-CallResult $bankData
  }

  if ($bankIndex.succeeded -and $null -ne $bankData -and $bankData.succeeded) {
    $registers = @(0x90,0x91,0x92,0x93,0x94,0x95,0xB0,0xB1,0xB2,0xB3,0xB4,0xB5,0xB6,0xB7,0xB8,0xB9,0xBA,0xBB,0xC0,0xC1,0xC2,0xC3,0xC4,0xC5,0xC6,0xC7,0xC8,0xC9,0xCA,0xCB,0xCC,0xCD,0xCE,0xCF)
    $dump.bank4Reads = @($registers | ForEach-Object {
      Read-HardwareMonitorRegister -IndexPort $indexPort -DataPort $dataPort -Register ([byte]$_)
    })
  }

  return $dump
}

function Probe-NuvotonSlot {
  param([int]$Slot, [UInt16]$IndexPort)

  $result = [ordered]@{
    vendor = "nuvoton"
    error = $null
    exit = $null
    chipId = $null
    baseDiscovery = $null
    hardwareMonitorDump = $null
  }

  try {
    Enter-NuvotonConfig -IndexPort $IndexPort
    $high = Read-SuperIoRegister -Register 0x20
    $low = Read-SuperIoRegister -Register 0x21
    $result.chipId = Get-ChipIdFields -High $high -Low $low

    if ($IncludeBaseDiscovery -and -not $result.chipId.absent) {
      Write-SuperIoRegister -Register 0x07 -Value 0x0B
      $cr30 = Read-SuperIoRegister -Register 0x30
      $cr60 = Read-SuperIoRegister -Register 0x60
      $cr61 = Read-SuperIoRegister -Register 0x61
      $cr64 = Read-SuperIoRegister -Register 0x64
      $cr65 = Read-SuperIoRegister -Register 0x65
      $normalBase = (($cr60 -band 0xFF) -shl 8) -bor ($cr61 -band 0xFF)
      $readOnlyBase = (($cr64 -band 0xFF) -shl 8) -bor ($cr65 -band 0xFF)

      $result.baseDiscovery = [ordered]@{
        logicalDevice = 0x0B
        logicalDeviceHex = "0x0B"
        cr30 = $cr30
        cr30Hex = Convert-ByteToHex $cr30
        activeBitSet = (($cr30 -band 0x01) -ne 0)
        cr60 = $cr60
        cr60Hex = Convert-ByteToHex $cr60
        cr61 = $cr61
        cr61Hex = Convert-ByteToHex $cr61
        normalBase = $normalBase
        normalBaseHex = Convert-WordToHex $normalBase
        normalBaseValid = Test-ValidIoBase $normalBase
        cr64 = $cr64
        cr64Hex = Convert-ByteToHex $cr64
        cr65 = $cr65
        cr65Hex = Convert-ByteToHex $cr65
        readOnlyBase = $readOnlyBase
        readOnlyBaseHex = Convert-WordToHex $readOnlyBase
        readOnlyBaseValid = Test-ValidIoBase $readOnlyBase
        findBarsInConfigMode = Convert-CallResult (Invoke-PawnIo -Name "ioctl_find_bars" -InputCells ([UInt64[]]@()) -OutputCells 0)
      }

      if ($IncludeHmRead -and (Test-ValidIoBase $normalBase)) {
        $result.hardwareMonitorDump = Read-NuvotonHmCandidateDump -BaseAddress ([UInt16]$normalBase)
      }
    }
  } catch {
    $result.error = $_.Exception.Message
  } finally {
    $result.exit = Convert-CallResult (Exit-NuvotonConfig -IndexPort $IndexPort)
  }

  return $result
}

function Probe-IteSlot {
  param([int]$Slot, [UInt16]$IndexPort)

  $result = [ordered]@{
    vendor = "ite"
    error = $null
    exit = $null
    chipId = $null
  }

  try {
    Enter-IteConfig -Slot $Slot -IndexPort $IndexPort
    $high = Read-SuperIoRegister -Register 0x20
    $low = Read-SuperIoRegister -Register 0x21
    $result.chipId = Get-ChipIdFields -High $high -Low $low
  } catch {
    $result.error = $_.Exception.Message
  } finally {
    $result.exit = Convert-CallResult (Exit-IteConfig)
  }

  return $result
}

function Probe-SuperIoSlots {
  $slots = @(
    [pscustomobject]@{ Slot = 0; IndexPort = [UInt16]0x2E; DataPort = [UInt16]0x2F },
    [pscustomobject]@{ Slot = 1; IndexPort = [UInt16]0x4E; DataPort = [UInt16]0x4F }
  )

  return @($slots | ForEach-Object {
    $slot = $_
    $slotResult = [ordered]@{
      slot = $slot.Slot
      indexPort = [int]$slot.IndexPort
      indexPortHex = Convert-WordToHex $slot.IndexPort
      dataPort = [int]$slot.DataPort
      dataPortHex = Convert-WordToHex $slot.DataPort
      selectSlot = $null
      findBarsBeforeConfig = $null
      attempts = @()
    }

    $select = Invoke-PawnIo -Name "ioctl_select_slot" -InputCells ([UInt64[]]@([UInt64]$slot.Slot)) -OutputCells 0
    $slotResult.selectSlot = Convert-CallResult $select
    if ($select.succeeded) {
      $slotResult.findBarsBeforeConfig = Convert-CallResult (Invoke-PawnIo -Name "ioctl_find_bars" -InputCells ([UInt64[]]@()) -OutputCells 0)
      $slotResult.attempts = @(
        Probe-NuvotonSlot -Slot $slot.Slot -IndexPort $slot.IndexPort
        Probe-IteSlot -Slot $slot.Slot -IndexPort $slot.IndexPort
      )
    }

    $slotResult
  })
}

$installCandidates = Get-PawnIoInstallCandidates
$dllPath = Find-FirstNamedFile -Roots $installCandidates -Names @("PawnIOLib.dll")
$modulePath = Find-FirstNamedFile -Roots $installCandidates -Names @("LpcIO.bin", "LpcIO.amx")

$result = [ordered]@{
  schema = "hardwarevisualizer.superio-hm-dump.v1"
  capturedAt = (Get-Date).ToString("o")
  dryRun = [bool]$DryRun
  elevated = Test-IsElevated
  includeBaseDiscovery = [bool]$IncludeBaseDiscovery
  includeHmRead = [bool]$IncludeHmRead
  safety = [ordered]@{
    purpose = "Independent hardware-validation dump for spec authoring; not a production implementation path."
    writesAllowed = @(
      "Super I/O configuration-mode entry/exit",
      "Super I/O logical-device selection",
      "Hardware Monitor index selection",
      "Hardware Monitor bank selection when -IncludeHmRead is set"
    )
    writesProhibited = @(
      "fan-control/PWM registers",
      "threshold/limit registers",
      "alarm-clear registers",
      "GPIO registers",
      "activation registers"
    )
  }
  machine = [ordered]@{
    baseBoard = Get-CimSummary -ClassName "Win32_BaseBoard"
    processor = Get-CimSummary -ClassName "Win32_Processor"
    operatingSystem = Get-CimSummary -ClassName "Win32_OperatingSystem"
  }
  pawnio = [ordered]@{
    installCandidates = @($installCandidates)
    dllPath = $dllPath
    modulePath = $modulePath
    libraryLoadable = $false
    version = $null
    versionHex = $null
    open = $null
    moduleLoad = $null
  }
  mutex = $null
  slots = @()
  error = $null
}

try {
  if ($null -eq $dllPath) {
    throw "PawnIOLib.dll was not found. Pass -PawnIoRoot if PawnIO is installed in a non-standard location."
  }
  if ($null -eq $modulePath) {
    throw "LpcIO.bin or LpcIO.amx was not found. Pass -PawnIoRoot if the LpcIO module is installed in a non-standard location."
  }

  if ($DryRun) {
    return
  }

  $script:PawnIoLibrary = [PawnIoNative]::LoadLibrary($dllPath)
  $result.pawnio.libraryLoadable = $true

  [UInt32]$version = 0
  $versionHr = [PawnIoNative]::Version($script:PawnIoLibrary, [ref]$version)
  $result.pawnio.versionCall = [ordered]@{
    hresult = Format-HResult $versionHr
    succeeded = Test-HResultSucceeded $versionHr
  }
  if (Test-HResultSucceeded $versionHr) {
    $result.pawnio.version = [int64]$version
    $result.pawnio.versionHex = ("0x{0:X8}" -f $version)
  }

  $openHr = [PawnIoNative]::Open($script:PawnIoLibrary, [ref]$script:PawnIoHandle)
  $result.pawnio.open = [ordered]@{
    hresult = Format-HResult $openHr
    succeeded = (Test-HResultSucceeded $openHr) -and ($script:PawnIoHandle -ne [IntPtr]::Zero)
  }
  if (-not $result.pawnio.open.succeeded) {
    throw "pawnio_open failed: $($result.pawnio.open.hresult)"
  }

  $blob = [IO.File]::ReadAllBytes($modulePath)
  $loadHr = [PawnIoNative]::Load($script:PawnIoLibrary, $script:PawnIoHandle, $blob)
  $result.pawnio.moduleLoad = [ordered]@{
    hresult = Format-HResult $loadHr
    succeeded = Test-HResultSucceeded $loadHr
    bytes = $blob.Length
  }
  if (-not $result.pawnio.moduleLoad.succeeded) {
    throw "pawnio_load failed: $($result.pawnio.moduleLoad.hresult)"
  }

  $mutexName = "Global\Access_ISABUS.HTP.Method"
  try {
    $mutex = [Threading.Mutex]::OpenExisting($mutexName)
    $mutexSource = "opened-existing"
  } catch [Threading.WaitHandleCannotBeOpenedException] {
    $mutex = [Threading.Mutex]::new($false, $mutexName)
    $mutexSource = "created"
  }

  $acquired = $false
  try {
    $acquired = $mutex.WaitOne($MutexTimeoutMs)
    $result.mutex = [ordered]@{
      name = $mutexName
      source = $mutexSource
      timeoutMs = $MutexTimeoutMs
      acquired = $acquired
    }
    if (-not $acquired) {
      throw "Timed out waiting for $mutexName"
    }

    $result.slots = Probe-SuperIoSlots
  } finally {
    if ($acquired) {
      $mutex.ReleaseMutex()
    }
    if ($null -ne $mutex) {
      $mutex.Dispose()
    }
  }
} catch {
  $result.error = $_.Exception.Message
} finally {
  if ($script:PawnIoHandle -ne [IntPtr]::Zero -and $script:PawnIoLibrary -ne [IntPtr]::Zero) {
    try {
      $closeHr = [PawnIoNative]::Close($script:PawnIoLibrary, $script:PawnIoHandle)
      $result.pawnio.close = [ordered]@{
        hresult = Format-HResult $closeHr
        succeeded = Test-HResultSucceeded $closeHr
      }
    } catch {
      $result.pawnio.close = [ordered]@{
        hresult = $null
        succeeded = $false
        error = $_.Exception.Message
      }
    }
  }
  if ($script:PawnIoLibrary -ne [IntPtr]::Zero) {
    [void][PawnIoNative]::FreeLibrary($script:PawnIoLibrary)
  }

  $outputDirectory = Split-Path -Parent $OutputPath
  if (-not [string]::IsNullOrWhiteSpace($outputDirectory) -and -not (Test-Path -LiteralPath $outputDirectory)) {
    New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
  }

  $json = ($result | ConvertTo-Json -Depth 32) -replace "`r`n", "`n"
  $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText([System.IO.Path]::GetFullPath($OutputPath), $json + "`n", $utf8NoBom)
  Write-Host "Wrote Super I/O diagnostic bundle to $OutputPath"
  if (-not $result.elevated) {
    Write-Warning "This run was not elevated. Re-run from Administrator PowerShell for real PawnIO driver access if pawnio_open failed with 0x80070005."
  }
  if ($IncludeHmRead) {
    Write-Warning "-IncludeHmRead performs raw HM index/data read plumbing for validation only. Attach the JSON to the spec-author handoff; do not use it as production decode evidence until the spec is implementation-ready."
  }
}
