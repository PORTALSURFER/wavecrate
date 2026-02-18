Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Creates a small diagnostic bundle to attach to bug reports.

.DESCRIPTION
Bundle contents are intentionally limited:
- newest log files (default: 5)
- `config.toml` (if present)
- tool/runtime versions (`rustc`, `cargo`, `git`)

Note: logs and config may contain local paths. Review before sharing.
#>

param(
  [int]$Logs = 5,
  [string]$OutDir = "dist\\bug_bundles"
)

function Get-DefaultConfigBase {
  if (-not [string]::IsNullOrWhiteSpace($env:SEMPAL_CONFIG_HOME)) {
    return $env:SEMPAL_CONFIG_HOME
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
  $base = Get-DefaultConfigBase
  $defaultRoot = Join-Path $base ".sempal"
  $configPath = Join-Path $defaultRoot "config.toml"

  $override = Get-AppDataDirOverrideFromConfig -ConfigPath $configPath
  if (-not [string]::IsNullOrWhiteSpace($override)) {
    if ([System.IO.Path]::IsPathRooted($override)) {
      return $override
    }
    Write-Warning "[bug_bundle][warn] app_data_dir in $configPath is not an absolute path; ignoring: $override"
  }
  return $defaultRoot
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$appRoot = Resolve-AppRoot
$logsDir = Join-Path $appRoot "logs"
$configPath = Join-Path $appRoot "config.toml"
$timestamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")

$bundleRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("sempal-bug-bundle-" + $timestamp)
$bundleDir = Join-Path $bundleRoot ("sempal-bug-bundle-" + $timestamp)
New-Item -ItemType Directory -Path $bundleDir -Force | Out-Null

New-Item -ItemType Directory -Path (Join-Path $bundleDir "meta") -Force | Out-Null
$infoPath = Join-Path $bundleDir "meta\\info.txt"

function Try-Cmd([string]$Exe, [string[]]$Args) {
  try {
    return (& $Exe @Args 2>$null | Out-String).Trim()
  } catch {
    return "n/a"
  }
}

@(
  "timestamp_utc=$timestamp"
  "repo_root=$rootDir"
  "app_root=$appRoot"
  "logs_dir=$logsDir"
  "config_path=$configPath"
  ""
  ("rustc_version=" + (Try-Cmd "rustc" @("--version")))
  ("cargo_version=" + (Try-Cmd "cargo" @("--version")))
  ("git_version=" + (Try-Cmd "git" @("--version")))
  ("os=" + $env:OS)
  ("ps_version=" + $PSVersionTable.PSVersion)
) | Set-Content -LiteralPath $infoPath -Encoding UTF8

if (Test-Path -LiteralPath $configPath -PathType Leaf) {
  New-Item -ItemType Directory -Path (Join-Path $bundleDir "config") -Force | Out-Null
  Copy-Item -LiteralPath $configPath -Destination (Join-Path $bundleDir "config\\config.toml") -Force
}

if (Test-Path -LiteralPath $logsDir -PathType Container) {
  New-Item -ItemType Directory -Path (Join-Path $bundleDir "logs") -Force | Out-Null
  $logFiles =
    Get-ChildItem -LiteralPath $logsDir -File -Filter "*.log" -ErrorAction SilentlyContinue |
    Sort-Object -Property LastWriteTime -Descending |
    Select-Object -First $Logs
  foreach ($log in $logFiles) {
    Copy-Item -LiteralPath $log.FullName -Destination (Join-Path $bundleDir ("logs\\" + $log.Name)) -Force
  }
}

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$zipPath = Join-Path $OutDir ("sempal-bug-bundle-" + $timestamp + ".zip")

Compress-Archive -Path $bundleDir -DestinationPath $zipPath -Force
Remove-Item -LiteralPath $bundleRoot -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ("[bug_bundle] wrote {0}" -f $zipPath)
Write-Host "[bug_bundle] NOTE: logs/config may contain local paths; review before sharing."

