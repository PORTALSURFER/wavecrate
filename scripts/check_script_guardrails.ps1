Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Runs PowerShell-side script guardrail checks for agent tooling.

.DESCRIPTION
Validates parseability and basic behavior contracts for preflight-facing
PowerShell scripts, including lightweight fixture checks.
#>

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$scriptsDir = Join-Path $rootDir "scripts"
$failures = 0

$psRunner = Get-Command pwsh -ErrorAction SilentlyContinue
if ($null -eq $psRunner) {
  $psRunner = Get-Command powershell -ErrorAction SilentlyContinue
}
if ($null -eq $psRunner) {
  throw "[guardrails] Neither pwsh nor powershell is available."
}
$psExe = $psRunner.Path

function Add-Failure {
  param([string]$Message)
  Write-Host "[guardrails] FAIL: $Message"
  $script:failures++
}

function Write-Pass {
  param([string]$Label)
  Write-Host "[guardrails] PASS: $Label"
}

function Assert-ScriptParses {
  param([string]$Path)
  $tokens = $null
  $errors = $null
  [System.Management.Automation.Language.Parser]::ParseFile($Path, [ref]$tokens, [ref]$errors) | Out-Null
  if ($errors -and $errors.Count -gt 0) {
    Add-Failure "parse check failed for $Path"
    foreach ($error in $errors) {
      Write-Host ("  - {0}" -f $error.Message)
    }
    return
  }
  Write-Pass "parse check for $Path"
}

function Invoke-ExpectExitCode {
  param(
    [string]$Label,
    [int]$ExpectedCode,
    [string]$WorkDir,
    [string]$ScriptPath,
    [string[]]$Arguments = @(),
    [hashtable]$EnvVars = @{}
  )

  $previous = @{}
  foreach ($key in $EnvVars.Keys) {
    $previous[$key] = [Environment]::GetEnvironmentVariable($key)
    [Environment]::SetEnvironmentVariable($key, [string]$EnvVars[$key])
  }

  try {
    Push-Location $WorkDir
    try {
      $prevEap = $ErrorActionPreference
      $ErrorActionPreference = "Continue"
      try {
        $output = & $psExe -NoProfile -File $ScriptPath @Arguments 2>&1
      } finally {
        $ErrorActionPreference = $prevEap
      }
      $exitCode = $LASTEXITCODE
    } finally {
      Pop-Location
    }
  } finally {
    foreach ($key in $EnvVars.Keys) {
      [Environment]::SetEnvironmentVariable($key, $previous[$key])
    }
  }

  if ($exitCode -eq $ExpectedCode) {
    Write-Pass $Label
    return
  }

  Add-Failure "$Label (expected $ExpectedCode, got $exitCode)"
  if ($null -ne $output) {
    foreach ($line in $output) {
      Write-Host ("  {0}" -f $line.ToString())
    }
  }
}

function New-TempDir {
  $tempPath = Join-Path ([System.IO.Path]::GetTempPath()) ("sempal-guardrails-" + [System.Guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Path $tempPath | Out-Null
  return $tempPath
}

Push-Location $rootDir
try {
  $scriptsToParse = @(
    (Join-Path $scriptsDir "run_agent_request.ps1"),
    (Join-Path $scriptsDir "run_agent_ci_checks.ps1"),
    (Join-Path $scriptsDir "run_agent_preflight.ps1"),
    (Join-Path $scriptsDir "ci_local.ps1"),
    (Join-Path $scriptsDir "refresh_memory_md.ps1")
  )
  foreach ($scriptPath in $scriptsToParse) {
    Assert-ScriptParses -Path $scriptPath
  }

  Invoke-ExpectExitCode -Label "run_agent_request --Help" -ExpectedCode 0 -WorkDir $rootDir -ScriptPath (Join-Path $scriptsDir "run_agent_request.ps1") -Arguments @("-Help")
  Invoke-ExpectExitCode -Label "run_agent_preflight --Help" -ExpectedCode 0 -WorkDir $rootDir -ScriptPath (Join-Path $scriptsDir "run_agent_preflight.ps1") -Arguments @("-Help")

  $fixtureDir = New-TempDir
  try {
    $repoDir = Join-Path $fixtureDir "repo"
    New-Item -ItemType Directory -Path $repoDir | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $repoDir "src") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $repoDir "scripts") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $repoDir "docs") | Out-Null

    Copy-Item (Join-Path $scriptsDir "check_file_size_budget.ps1") (Join-Path $repoDir "scripts/check_file_size_budget.ps1")
    Set-Content -Path (Join-Path $repoDir "src/too_many_lines.rs") -Value @(
      "fn main() {",
      "    println!(`"a`");",
      "    println!(`"b`");",
      "    println!(`"c`");",
      "    println!(`"d`");",
      "}"
    )

    git -C $repoDir init -q
    git -C $repoDir config user.name "sempal-ci"
    git -C $repoDir config user.email "ci@sempal.test"
    git -C $repoDir add src/too_many_lines.rs
    git -C $repoDir commit -qm "seed"

    Invoke-ExpectExitCode -Label "file size budget catches over-limit file" -ExpectedCode 1 -WorkDir $repoDir -ScriptPath (Join-Path $repoDir "scripts/check_file_size_budget.ps1") -Arguments @("-All", "-Limit", "3")
    Invoke-ExpectExitCode -Label "file size budget passes under limit" -ExpectedCode 0 -WorkDir $repoDir -ScriptPath (Join-Path $repoDir "scripts/check_file_size_budget.ps1") -Arguments @("-All", "-Limit", "10")
  } finally {
    Remove-Item -Recurse -Force $fixtureDir -ErrorAction SilentlyContinue
  }

  $memoryFixtureDir = New-TempDir
  try {
    $repoDir = Join-Path $memoryFixtureDir "repo"
    New-Item -ItemType Directory -Path $repoDir | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $repoDir "scripts") | Out-Null
    Copy-Item (Join-Path $scriptsDir "check_memory_log.ps1") (Join-Path $repoDir "scripts/check_memory_log.ps1")
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    Set-Content -Path (Join-Path $repoDir "MEMORY.md") -Value @(
      "# MEMORY",
      "Last Updated: $timestamp",
      "Updated By: Codex"
    )

    Invoke-ExpectExitCode -Label "memory log passes with matching required updater" -ExpectedCode 0 -WorkDir $repoDir -ScriptPath (Join-Path $repoDir "scripts/check_memory_log.ps1") -EnvVars @{
      MEMORY_REQUIRED_UPDATER = "Codex"
    }
    Invoke-ExpectExitCode -Label "memory log fails with mismatched required updater" -ExpectedCode 1 -WorkDir $repoDir -ScriptPath (Join-Path $repoDir "scripts/check_memory_log.ps1") -EnvVars @{
      MEMORY_REQUIRED_UPDATER = "Human"
    }
  } finally {
    Remove-Item -Recurse -Force $memoryFixtureDir -ErrorAction SilentlyContinue
  }
} finally {
  Pop-Location
}

if ($failures -gt 0) {
  Write-Host "[guardrails] FAILED: $failures checks failed."
  exit 1
}

Write-Host "[guardrails] OK"
exit 0
