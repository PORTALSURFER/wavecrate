[CmdletBinding()]
param(
    [string]$PackName = "desktop-regression",
    [string]$CaseFilter,
    [string]$ArtifactsDir = "artifacts/gui-aiv-suite",
    [string]$BinaryPath = "target/debug/sempal.exe",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "gui_aiv_common.ps1")
. (Join-Path $PSScriptRoot "gui_aiv_execution.ps1")

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$artifactsRoot = Join-Path $repoRoot $ArtifactsDir
$manifestPath = Join-Path $artifactsRoot "suite-manifest.json"
$suiteReportPath = Join-Path $artifactsRoot "suite-report.json"
$suiteSummaryPath = Join-Path $artifactsRoot "suite-summary.md"

New-CleanDirectory -Path $artifactsRoot

if (-not $SkipBuild) {
    Write-Host "[gui-aiv] cargo build --bin sempal"
    cargo build --bin sempal
    if ($LASTEXITCODE -ne 0) { throw "failed to build sempal binary" }

    Write-Host "[gui-aiv] cargo build -p gui-test-cli"
    cargo build -p gui-test-cli
    if ($LASTEXITCODE -ne 0) { throw "failed to build gui-test-cli" }
}

$resolvedBinary = Get-ResolvedBinaryPath -BasePath $repoRoot -RelativeOrAbsolutePath $BinaryPath
$script:GuiCliPath = Join-Path $repoRoot "target/debug/gui-test-cli.exe"

if (Test-Path -LiteralPath $script:GuiCliPath) {
    Write-Host "[gui-aiv] $script:GuiCliPath export-aiv-suite $PackName $manifestPath"
    & $script:GuiCliPath export-aiv-suite $PackName $manifestPath
    if ($LASTEXITCODE -ne 0) { throw "failed to export desktop AIV suite manifest" }
} else {
    Write-Host "[gui-aiv] cargo run -p gui-test-cli -- export-aiv-suite $PackName $manifestPath"
    cargo run -p gui-test-cli -- export-aiv-suite $PackName $manifestPath
    if ($LASTEXITCODE -ne 0) { throw "failed to export desktop AIV suite manifest" }
}

$manifest = Read-JsonFile -Path $manifestPath
$cases = @($manifest.cases)
if ($CaseFilter) {
    $cases = @($cases | Where-Object { $_.name -like "*$CaseFilter*" })
}
if ($cases.Count -eq 0) {
    throw "no desktop AIV cases matched PackName=$PackName CaseFilter=$CaseFilter"
}

$caseResults = @()
foreach ($case in $cases) {
    $caseRoot = Join-Path $artifactsRoot $case.name
    $sandboxDir = Join-Path $caseRoot "sandbox"
    $runtimeArtifactsDir = Join-Path $caseRoot "runtime"
    $screenshotsDir = Join-Path $caseRoot "screenshots"
    $caseManifestPath = Join-Path $caseRoot "case-manifest.json"
    $bundlePath = Join-Path $caseRoot "aiv-bundle.json"
    $caseReportPath = Join-Path $caseRoot "case-report.json"
    $guiArtifactPath = Join-Path $runtimeArtifactsDir "gui_test_latest.json"
    New-CleanDirectory -Path $caseRoot
    Ensure-Directory -Path $sandboxDir
    Ensure-Directory -Path $runtimeArtifactsDir
    Ensure-Directory -Path $screenshotsDir
    Write-JsonFile -Path $caseManifestPath -Value $case
    Write-JsonFile -Path $bundlePath -Value @{
        pack_name = $PackName
        case = $case
        artifact_path = $guiArtifactPath
        screenshots_dir = $screenshotsDir
    }

    $startedAt = Get-Date
    $process = $null
    $currentStepKind = $null
    $result = $null
    try {
        Write-Host "[gui-aiv] start case $($case.name)"
        $process = Start-CaseProcess -ResolvedBinary $resolvedBinary -RepoRoot $repoRoot -SandboxDir $sandboxDir -RuntimeArtifactsDir $runtimeArtifactsDir -Case $case
        if ($null -eq $process) {
            throw "failed to start sempal process for case $($case.name)"
        }
        Wait-ForWindow -Title $case.window_title -TimeoutMs 30000
        $null = Ensure-WindowForeground -Title $case.window_title
        Wait-ForFile -Path $guiArtifactPath -TimeoutMs 30000
        foreach ($step in $case.steps) {
            $currentStepKind = [string]$step.kind
            $previousWriteTimeUtc = if (Test-Path -LiteralPath $guiArtifactPath) {
                (Get-Item -LiteralPath $guiArtifactPath).LastWriteTimeUtc
            } else {
                $null
            }
            Invoke-Step -Step $step -ArtifactPath $guiArtifactPath -WindowTitle $case.window_title -ScreenshotsDir $screenshotsDir
            if ($currentStepKind -in @("click_node", "type_into_node", "press_key", "drag_in_node", "scroll_in_node")) {
                Wait-ForArtifactChange -Path $guiArtifactPath -PreviousWriteTimeUtc $previousWriteTimeUtc
            }
        }
        foreach ($assertion in $case.expected_assertions) {
            $currentStepKind = [string]$assertion.kind
            Wait-ForAssertion -ArtifactPath $guiArtifactPath -Assertion $assertion
        }
        $result = [pscustomobject]@{
            name = [string]$case.name
            status = "passed"
            duration_ms = [int][Math]::Round(((Get-Date) - $startedAt).TotalMilliseconds)
            fixture_tag = [string]$case.fixture_tag
            runtime_artifact = $guiArtifactPath
            aiv_bundle = $bundlePath
            failure_message = $null
            failure_step_kind = $null
            failure_before_screenshot = $null
            failure_after_screenshot = $null
        }
    } catch {
        $failureBefore = $null
        $failureAfter = $null
        try {
            $failureBefore = Join-Path $screenshotsDir "failure-before-close.png"
            Capture-Screenshot -OutputPath $failureBefore
        } catch {
        }
        try {
            $null = Ensure-WindowForeground -Title $case.window_title
            $failureAfter = Join-Path $screenshotsDir "failure-after-focus.png"
            Capture-Screenshot -OutputPath $failureAfter
        } catch {
        }
        $result = [pscustomobject]@{
            name = [string]$case.name
            status = "failed"
            duration_ms = [int][Math]::Round(((Get-Date) - $startedAt).TotalMilliseconds)
            fixture_tag = [string]$case.fixture_tag
            runtime_artifact = $guiArtifactPath
            aiv_bundle = $bundlePath
            failure_message = $_.Exception.Message
            failure_step_kind = $currentStepKind
            failure_before_screenshot = $failureBefore
            failure_after_screenshot = $failureAfter
        }
        Write-Host "[gui-aiv] case failed $($case.name): $($_.Exception.Message)"
    } finally {
        Stop-CaseProcess -Process $process
    }

    Write-JsonFile -Path $caseReportPath -Value $result
    $caseResults += $result
}

$suiteStatus = if ($caseResults | Where-Object { $_.status -eq "failed" }) { "failed" } else { "passed" }
Write-JsonFile -Path $suiteReportPath -Value @{
    pack_name = $PackName
    case_filter = $CaseFilter
    status = $suiteStatus
    manifest_path = $manifestPath
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    cases = $caseResults
}
Write-SuiteSummary -Path $suiteSummaryPath -PackName $PackName -CaseResults $caseResults

if ($suiteStatus -ne "passed") {
    throw "desktop AIV suite failed; see $suiteReportPath"
}
