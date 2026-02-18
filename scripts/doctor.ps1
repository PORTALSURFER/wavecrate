Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Environment sanity checks for local development and agent runs.

.DESCRIPTION
Checks the common footguns called out in README:
- `CPAL_ASIO_DIR` (Windows ASIO builds)
- `SEMPAL_NATIVE_FONT_PATH` (native shell font override)
- presence of `git lfs`
Also checks toolchain sanity:
- pinned Rust toolchain vs rust-toolchain.toml
- rustfmt/clippy present for the pinned toolchain
- presence of `rg` (ripgrep)
Also prints the expected `.sempal/logs` locations for each OS.
#>

$failures = 0
$warnings = 0

function Write-Info([string]$Message) { Write-Host "[doctor] $Message" }
function Write-Warn([string]$Message) { Write-Warning "[doctor][warn] $Message"; $script:warnings++ }
function Write-Err([string]$Message) { Write-Error "[doctor][error] $Message"; $script:failures++ }

function Write-BootstrapHint {
  Write-Warn "Run bootstrap to install pinned toolchain + tools:"
  Write-Warn "  - bash scripts/bootstrap.sh"
  Write-Warn "  - powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1"
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Write-Info "Repo: $rootDir"

$os =
  if ($IsWindows) { "windows" }
  elseif ($IsMacOS) { "macos" }
  elseif ($IsLinux) { "linux" }
  else { "unknown" }
Write-Info "OS: $os"

Write-Info "Checking ripgrep (rg)..."
$rg = Get-Command rg -ErrorAction SilentlyContinue
if ($null -ne $rg) {
  Write-Info "rg: present"
} else {
  Write-Warn "rg: missing (recommended; used by several repo checks)"
  Write-BootstrapHint
}

Write-Info "Checking Rust toolchain (pinned)..."
$toolchainToml = Join-Path $rootDir "rust-toolchain.toml"
if (-not (Test-Path -LiteralPath $toolchainToml)) {
  Write-Warn "rust-toolchain.toml not found at repo root (can't verify toolchain pin)"
} else {
  $rustup = Get-Command rustup -ErrorAction SilentlyContinue
  if ($null -eq $rustup) {
    Write-Err "rustup not found on PATH (can't verify/install pinned toolchain)"
    Write-BootstrapHint
  } else {
    $channel = "stable"
    foreach ($line in Get-Content -LiteralPath $toolchainToml) {
      if ($line -match '^\s*channel\s*=\s*"([^"]+)"\s*$') {
        $channel = $Matches[1]
        break
      }
    }
    Write-Info "Pinned toolchain channel: $channel"

    # Verify toolchain exists by trying to run rustc under it.
    try {
      rustup run $channel rustc -V | Out-Null
      Write-Info "Pinned toolchain installed: yes"
    } catch {
      Write-Err "Pinned toolchain is not installed: $channel"
      Write-BootstrapHint
    }

    # Warn if active toolchain doesn't match the pin.
    try {
      $active = (rustup show active-toolchain | Select-Object -First 1)
      $activeTok = ($active -split '\s+')[0]
      if (-not [string]::IsNullOrWhiteSpace($activeTok)) {
        if ($activeTok.StartsWith($channel)) {
          Write-Info "Active toolchain: $activeTok (matches pin)"
        } else {
          Write-Warn "Active toolchain: $activeTok (does not match pin: $channel)"
          Write-Warn "Consider: rustup default $channel"
        }
      } else {
        Write-Warn "Could not determine active toolchain via rustup"
      }
    } catch {
      Write-Warn "Could not determine active toolchain via rustup"
    }

    # Verify components exist for pinned toolchain.
    try {
      $installed = rustup component list --toolchain $channel --installed
      $hasFmt = $false
      $hasClippy = $false
      foreach ($l in $installed) {
        if ($l -match '^(rustfmt)') { $hasFmt = $true }
        if ($l -match '^(clippy)') { $hasClippy = $true }
      }
      if ($hasFmt) {
        Write-Info "rustfmt: installed (toolchain $channel)"
      } else {
        Write-Err "rustfmt is not installed for toolchain $channel"
        Write-BootstrapHint
      }
      if ($hasClippy) {
        Write-Info "clippy: installed (toolchain $channel)"
      } else {
        Write-Err "clippy is not installed for toolchain $channel"
        Write-BootstrapHint
      }
    } catch {
      Write-Warn "Could not query rustup components for toolchain $channel"
      Write-BootstrapHint
    }
  }
}

Write-Info "Checking Git LFS..."
$git = Get-Command git -ErrorAction SilentlyContinue
if ($null -eq $git) {
  Write-Warn "git not found on PATH"
} else {
  try {
    git lfs version | Out-Null
    Write-Info "Git LFS: present"
  } catch {
    Write-Warn "Git LFS: missing (install git-lfs if you see checkout/build issues with large assets)"
  }
}

Write-Info "Checking SEMPAL_NATIVE_FONT_PATH..."
$fontPath = $env:SEMPAL_NATIVE_FONT_PATH
if ([string]::IsNullOrWhiteSpace($fontPath)) {
  Write-Info "SEMPAL_NATIVE_FONT_PATH: not set (OK)"
} elseif (Test-Path -LiteralPath $fontPath -PathType Leaf) {
  Write-Info "SEMPAL_NATIVE_FONT_PATH: OK ($fontPath)"
} else {
  Write-Err "SEMPAL_NATIVE_FONT_PATH is set but not a file: $fontPath"
}

Write-Info "Checking CPAL_ASIO_DIR (Windows ASIO builds)..."
$asioDir = $env:CPAL_ASIO_DIR
if ([string]::IsNullOrWhiteSpace($asioDir)) {
  Write-Info "CPAL_ASIO_DIR: not set (OK unless building Windows ASIO support)"
} elseif (-not (Test-Path -LiteralPath $asioDir -PathType Container)) {
  Write-Err "CPAL_ASIO_DIR is set but not a directory: $asioDir"
} else {
  $hostDir = Join-Path $asioDir "host"
  $commonDir = Join-Path $asioDir "common"
  if ((Test-Path -LiteralPath $hostDir -PathType Container) -and (Test-Path -LiteralPath $commonDir -PathType Container)) {
    Write-Info "CPAL_ASIO_DIR: looks like an ASIO SDK root ($asioDir)"
  } else {
    Write-Err "CPAL_ASIO_DIR exists but doesn't look like ASIO SDK root (expected host/ and common/): $asioDir"
  }
}

Write-Info "Expected log locations:"
Write-Info "  Linux:   `$HOME/.config/.sempal/logs"
Write-Info "  macOS:   `$HOME/Library/Application Support/.sempal/logs"
if ($IsWindows -and -not [string]::IsNullOrWhiteSpace($env:APPDATA)) {
  Write-Info ("  Windows: {0}" -f (Join-Path $env:APPDATA ".sempal\\logs"))
} else {
  Write-Info "  Windows: %APPDATA%\\.sempal\\logs"
}
if ($IsLinux -and (Test-Path -LiteralPath "/proc/version")) {
  try {
    $procVersion = Get-Content -LiteralPath "/proc/version" -Raw
    if ($procVersion -match "(?i)microsoft") {
      Write-Info "  WSL hint: /mnt/c/Users/<you>/AppData/Roaming/.sempal/logs"
    }
  } catch {
    # best-effort WSL detection; ignore errors
  }
}

if ($failures -gt 0) {
  Write-Info "Result: FAIL ($failures errors, $warnings warnings)"
  exit 1
}

Write-Info "Result: OK ($warnings warnings)"
exit 0
