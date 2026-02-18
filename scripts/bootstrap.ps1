Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Bootstrap a known-good local dev environment (humans + agents).

.DESCRIPTION
- Installs the pinned Rust toolchain from rust-toolchain.toml
- Ensures rustfmt/clippy are available
- Checks git-lfs and Python
- Prints next-step commands
#>

$verifyOnly = $false
if ($args -contains "--verify-only") {
  $verifyOnly = $true
  $args = @($args | Where-Object { $_ -ne "--verify-only" })
}
if ($args -contains "-h" -or $args -contains "--help") {
  Write-Host "Usage: scripts/bootstrap.ps1 [--verify-only]"
  Write-Host ""
  Write-Host "Default: installs/ensures a known-good local environment (pinned toolchain + rustfmt/clippy)."
  Write-Host "--verify-only: performs checks only (no installs); exits non-zero if missing."
  exit 0
}

$failures = 0

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  Write-Host "[bootstrap] repo: $rootDir"

  if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "[bootstrap] ERROR: git not found on PATH"
  }

  $rg = Get-Command rg -ErrorAction SilentlyContinue
  if ($null -ne $rg) {
    Write-Host "[bootstrap] rg: OK"
  } else {
    Write-Host "[bootstrap] rg: MISSING (recommended; several repo checks use it)"
    Write-Host "[bootstrap]   Install ripgrep (rg). Examples:"
    Write-Host "[bootstrap]     Windows: winget install BurntSushi.ripgrep.MSVC"
    Write-Host "[bootstrap]     Windows (alt): choco install ripgrep"
    if ($verifyOnly) { $failures++ }
  }

  $hasGitLfs = $false
  try {
    git lfs version | Out-Null
    $hasGitLfs = $true
  } catch {
    $hasGitLfs = $false
  }

  if ($hasGitLfs) {
    Write-Host "[bootstrap] git-lfs: OK"
    try { git lfs install --local | Out-Null } catch { }
  } else {
    Write-Host "[bootstrap] git-lfs: MISSING (recommended)"
    Write-Host "[bootstrap]   Install git-lfs and run: git lfs install --local"
  }

  # Prefer python3, then py -3, then python.
  $pythonCmd = $null
  foreach ($candidate in @("python3", "py", "python")) {
    $cmd = Get-Command $candidate -ErrorAction SilentlyContinue
    if ($null -ne $cmd) { $pythonCmd = $candidate; break }
  }

  if ($null -ne $pythonCmd) {
    try {
      if ($pythonCmd -eq "py") {
        $ver = (& py -3 -c "import sys; print('.'.join(map(str, sys.version_info[:3])))" 2>$null)
        Write-Host "[bootstrap] python: OK (py -3 $ver)"
      } else {
        $ver = (& $pythonCmd -c "import sys; print('.'.join(map(str, sys.version_info[:3])))" 2>$null)
        Write-Host "[bootstrap] python: OK ($pythonCmd $ver)"
      }
    } catch {
      Write-Host "[bootstrap] python: present (version unknown)"
    }
  } else {
    Write-Host "[bootstrap] python: MISSING (recommended; used by some tooling/scripts)"
  }

  if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw "[bootstrap] ERROR: rustup not found on PATH. Install from https://rustup.rs/ and re-run."
  }

  $toolchainFile = Join-Path $rootDir "rust-toolchain.toml"
  if (-not (Test-Path -LiteralPath $toolchainFile)) {
    throw "[bootstrap] ERROR: rust-toolchain.toml not found at repo root"
  }

  # Parse pinned channel with a simple regex (works without tomllib).
  $channel = "stable"
  foreach ($line in Get-Content -LiteralPath $toolchainFile) {
    if ($line -match '^\s*channel\s*=\s*"([^"]+)"\s*$') {
      $channel = $Matches[1]
      break
    }
  }

  Write-Host "[bootstrap] rust toolchain (pinned): $channel"

  $toolchainInstalled = $false
  try {
    rustup run $channel rustc -V | Out-Null
    $toolchainInstalled = $true
    Write-Host "[bootstrap] pinned toolchain installed: yes"
  } catch {
    Write-Host "[bootstrap] pinned toolchain installed: no"
    if ($verifyOnly) {
      $failures++
    } else {
      Write-Host "[bootstrap] rustup toolchain install $channel"
      rustup toolchain install $channel --profile minimal
    }
  }

  $installed = @()
  try {
    $installed = rustup component list --toolchain $channel --installed
  } catch {
    $installed = @()
  }

  $hasFmt = $false
  $hasClippy = $false
  foreach ($l in $installed) {
    if ($l -match '^(rustfmt)') { $hasFmt = $true }
    if ($l -match '^(clippy)') { $hasClippy = $true }
  }

  if ($hasFmt) {
    Write-Host "[bootstrap] rustfmt: installed"
  } else {
    Write-Host "[bootstrap] rustfmt: missing"
    if ($verifyOnly) {
      $failures++
    } else {
      Write-Host "[bootstrap] rustup component add rustfmt --toolchain $channel"
      rustup component add rustfmt --toolchain $channel
    }
  }

  if ($hasClippy) {
    Write-Host "[bootstrap] clippy: installed"
  } else {
    Write-Host "[bootstrap] clippy: missing"
    if ($verifyOnly) {
      $failures++
    } else {
      Write-Host "[bootstrap] rustup component add clippy --toolchain $channel"
      rustup component add clippy --toolchain $channel
    }
  }

  Write-Host ""
  Write-Host "[bootstrap] Next steps:"
  Write-Host "  - Environment sanity:   powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1"
  Write-Host "  - CI parity checks:     powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1"
  Write-Host "  - Safe local run:       powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1 --"

  if ($verifyOnly) {
    if ($failures -gt 0) {
      Write-Error ("[bootstrap] Result: FAIL ({0} missing requirements). Hint: run without --verify-only to install." -f $failures)
      exit 1
    }
    Write-Host "[bootstrap] Result: OK"
  }
} finally {
  Pop-Location
}
