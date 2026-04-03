[CmdletBinding()]
param(
    [string]$ArtifactPath = "artifacts/gui-test/gui-test-snapshot.json",
    [string]$ScenarioPackOutputDir = "artifacts/gui-test/scenario-pack"
)

$ErrorActionPreference = "Stop"

powershell -ExecutionPolicy Bypass -File scripts/run_gui_contract.ps1
if ($LASTEXITCODE -ne 0) { throw "gui contract lane failed" }

Write-Host "[gui-suite] cargo test --manifest-path vendor/radiant/Cargo.toml startup_shot_matches_fixture"
cargo test --manifest-path vendor/radiant/Cargo.toml startup_shot_matches_fixture
if ($LASTEXITCODE -ne 0) { throw "gui snapshot fixture smoke failed" }

$artifactDir = Split-Path -Parent $ArtifactPath
if ($artifactDir) {
    New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
}

Write-Host "[gui-suite] cargo run -p gui-test-cli -- snapshot $ArtifactPath"
cargo run -p gui-test-cli -- snapshot $ArtifactPath
if ($LASTEXITCODE -ne 0) { throw "gui snapshot export failed" }

Write-Host "[gui-suite] cargo run -p gui-test-cli -- run-scenario-pack contract-smoke $ScenarioPackOutputDir"
cargo run -p gui-test-cli -- run-scenario-pack contract-smoke $ScenarioPackOutputDir
if ($LASTEXITCODE -ne 0) { throw "gui scenario-pack export failed" }
