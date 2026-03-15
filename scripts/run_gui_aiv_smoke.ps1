[CmdletBinding()]
param(
    [string]$CaseFilter,
    [string]$ArtifactsDir = "artifacts/gui-aiv-smoke",
    [string]$BinaryPath = "target/debug/sempal.exe",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$suiteScript = Join-Path $PSScriptRoot "run_gui_aiv_suite.ps1"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$suiteReportPath = Join-Path (Join-Path $repoRoot $ArtifactsDir) "suite-report.json"
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
try {
    powershell @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "desktop AIV smoke suite failed"
    }
} catch {
    $focusFailures = 0
    $windowFailures = 0
    if (Test-Path -LiteralPath $suiteReportPath) {
        try {
            $suiteReport = Get-Content -LiteralPath $suiteReportPath -Raw | ConvertFrom-Json
            $failedCases = @($suiteReport.cases | Where-Object { $_.status -eq "failed" })
            $focusFailures = @($failedCases | Where-Object { $_.failure_category -eq "focus_recovery" }).Count
            $windowFailures = @($failedCases | Where-Object { $_.failure_category -eq "window_lifecycle" }).Count
        } catch {
        }
    }
    if ($focusFailures -gt 0 -or $windowFailures -gt 0) {
        throw "desktop AIV smoke suite failed (focus/window recovery issue; focus=$focusFailures window=$windowFailures); see $suiteReportPath"
    }
    throw
}
