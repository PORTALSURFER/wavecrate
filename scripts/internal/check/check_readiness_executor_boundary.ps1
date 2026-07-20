<#
.SYNOPSIS
Keeps readiness execution outside the legacy controller compile graph.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$failures = New-Object System.Collections.Generic.List[string]

function Require-Literal([string]$Path, [string]$Literal, [string]$Message) {
  $content = Get-Content -LiteralPath $Path -Raw
  if (-not $content.Contains($Literal)) {
    $script:failures.Add($Message)
  }
}

Push-Location $rootDir
try {
  if (Test-Path -LiteralPath "src/internal_analysis_jobs.rs") {
    $failures.Add("src/internal_analysis_jobs.rs is a retired bridge and must not exist.")
  }
  if ((Test-Path -LiteralPath "src/app/controller/library/analysis_jobs/pool/job_execution") -and
      (Get-ChildItem -LiteralPath "src/app/controller/library/analysis_jobs/pool/job_execution" -Recurse -File -Filter "*.rs" | Select-Object -First 1)) {
    $failures.Add("Readiness executors must not live under the legacy controller tree.")
  }

  if (-not (Test-Path -LiteralPath "src/readiness_execution/mod.rs")) {
    $failures.Add("Missing library-owned src/readiness_execution/mod.rs.")
  } else {
    Get-ChildItem -LiteralPath "src/readiness_execution" -Recurse -File -Filter "*.rs" |
      Select-String -Pattern 'crate::(app|app_core)::|internal_analysis_jobs|app::controller::library::analysis_jobs' |
      ForEach-Object {
        $failures.Add("src/readiness_execution must not depend on legacy app/controller modules.")
      }
  }

  Require-Literal "Cargo.toml" 'default = []' `
    "The default Wavecrate feature set must remain free of legacy-controller."
  Require-Literal "src/lib.rs" 'pub mod readiness_execution;' `
    "The readiness executor must remain an explicit library API."
  Require-Literal "tools/gui-test-cli/Cargo.toml" 'features = ["legacy-controller"]' `
    "gui-test-cli must opt into legacy-controller explicitly."
  Require-Literal "tools/bench-cli/Cargo.toml" 'features = ["legacy-controller"]' `
    "wavecrate-bench-cli must opt into legacy-controller explicitly."

  $libSource = Get-Content -LiteralPath "src/lib.rs" -Raw
  foreach ($declaration in @("mod app;", "pub mod app_core;", "pub mod gui_test;")) {
    $escaped = [regex]::Escape($declaration)
    if ($libSource -notmatch ('#\[cfg\(any\(test, feature = "legacy-controller"\)\)\]\r?\n(?:#\[[^\r\n]+\]\r?\n)?' + $escaped)) {
      $failures.Add("$declaration must remain gated by test or legacy-controller.")
    }
  }

  Get-ChildItem -LiteralPath "src/native_app/source_processing" -Recurse -File -Filter "*.rs" |
    Select-String -Pattern 'internal_analysis_jobs|analysis_jobs::(run_feature_stage|run_embedding_stage)' |
    ForEach-Object {
      $failures.Add("Native source processing must call the readiness-owned executor API directly.")
    }

  if ($failures.Count -gt 0) {
    foreach ($failure in ($failures | Sort-Object -Unique)) {
      Write-Host ("[readiness_executor_boundary] {0}" -f $failure)
    }
    exit 1
  }

  Write-Host "[readiness_executor_boundary] OK"
  exit 0
} finally {
  Pop-Location
}
