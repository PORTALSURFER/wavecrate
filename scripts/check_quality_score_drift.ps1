
<#
.SYNOPSIS
Checks quality score drift for high-visibility guardrails.

.DESCRIPTION
Verifies `docs/QUALITY_SCORE.md` reflects the current health of guardrails.

This is the PowerShell equivalent of `scripts/check_quality_score_drift.sh`.
#>

param(
  [string]$Base = "",
  [string]$Head = "HEAD",
  [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


if ($Help) {
  Write-Host "Usage: scripts/check_quality_score_drift.ps1 [-Base <ref>] [-Head <ref>]"
  exit 0
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$qualityScorePath = Join-Path $rootDir "docs/QUALITY_SCORE.md"
$qualityArea = "Agent-facing guardrails"
$minHealthyScore = 4
$maxDegradedScore = 3
$psRunner = Get-Command pwsh -ErrorAction SilentlyContinue
if ($null -eq $psRunner) {
  $psRunner = Get-Command powershell -ErrorAction SilentlyContinue
}
if ($null -eq $psRunner) {
  throw "[quality_score] Neither pwsh nor powershell is available."
}
$psExe = $psRunner.Path

function Invoke-GuardrailCheck {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Label,
    [Parameter(Mandatory = $true)]
    [string]$ScriptPath,
    [string]$BaseRef = "",
    [string]$HeadRef = "HEAD"
  )

  $args = @("-NoProfile", "-File", $ScriptPath)
  if (-not [string]::IsNullOrWhiteSpace($BaseRef)) {
    $args += @("-Base", $BaseRef)
  }
  if (-not [string]::IsNullOrWhiteSpace($HeadRef)) {
    $args += @("-Head", $HeadRef)
  }

  $stdoutPath = [System.IO.Path]::GetTempFileName()
  $stderrPath = [System.IO.Path]::GetTempFileName()
  try {
    $process = Start-Process `
      -FilePath $psExe `
      -ArgumentList $args `
      -NoNewWindow `
      -Wait `
      -PassThru `
      -RedirectStandardOutput $stdoutPath `
      -RedirectStandardError $stderrPath

    foreach ($line in [System.IO.File]::ReadAllLines($stdoutPath)) {
      Write-Host $line
    }
    foreach ($line in [System.IO.File]::ReadAllLines($stderrPath)) {
      Write-Host $line
    }

    $exitCode = $process.ExitCode
  } finally {
    Remove-Item -LiteralPath $stdoutPath -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $stderrPath -ErrorAction SilentlyContinue
  }

  if ($exitCode -eq 0) {
    Write-Host "[quality_score] PASS: $Label"
    return $true
  }
  Write-Host "[quality_score] FAIL: $Label (exit $exitCode)"
  return $false
}

function Get-QualityScoreForArea {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [Parameter(Mandatory = $true)]
    [string]$Area
  )

  foreach ($line in Get-Content -LiteralPath $Path) {
    if (-not $line.TrimStart().StartsWith("|")) { continue }
    if ($line -match '^\|\s*Area\s*\|') { continue }
    if ($line -match '^\|\s*-+') { continue }

    $parts = $line.Split("|")
    if ($parts.Count -lt 4) { continue }
    $areaPart = $parts[1].Trim()
    $scorePart = $parts[2].Trim()
    if ($areaPart -eq $Area) {
      return $scorePart
    }
  }
  return ""
}

function Assert-ScoreInRange {
  param(
    [int]$Score,
    [string]$State
  )

  if ($Score -lt 0 -or $Score -gt 5) {
    Write-Error "[quality_score] FAIL: Quality score must be in range 0-5."
    return $false
  }

  if ($State -eq "healthy") {
    if ($Score -lt $minHealthyScore) {
      Write-Error "[quality_score] FAIL: Guardrails are passing, but score ($Score) for '$qualityArea' is below $minHealthyScore."
      Write-Host "[quality_score] Update this row in $qualityScorePath to reflect the repaired state."
      return $false
    }
    return $true
  }

  if ($Score -gt $maxDegradedScore) {
    Write-Error "[quality_score] FAIL: Guardrails are degraded, but score ($Score) for '$qualityArea' still appears healthy."
    Write-Host "[quality_score] Lower this row in $qualityScorePath until this is no longer the case."
    return $false
  }
  return $true
}

$fileBudgetOk = Invoke-GuardrailCheck `
  -Label "scripts/check_file_size_budget.ps1" `
  -ScriptPath (Join-Path $rootDir "scripts/check_file_size_budget.ps1") `
  -BaseRef $Base `
  -HeadRef $Head

$tasteOk = Invoke-GuardrailCheck `
  -Label "scripts/check_rust_taste_invariants.ps1" `
  -ScriptPath (Join-Path $rootDir "scripts/check_rust_taste_invariants.ps1") `
  -BaseRef $Base `
  -HeadRef $Head

$guardrailFailed = (-not $fileBudgetOk) -or (-not $tasteOk)

if (-not (Test-Path -LiteralPath $qualityScorePath)) {
  Write-Error "[quality_score] FAIL: Missing file $qualityScorePath."
  exit 1
}

$qualityScoreText = Get-QualityScoreForArea -Path $qualityScorePath -Area $qualityArea
if ([string]::IsNullOrWhiteSpace($qualityScoreText)) {
  Write-Error "[quality_score] FAIL: Missing '$qualityArea' row in $qualityScorePath."
  exit 1
}

$qualityScore = 0
if (-not [int]::TryParse($qualityScoreText, [ref]$qualityScore)) {
  Write-Error "[quality_score] FAIL: Parsed quality score '$qualityScoreText' is not an integer."
  exit 1
}

if ($guardrailFailed) {
  if (-not (Assert-ScoreInRange -Score $qualityScore -State "degraded")) {
    exit 1
  }
  Write-Host "[quality_score] NOTICE: Guardrails are currently failing; quality score is downgraded ($qualityScore)."
  exit 0
}

if (-not (Assert-ScoreInRange -Score $qualityScore -State "healthy")) {
  exit 1
}

Write-Host "[quality_score] OK: guardrails are healthy and score is $qualityScore."
exit 0
