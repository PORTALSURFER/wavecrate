Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Ensures `docs/README.md` remains a reliable system-of-record landing page.

.DESCRIPTION
Checks:
- Required docs are referenced by path in `docs/README.md`
- Any `docs/*.md` path referenced in `docs/README.md` exists on disk
- The improvement-audit plan entry stays phase-neutral and points readers to
  `tmp/improvement_audit_plan.md` as the canonical audit-lane status source
#>

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  $docsReadme = "docs/README.md"
  if (-not (Test-Path -LiteralPath $docsReadme -PathType Leaf)) {
    throw "[docs_index] Missing $docsReadme"
  }

  $required = @(
    "docs/INDEX.md"
    "docs/FEATURE_CHECKLIST.md"
    "docs/ARCHITECTURE.md"
    "docs/ENV_VARS.md"
    "docs/TEST.md"
    "docs/design_principles.md"
    "docs/plans/index.md"
    "docs/plans/TEMPLATE_execution_plan.md"
    "docs/plans/TEMPLATE_investigation.md"
    "docs/run_contracts.md"
)
  $requiredPointers = @(
    "tmp/improvement_audit_plan.md"
  )
  $canonicalAuditPhrase = "canonical source for the current audit lane status and execution order"

  $text = Get-Content -LiteralPath $docsReadme -Raw

  $missingRefs = @()
  foreach ($path in ($required + $requiredPointers)) {
    if (-not ($text -like "*$path*")) {
      $missingRefs += $path
    }
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
      throw ("[docs_index] Required file missing on disk: {0}" -f $path)
    }
  }
  if ($missingRefs.Count -gt 0) {
    Write-Error "[docs_index] docs/README.md is missing required references:"
    foreach ($m in $missingRefs) {
      Write-Host (" - {0}" -f $m)
    }
    exit 1
  }

  $matches = [regex]::Matches($text, '\bdocs/[A-Za-z0-9._/-]+\.md\b')
  $paths = New-Object "System.Collections.Generic.HashSet[string]"
  foreach ($m in $matches) {
    [void]$paths.Add($m.Value)
  }

  $missing = @()
  foreach ($p in $paths) {
    if (-not (Test-Path -LiteralPath $p -PathType Leaf)) {
      $missing += $p
    }
  }
  if ($missing.Count -gt 0) {
    Write-Error "[docs_index] docs/README.md references missing files:"
    foreach ($m in ($missing | Sort-Object)) {
      Write-Host (" - {0}" -f $m)
    }
    exit 1
  }

  $auditBullet = [regex]::Match($text, '(?ms)- `tmp/improvement_audit_plan\.md`(?<body>.*?)(?:\r?\n- |\r?\n## |\z)')
  if (-not $auditBullet.Success) {
    Write-Error "[docs_index] docs/README.md is missing the improvement-audit plan bullet."
    exit 1
  }

  $normalizedAuditBullet = ($auditBullet.Value -replace '\s+', ' ').Trim()
  if (-not ($normalizedAuditBullet -like "*$canonicalAuditPhrase*")) {
    Write-Error "[docs_index] docs/README.md must describe tmp/improvement_audit_plan.md as the canonical audit-lane status source."
    exit 1
  }

  if ($normalizedAuditBullet -match 'Phase\s+[0-9]+' -or
      $normalizedAuditBullet -match 'item\s+[0-9]+' -or
      $normalizedAuditBullet -match 'waiting\s+for\s+explicit\s+confirmation') {
    Write-Error "[docs_index] docs/README.md should not duplicate mutable phase/item/waiting status for tmp/improvement_audit_plan.md."
    exit 1
  }

  Write-Host ("[docs_index] OK ({0} referenced docs files)" -f $paths.Count)
} finally {
  Pop-Location
}
