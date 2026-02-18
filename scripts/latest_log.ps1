Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Prints the resolved log directory, the newest log file, and a tail snippet.

.DESCRIPTION
Resolution order for the `.sempal` root:
1) `SEMPAL_CONFIG_HOME` (config base override, if set)
2) OS default config base (`%APPDATA%` on Windows, app-support on macOS, XDG on Linux)
3) `app_data_dir` in `<app_root>/config.toml` (absolute path expected; overrides `.sempal` root)

This is best-effort and intended for quick diagnostics (humans + agents).
#>

param(
  [int]$Lines = 200,
  [switch]$Sandbox
)

function Get-SandboxConfigBase {
  $rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
  return (Join-Path $rootDir ".sandbox\\sempal")
}

function Get-DefaultConfigBase {
  if (-not [string]::IsNullOrWhiteSpace($env:SEMPAL_CONFIG_HOME)) {
    return $env:SEMPAL_CONFIG_HOME
  }
  $sandboxBase = Get-SandboxConfigBase
  if ($Sandbox -or (Test-Path -LiteralPath $sandboxBase -PathType Container)) {
    return $sandboxBase
  }
  if ($IsWindows) {
    if (-not [string]::IsNullOrWhiteSpace($env:APPDATA)) {
      return $env:APPDATA
    }
    return (Join-Path $env:USERPROFILE "AppData\\Roaming")
  }
  if ($IsMacOS) {
    return (Join-Path $HOME "Library/Application Support")
  }
  if (-not [string]::IsNullOrWhiteSpace($env:XDG_CONFIG_HOME)) {
    return $env:XDG_CONFIG_HOME
  }
  return (Join-Path $HOME ".config")
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$sandboxBase = Get-SandboxConfigBase
$configBase = Get-DefaultConfigBase
$usedSandbox = $false
if ([string]::IsNullOrWhiteSpace($env:SEMPAL_CONFIG_HOME) -and ($configBase -eq $sandboxBase)) {
  $usedSandbox = $true
}

function Get-AppDataDirOverrideFromConfig {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ConfigPath
  )

  if (-not (Test-Path -LiteralPath $ConfigPath -PathType Leaf)) {
    return $null
  }

  $line = (Select-String -Path $ConfigPath -Pattern '^\s*app_data_dir\s*=' -SimpleMatch | Select-Object -First 1)
  if ($null -eq $line) {
    return $null
  }

  $text = $line.Line
  $value = $text -replace '^\s*app_data_dir\s*=\s*', ''
  $value = $value -replace '\s+#.*$', ''
  $value = $value.Trim()

  if (($value.StartsWith('"') -and $value.EndsWith('"')) -or ($value.StartsWith("'") -and $value.EndsWith("'"))) {
    $value = $value.Substring(1, $value.Length - 2)
  }

  if ([string]::IsNullOrWhiteSpace($value)) {
    return $null
  }
  return $value
}

function Resolve-AppRoot {
  $defaultRoot = Join-Path $configBase ".sempal"
  $configPath = Join-Path $defaultRoot "config.toml"

  $override = Get-AppDataDirOverrideFromConfig -ConfigPath $configPath
  if (-not [string]::IsNullOrWhiteSpace($override)) {
    if ([System.IO.Path]::IsPathRooted($override)) {
      return $override
    }
    Write-Warning "[latest_log][warn] app_data_dir in $configPath is not an absolute path; ignoring: $override"
  }

  return $defaultRoot
}

$appRoot = Resolve-AppRoot
$logsDir = Join-Path $appRoot "logs"

Write-Host ("[latest_log] config_base_dir={0}" -f $configBase)
Write-Host ("[latest_log] preferred_sandbox_config_home={0}" -f $sandboxBase)
Write-Host ("[latest_log] used_sandbox_config_home={0}" -f $usedSandbox.ToString().ToLowerInvariant())
Write-Host ("[latest_log] app_root={0}" -f $appRoot)
Write-Host ("[latest_log] logs_dir={0}" -f $logsDir)

if (-not (Test-Path -LiteralPath $logsDir -PathType Container)) {
  Write-Error ("[latest_log][error] logs dir does not exist: {0}" -f $logsDir)
  exit 1
}

$newest = Get-ChildItem -LiteralPath $logsDir -File -Filter "*.log" -ErrorAction SilentlyContinue |
  Sort-Object -Property LastWriteTime -Descending |
  Select-Object -First 1

if ($null -eq $newest) {
  Write-Host ("[latest_log] No .log files found under {0}" -f $logsDir)
  exit 0
}

Write-Host ("[latest_log] newest_log={0}" -f $newest.FullName)
Write-Host ("[latest_log] tail_lines={0}" -f $Lines)
Get-Content -LiteralPath $newest.FullName -Tail $Lines
