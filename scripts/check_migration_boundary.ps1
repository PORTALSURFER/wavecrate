Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Checks the migration boundary between legacy `crate::app` and `src/app_core`.

.DESCRIPTION
Fails if any file under `src/app_core` references `crate::app::` except:
- `src/app_core/app_api.rs`

This mirrors `scripts/check_migration_boundary.sh` for Windows environments that
don’t have `bash`/`rg` available.
#>

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  $appCoreDir = Join-Path $rootDir "src/app_core"
  $allowedFile = Join-Path $appCoreDir "app_api.rs"

  if (-not (Test-Path -LiteralPath $appCoreDir)) {
    throw "Expected app_core directory not found: $appCoreDir"
  }

  $violations = New-Object System.Collections.Generic.List[string]

  $files = Get-ChildItem -LiteralPath $appCoreDir -Recurse -File -Filter "*.rs"
  foreach ($file in $files) {
    $matches = Select-String -LiteralPath $file.FullName -SimpleMatch -Pattern "crate::app::" -ErrorAction SilentlyContinue
    foreach ($m in $matches) {
      if ($m.Path -eq $allowedFile) {
        continue
      }
      $violations.Add(("{0}:{1}:{2}" -f $m.Path, $m.LineNumber, $m.Line.Trim()))
    }
  }

  if ($violations.Count -eq 0) {
    Write-Host "Migration boundary check passed: no legacy app references in app_core."
    exit 0
  }

  Write-Error "Migration boundary check failed: direct crate::app references were found outside app_core::app_api."
  foreach ($v in ($violations | Sort-Object)) {
    Write-Host (" - {0}" -f $v)
  }
  Write-Host ("Allowed app_core migration boundary location: {0}" -f $allowedFile)
  exit 1
} finally {
  Pop-Location
}

