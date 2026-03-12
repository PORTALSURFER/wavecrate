[CmdletBinding()]
param(
    [string]$CaseFilter,
    [string]$ArtifactsDir = "artifacts/gui-aiv-smoke",
    [string]$BinaryPath = "target/debug/sempal.exe",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$suiteScript = Join-Path $PSScriptRoot "run_gui_aiv_suite.ps1"
$arguments = @(
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    $suiteScript,
    "-PackName",
    "desktop-smoke",
    "-ArtifactsDir",
    $ArtifactsDir,
    "-BinaryPath",
    $BinaryPath
)
if ($CaseFilter) {
    $arguments += @("-CaseFilter", $CaseFilter)
}
if ($SkipBuild) {
    $arguments += "-SkipBuild"
}
powershell @arguments
if ($LASTEXITCODE -ne 0) {
    throw "desktop AIV smoke suite failed"
}
