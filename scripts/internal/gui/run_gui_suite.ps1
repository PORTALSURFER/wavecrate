[CmdletBinding()]
param(
    [string]$ArtifactPath = "artifacts/gui-test/gui-test-snapshot.json",
    [string]$ScenarioPackOutputDir = "artifacts/gui-test/scenario-pack"
)

$ErrorActionPreference = "Stop"

powershell -ExecutionPolicy Bypass -File scripts/internal/gui/run_gui_contract.ps1
if ($LASTEXITCODE -ne 0) { throw "gui contract lane failed" }

Write-Host "[gui-suite] cargo test -p wavecrate --lib startup_shot_matches_fixture"
cargo test -p wavecrate --lib startup_shot_matches_fixture
if ($LASTEXITCODE -ne 0) { throw "gui snapshot fixture smoke failed" }

$artifactDir = Split-Path -Parent $ArtifactPath
if ($artifactDir) {
    New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
}

Write-Host "[gui-suite] cargo run -p gui-test-cli -- snapshot $ArtifactPath"
cargo run -p gui-test-cli -- snapshot $ArtifactPath
if ($LASTEXITCODE -ne 0) { throw "gui snapshot export failed" }
$nativeArtifact = Get-Content -LiteralPath $ArtifactPath -Raw | ConvertFrom-Json
if ($nativeArtifact.fixture_runtime -ne "native-app") {
    throw "gui snapshot export did not use the native app runtime"
}
if ($nativeArtifact.runtime_composition.native_source_watchers -ne 1) {
    throw "gui snapshot export did not start exactly one native source watcher"
}
if ($nativeArtifact.runtime_composition.native_readiness_supervisors -ne 1) {
    throw "gui snapshot export did not start exactly one native readiness supervisor"
}
if ($nativeArtifact.runtime_composition.legacy_analysis_pools -ne 0) {
    throw "gui snapshot export started a legacy analysis pool"
}
if ($nativeArtifact.shutdown_artifact.source_processing.joined -ne $true) {
    throw "gui snapshot export did not complete native source-processing shutdown"
}

Write-Host "[gui-suite] cargo run -p gui-test-cli -- run-scenario-pack contract-smoke $ScenarioPackOutputDir"
cargo run -p gui-test-cli -- run-scenario-pack contract-smoke $ScenarioPackOutputDir
if ($LASTEXITCODE -ne 0) { throw "gui scenario-pack export failed" }
