[CmdletBinding()]
param(
    [string]$OutputPath = "artifacts/gui-test/aiv-suite.json"
)

$ErrorActionPreference = "Stop"

$outputDir = Split-Path -Parent $OutputPath
if ($outputDir) {
    New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
}

Write-Host "[gui-aiv] cargo run -p gui-test-cli -- export-aiv-suite $OutputPath"
cargo run -p gui-test-cli -- export-aiv-suite $OutputPath
if ($LASTEXITCODE -ne 0) { throw "gui AIV suite export failed" }

Write-Host "[gui-aiv] exported suite to $OutputPath"
