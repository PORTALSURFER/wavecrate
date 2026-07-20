<#
.SYNOPSIS
Rejects ambiguous SourceDatabase open aliases.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$callPattern = 'SourceDatabase::(open|open_with_database_root|open_fast|open_fast_with_database_root|open_read_only|open_read_only_with_database_root|open_connection)\s*\('
$declarationPattern = 'pub fn (open|open_with_database_root|open_fast|open_fast_with_database_root|open_read_only|open_read_only_with_database_root|open_connection)\s*\('
$violations = New-Object System.Collections.Generic.List[string]

Push-Location $rootDir
try {
  foreach ($scope in @("benches", "crates", "src", "tests", "tools")) {
    if (-not (Test-Path -LiteralPath $scope)) { continue }
    Get-ChildItem -LiteralPath $scope -Recurse -File -Filter "*.rs" | ForEach-Object {
      $relative = $_.FullName.Replace($rootDir + [IO.Path]::DirectorySeparatorChar, "")
      Select-String -LiteralPath $_.FullName -Pattern $callPattern | ForEach-Object {
        $violations.Add(("{0}:{1}:{2}" -f $relative, $_.LineNumber, $_.Line.Trim()))
      }
    }
  }

  $apiPath = Join-Path $rootDir "crates/wavecrate-library/src/sample_sources/db/mod.rs"
  Select-String -LiteralPath $apiPath -Pattern $declarationPattern | ForEach-Object {
    $relative = $apiPath.Replace($rootDir + [IO.Path]::DirectorySeparatorChar, "")
    $violations.Add(("{0}:{1}:{2}" -f $relative, $_.LineNumber, $_.Line.Trim()))
  }

  if ($violations.Count -gt 0) {
    Write-Host "[source_db_roles] Ambiguous SourceDatabase opens are forbidden:"
    foreach ($violation in ($violations | Sort-Object -Unique)) {
      Write-Host $violation
    }
    Write-Host "[source_db_roles] Use an explicit role-specific API; fixture setup may use open_for_test_fixture_source_write."
    exit 1
  }

  Write-Host "[source_db_roles] OK"
  exit 0
} finally {
  Pop-Location
}
