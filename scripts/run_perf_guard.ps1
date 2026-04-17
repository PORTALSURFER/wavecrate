Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Runs the local runtime performance guard on Windows.

.DESCRIPTION
Executes the GUI-focused `sempal-bench` workspace package with the same
benchmark inputs used by local CI, then validates the generated JSON report
against the repository's warning and fail thresholds.

Warning thresholds are non-blocking and are emitted as log lines. Explicit fail
thresholds remain blocking so the CI wrapper can stop on hard regressions.
#>

function Get-EnvString {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [string]$Default
  )

  $value = [Environment]::GetEnvironmentVariable($Name)
  if ([string]::IsNullOrWhiteSpace($value)) {
    return $Default
  }
  return $value
}

function Get-EnvInt {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [int]$Default
  )

  $raw = Get-EnvString -Name $Name -Default $Default
  $parsed = 0
  if (-not [int]::TryParse($raw, [ref]$parsed)) {
    throw "[perf_guard] ERROR: $Name must be an integer."
  }
  return $parsed
}

function Get-EnvDouble {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [Parameter(Mandatory = $true)]
    [double]$Default
  )

  $raw = Get-EnvString -Name $Name -Default $Default
  $parsed = 0.0
  if (-not [double]::TryParse($raw, [System.Globalization.NumberStyles]::Float, [System.Globalization.CultureInfo]::InvariantCulture, [ref]$parsed)) {
    throw "[perf_guard] ERROR: $Name must be a number."
  }
  return $parsed
}

function Get-OptionalEnvInt {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name
  )

  $raw = [Environment]::GetEnvironmentVariable($Name)
  if ([string]::IsNullOrWhiteSpace($raw)) {
    return $null
  }
  $parsed = 0
  if (-not [int]::TryParse($raw, [ref]$parsed)) {
    throw "[perf_guard] ERROR: $Name must be an integer."
  }
  return $parsed
}

function Get-OptionalEnvDouble {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name
  )

  $raw = [Environment]::GetEnvironmentVariable($Name)
  if ([string]::IsNullOrWhiteSpace($raw)) {
    return $null
  }
  $parsed = 0.0
  if (-not [double]::TryParse($raw, [System.Globalization.NumberStyles]::Float, [System.Globalization.CultureInfo]::InvariantCulture, [ref]$parsed)) {
    throw "[perf_guard] ERROR: $Name must be a number."
  }
  return $parsed
}

function Get-BoolEnvFlag {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,
    [bool]$Default = $false
  )

  $raw = [Environment]::GetEnvironmentVariable($Name)
  if ([string]::IsNullOrWhiteSpace($raw)) {
    return $Default
  }
  return @('1', 'true', 'yes', 'on').Contains($raw.Trim().ToLowerInvariant())
}

function Get-Median {
  param(
    [Parameter(Mandatory = $true)]
    [double[]]$Values
  )

  $ordered = @($Values | Sort-Object)
  if ($ordered.Count -eq 0) {
    throw "[perf_guard] ERROR: median requested with no values."
  }
  $middle = [int][Math]::Floor($ordered.Count / 2)
  if (($ordered.Count % 2) -eq 1) {
    return [double]$ordered[$middle]
  }
  return ([double]$ordered[$middle - 1] + [double]$ordered[$middle]) / 2.0
}

function Get-MedianInt {
  param(
    [Parameter(Mandatory = $true)]
    [double[]]$Values
  )

  return [int][Math]::Round((Get-Median -Values $Values), [MidpointRounding]::AwayFromZero)
}

function Test-HasProperty {
  param(
    [Parameter(Mandatory = $true)]
    [object]$Object,
    [Parameter(Mandatory = $true)]
    [string]$Key
  )

  if ($Object -is [System.Collections.IDictionary]) {
    return $Object.Contains($Key)
  }
  return $null -ne $Object.PSObject.Properties[$Key]
}

function Get-RequiredPropertyValue {
  param(
    [Parameter(Mandatory = $true)]
    [object]$Object,
    [Parameter(Mandatory = $true)]
    [string]$Key
  )

  if (-not (Test-HasProperty -Object $Object -Key $Key)) {
    throw "[perf_guard] ERROR: missing `$Key` in benchmark report."
  }
  if ($Object -is [System.Collections.IDictionary]) {
    return $Object[$Key]
  }
  return $Object.$Key
}

function Get-ScenarioSamples {
  param(
    [Parameter(Mandatory = $true)]
    [object[]]$GuiReports,
    [Parameter(Mandatory = $true)]
    [string]$ScenarioKey
  )

  $samples = @()
  for ($index = 0; $index -lt $GuiReports.Count; $index += 1) {
    $gui = $GuiReports[$index]
    if (-not (Test-HasProperty -Object $gui -Key $ScenarioKey)) {
      Write-Warning "[perf_guard] missing scenario '$ScenarioKey' in run $($index + 1); excluding run from this scenario"
      continue
    }
    $scenario = Get-RequiredPropertyValue -Object $gui -Key $ScenarioKey
    if ($scenario -isnot [System.Collections.IDictionary] -and $scenario -isnot [psobject]) {
      Write-Warning "[perf_guard] malformed scenario '$ScenarioKey' in run $($index + 1); excluding run from this scenario"
      continue
    }
    $samples += ,$scenario
  }
  return $samples
}

function Invoke-PerfBenchRun {
  param(
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,
    [Parameter(Mandatory = $true)]
    [int]$GuiRows,
    [Parameter(Mandatory = $true)]
    [int]$GuiInteractionRows,
    [Parameter(Mandatory = $true)]
    [int]$GuiInteractionIters,
    [Parameter(Mandatory = $true)]
    [int]$WarmupIters,
    [Parameter(Mandatory = $true)]
    [int]$MeasureIters
  )

  cargo run -p sempal-bench-cli --bin sempal-bench -- `
    --out $OutputPath `
    --no-analysis `
    --no-similarity `
    --gui `
    --gui-rows $GuiRows `
    --gui-interaction-rows $GuiInteractionRows `
    --gui-interaction-iters $GuiInteractionIters `
    --warmup-iters $WarmupIters `
    --measure-iters $MeasureIters
  if ($LASTEXITCODE -ne 0) {
    throw "[perf_guard] ERROR: sempal-bench failed with exit code $LASTEXITCODE."
  }
}

function Join-LogFiles {
  param(
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,
    [Parameter(Mandatory = $true)]
    [string[]]$InputPaths
  )

  $content = New-Object System.Collections.Generic.List[string]
  foreach ($path in $InputPaths) {
    if (Test-Path $path) {
      $content.Add((Get-Content $path -Raw))
    }
  }
  Set-Content -Path $OutputPath -Value ($content -join [Environment]::NewLine)
}

function Invoke-StartupProfileRun {
  param(
    [Parameter(Mandatory = $true)]
    [string]$BinaryPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,
    [Parameter(Mandatory = $true)]
    [int]$TimeoutSecs,
    [Parameter(Mandatory = $true)]
    [string]$WorkingDirectory
  )

  $stdoutPath = "$OutputPath.stdout.log"
  $stderrPath = "$OutputPath.stderr.log"
  Remove-Item -LiteralPath $stdoutPath, $stderrPath, $OutputPath -ErrorAction SilentlyContinue

  $previousStartupProfile = $env:SEMPAL_NATIVE_STARTUP_PROFILE
  $env:SEMPAL_NATIVE_STARTUP_PROFILE = "1"
  try {
    $process = Start-Process `
      -FilePath $BinaryPath `
      -WorkingDirectory $WorkingDirectory `
      -RedirectStandardOutput $stdoutPath `
      -RedirectStandardError $stderrPath `
      -PassThru
    $timedOut = -not $process.WaitForExit($TimeoutSecs * 1000)
    if ($timedOut) {
      Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
      Wait-Process -Id $process.Id -Timeout 1 -ErrorAction SilentlyContinue
    }
  } finally {
    if ($null -eq $previousStartupProfile) {
      Remove-Item Env:SEMPAL_NATIVE_STARTUP_PROFILE -ErrorAction SilentlyContinue
    } else {
      $env:SEMPAL_NATIVE_STARTUP_PROFILE = $previousStartupProfile
    }
  }

  Join-LogFiles -OutputPath $OutputPath -InputPaths @($stdoutPath, $stderrPath)
  if (-not $timedOut -and $process.ExitCode -ne 0) {
    Write-Warning "[perf_guard] startup profiling exited with status $($process.ExitCode); see $OutputPath"
  }
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$outPath = Get-EnvString -Name "SEMPAL_PERF_GUARD_OUT" -Default "target/perf/bench.json"
$guiRows = Get-EnvInt -Name "SEMPAL_PERF_GUARD_GUI_ROWS" -Default 2500
$guiInteractionRows = Get-EnvInt -Name "SEMPAL_PERF_GUARD_GUI_INTERACTION_ROWS" -Default 1500
$guiInteractionIters = Get-EnvInt -Name "SEMPAL_PERF_GUARD_GUI_INTERACTION_ITERS" -Default 24
$warmupIters = Get-EnvInt -Name "SEMPAL_PERF_GUARD_WARMUP_ITERS" -Default 3
$measureIters = Get-EnvInt -Name "SEMPAL_PERF_GUARD_MEASURE_ITERS" -Default 16
$runs = Get-EnvInt -Name "SEMPAL_PERF_GUARD_RUNS" -Default 1
$startupProfileEnabled = Get-BoolEnvFlag -Name "SEMPAL_PERF_GUARD_STARTUP_PROFILE"
$startupTimeoutSecs = Get-EnvInt -Name "SEMPAL_PERF_GUARD_STARTUP_TIMEOUT_SECS" -Default 6
$startupRequireValidRuns = Get-BoolEnvFlag -Name "SEMPAL_PERF_GUARD_STARTUP_REQUIRE_VALID_RUNS"
$startupLockEnvOut = [Environment]::GetEnvironmentVariable("SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_OUT")
$startupLockEnvIn = Get-EnvString -Name "SEMPAL_PERF_GUARD_STARTUP_LOCK_ENV_IN" -Default (Join-Path $rootDir "scripts\perf_locks\startup_thresholds.env")
$startupLockMinValidRuns = Get-OptionalEnvInt -Name "SEMPAL_PERF_GUARD_STARTUP_LOCK_MIN_VALID_RUNS"

if ($runs -lt 1) {
  throw "[perf_guard] ERROR: SEMPAL_PERF_GUARD_RUNS must be an integer >= 1."
}

if ($startupProfileEnabled -and (Test-Path $startupLockEnvIn)) {
  Get-Content $startupLockEnvIn | ForEach-Object {
    if ($_ -match '^\s*(?:#|$)') {
      return
    }
    if ($_ -match 'SEMPAL_PERF_[A-Z0-9_]+=') {
      $parts = $_ -split '=', 2
      $name = $parts[0].Trim(' :${}" ')
      $value = $parts[1].Trim(' "}')
      [Environment]::SetEnvironmentVariable($name, $value)
    }
  }
  Write-Host "[perf_guard] loaded startup threshold lock env: $startupLockEnvIn"
}

$reportDir = Split-Path -Parent $outPath
if (-not [string]::IsNullOrWhiteSpace($reportDir)) {
  New-Item -ItemType Directory -Path (Join-Path $rootDir $reportDir) -Force | Out-Null
}

$scenarioConfigs = @(
  @{ Key = "browser_filter_churn_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_FILTER_CHURN"; WarnDefault = 10000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_FILTER_CHURN"; FailDefault = $null },
  @{ Key = "browser_query_churn_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_QUERY_CHURN"; WarnDefault = 12000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_QUERY_CHURN"; FailDefault = $null },
  @{ Key = "browser_sort_toggle_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_SORT_CHURN"; WarnDefault = 10000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_SORT_CHURN"; FailDefault = $null },
  @{ Key = "hover_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_HOVER"; WarnDefault = 8000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_HOVER"; FailDefault = $null },
  @{ Key = "wheel_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_WHEEL"; WarnDefault = 10000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_WHEEL"; FailDefault = 30000 },
  @{ Key = "browser_focus_preview_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_FOCUS_PREVIEW"; WarnDefault = 10000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_FOCUS_PREVIEW"; FailDefault = $null },
  @{ Key = "browser_focus_commit_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_FOCUS_COMMIT"; WarnDefault = 16000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_FOCUS_COMMIT"; FailDefault = 100000 },
  @{ Key = "map_pan_proxy_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_MAP_PAN_PROXY"; WarnDefault = 12000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_MAP_PAN_PROXY"; FailDefault = 4000 },
  @{ Key = "waveform_interaction_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_WAVEFORM"; WarnDefault = 10000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_WAVEFORM"; FailDefault = $null },
  @{ Key = "waveform_pan_zoom_adjacent_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_WAVEFORM_ADJACENT"; WarnDefault = 12000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_WAVEFORM_ADJACENT"; FailDefault = $null },
  @{ Key = "volume_drag_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_VOLUME"; WarnDefault = 8000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_VOLUME"; FailDefault = $null },
  @{ Key = "idle_cursor_motion_latency"; WarnEnv = "SEMPAL_PERF_WARN_P95_US_IDLE_CURSOR"; WarnDefault = 8000; FailEnv = "SEMPAL_PERF_FAIL_P95_US_IDLE_CURSOR"; FailDefault = $null }
)

$warnJankRatio = Get-EnvDouble -Name "SEMPAL_PERF_WARN_FRAME_JANK_RATIO" -Default 0.10
$warnMissedPresentRatio = Get-EnvDouble -Name "SEMPAL_PERF_WARN_MISSED_PRESENT_PROXY_RATIO" -Default 0.05
$failJankRatio = Get-OptionalEnvDouble -Name "SEMPAL_PERF_FAIL_FRAME_JANK_RATIO"
$failMissedPresentRatio = Get-OptionalEnvDouble -Name "SEMPAL_PERF_FAIL_MISSED_PRESENT_PROXY_RATIO"

$reportPaths = New-Object System.Collections.Generic.List[string]
$startupLogPaths = New-Object System.Collections.Generic.List[string]
$canonicalReportPath = Join-Path $rootDir $outPath
$startupSummaryOut = Get-EnvString -Name "SEMPAL_PERF_GUARD_STARTUP_SUMMARY_OUT" -Default ([System.IO.Path]::ChangeExtension($canonicalReportPath, $null) + ".startup_summary.json")

if ($startupProfileEnabled) {
  $startupBinary = Join-Path $rootDir "target\debug\sempal.exe"
  Write-Host "[perf_guard] building sempal startup binary for profile capture"
  cargo build --bin sempal | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "[perf_guard] ERROR: cargo build --bin sempal failed with exit code $LASTEXITCODE."
  }
  if ($runs -ge 3) {
    $startupMinValidRunsDefault = 3
  } else {
    $startupMinValidRunsDefault = 1
  }
  $startupMinValidRuns = Get-EnvInt -Name "SEMPAL_PERF_GUARD_STARTUP_MIN_VALID_RUNS" -Default $startupMinValidRunsDefault
} else {
  $startupMinValidRuns = 1
}

Push-Location $rootDir
try {
  for ($run = 1; $run -le $runs; $run += 1) {
    $runOut = $canonicalReportPath
    if ($runs -gt 1) {
      $runOut = [System.IO.Path]::ChangeExtension($canonicalReportPath, $null) + ".run$run.json"
    }
    $reportPaths.Add($runOut)
    Write-Host "[perf_guard] running sempal-bench interaction profile (run $run/$runs)"
    Invoke-PerfBenchRun `
      -OutputPath $runOut `
      -GuiRows $guiRows `
      -GuiInteractionRows $guiInteractionRows `
      -GuiInteractionIters $guiInteractionIters `
      -WarmupIters $warmupIters `
      -MeasureIters $measureIters
    if ($startupProfileEnabled) {
      $startupLog = [System.IO.Path]::ChangeExtension($canonicalReportPath, $null) + ".startup.run$run.log"
      $startupLogPaths.Add($startupLog)
      Write-Host "[perf_guard] capturing native startup profile (run $run/$runs)"
      Invoke-StartupProfileRun `
        -BinaryPath $startupBinary `
        -OutputPath $startupLog `
        -TimeoutSecs $startupTimeoutSecs `
        -WorkingDirectory $rootDir
    }
  }

  if ($runs -gt 1) {
    Copy-Item -Path $reportPaths[$reportPaths.Count - 1] -Destination $canonicalReportPath -Force
  }

  Write-Host "[perf_guard] parsing benchmark reports ($runs run(s)); canonical report: $canonicalReportPath"

  $guiReports = @()
  foreach ($reportPath in $reportPaths) {
    if (-not (Test-Path $reportPath)) {
      throw "[perf_guard] ERROR: report missing at $reportPath"
    }
    $report = Get-Content $reportPath -Raw | ConvertFrom-Json
    if (-not (Test-HasProperty -Object $report -Key "gui") -or $null -eq (Get-RequiredPropertyValue -Object $report -Key "gui")) {
      throw "[perf_guard] ERROR: missing `gui` benchmark section in $reportPath"
    }
    $guiReports += ,(Get-RequiredPropertyValue -Object $report -Key "gui")
  }

  if ($guiReports.Count -gt 0 -and (Test-HasProperty -Object $guiReports[0] -Key "retained_app_model_projection_p95_us")) {
    $retainedProjectionP95Us = Get-MedianInt -Values ($guiReports | ForEach-Object {
      [double](Get-RequiredPropertyValue -Object $_ -Key "retained_app_model_projection_p95_us")
    })
    Write-Host "[perf_guard] retained_app_model_projection_p95_us: median=$retainedProjectionP95Us us (diagnostic, retained runtime path)"
  }

  $controllerProjectionSamples = @()
  foreach ($gui in $guiReports) {
    if (-not (Test-HasProperty -Object $gui -Key "controller_app_model_projection")) {
      continue
    }
    $summary = Get-RequiredPropertyValue -Object $gui -Key "controller_app_model_projection"
    if ($summary -is [System.Collections.IDictionary] -or $summary -is [psobject]) {
      $controllerProjectionSamples += [double](Get-RequiredPropertyValue -Object $summary -Key "p95_us")
    }
  }
  if ($controllerProjectionSamples.Count -gt 0) {
    $controllerProjectionP95Us = Get-MedianInt -Values $controllerProjectionSamples
    Write-Host "[perf_guard] controller_app_model_projection_p95_us: median=$controllerProjectionP95Us us (diagnostic, legacy controller path)"
  }

  $warned = $false
  $failed = $false

  foreach ($config in $scenarioConfigs) {
    $scenarioKey = $config.Key
    $samples = @(Get-ScenarioSamples -GuiReports $guiReports -ScenarioKey $scenarioKey)
    if ($samples.Count -eq 0) {
      Write-Warning "[perf_guard] skipping scenario '$scenarioKey' because no runs provided it"
      continue
    }

    $p50 = Get-MedianInt -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "p50_us") })
    $p95 = Get-MedianInt -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "p95_us") })
    $p99 = Get-MedianInt -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "p99_us") })
    $maxUs = [int](($samples | ForEach-Object { [int](Get-RequiredPropertyValue -Object $_ -Key "max_us") } | Measure-Object -Maximum).Maximum)
    $meanUs = Get-Median -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "mean_us") })
    $stdDevUs = Get-Median -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "stddev_us") })
    $outlierHighCount = Get-MedianInt -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "outlier_high_count") })
    $outlierHighRatio = Get-Median -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "outlier_high_ratio") })
    $frameBudgetUs = Get-MedianInt -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "frame_budget_us") })
    $frameJankCount = Get-MedianInt -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "frame_jank_count") })
    $frameJankRatio = Get-Median -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "frame_jank_ratio") })
    $missedPresentCount = Get-MedianInt -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "missed_present_proxy_count") })
    $missedPresentRatio = Get-Median -Values ($samples | ForEach-Object { [double](Get-RequiredPropertyValue -Object $_ -Key "missed_present_proxy_ratio") })

    $warnLimit = Get-EnvInt -Name $config.WarnEnv -Default $config.WarnDefault
    $failLimit = Get-OptionalEnvInt -Name $config.FailEnv
    if ($null -eq $failLimit -and $null -ne $config.FailDefault) {
      $failLimit = [int]$config.FailDefault
    }

    $status = "(warn>$warnLimit" + "us"
    if ($null -ne $failLimit) {
      $status += ", fail>$failLimit" + "us"
    }
    $status += ")"

    Write-Host (
      "[perf_guard] {0}: p50={1}us p95={2}us p99={3}us max={4}us mean={5:N1}us stddev={6:N1}us outliers={7} ({8:P1}) runs={9} {10}" -f
      $scenarioKey, $p50, $p95, $p99, $maxUs, $meanUs, $stdDevUs, $outlierHighCount, $outlierHighRatio, $samples.Count, $status
    )
    Write-Host (
      "[perf_guard]   {0} frame_quality_proxy: budget={1}us jank={2} ({3:P1}) missed_present={4} ({5:P1}) (warn_jank>{6:P1} warn_missed>{7:P1})" -f
      $scenarioKey, $frameBudgetUs, $frameJankCount, $frameJankRatio, $missedPresentCount, $missedPresentRatio, $warnJankRatio, $warnMissedPresentRatio
    )

    if ($p95 -gt $warnLimit) {
      $warned = $true
      Write-Warning "[perf_guard] $scenarioKey exceeded warning threshold: p95=${p95}us > ${warnLimit}us"
    }
    if ($null -ne $failLimit -and $p95 -gt $failLimit) {
      $failed = $true
      Write-Error "[perf_guard] $scenarioKey exceeded fail threshold: p95=${p95}us > ${failLimit}us"
    }
    if ($frameJankRatio -gt $warnJankRatio) {
      $warned = $true
      Write-Warning "[perf_guard] $scenarioKey exceeded frame-jank warning threshold: $([string]::Format('{0:P1}', $frameJankRatio)) > $([string]::Format('{0:P1}', $warnJankRatio))"
    }
    if ($null -ne $failJankRatio -and $frameJankRatio -gt $failJankRatio) {
      $failed = $true
      Write-Error "[perf_guard] $scenarioKey exceeded frame-jank fail threshold: $([string]::Format('{0:P1}', $frameJankRatio)) > $([string]::Format('{0:P1}', $failJankRatio))"
    }
    if ($missedPresentRatio -gt $warnMissedPresentRatio) {
      $warned = $true
      Write-Warning "[perf_guard] $scenarioKey exceeded missed-present warning threshold: $([string]::Format('{0:P1}', $missedPresentRatio)) > $([string]::Format('{0:P1}', $warnMissedPresentRatio))"
    }
    if ($null -ne $failMissedPresentRatio -and $missedPresentRatio -gt $failMissedPresentRatio) {
      $failed = $true
      Write-Error "[perf_guard] $scenarioKey exceeded missed-present fail threshold: $([string]::Format('{0:P1}', $missedPresentRatio)) > $([string]::Format('{0:P1}', $failMissedPresentRatio))"
    }
  }

  if ($warned) {
    Write-Host "[perf_guard] completed with warnings"
  } else {
    Write-Host "[perf_guard] completed without warnings"
  }

  if ($failed) {
    throw "[perf_guard] ERROR: fail thresholds exceeded."
  }

  if ($startupProfileEnabled) {
    $startupSummaryArgs = @(
      "scripts/perf_startup_summary.py",
      "--output",
      $startupSummaryOut,
      "--warn-first-present-ms",
      (Get-EnvDouble -Name "SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_MS" -Default 800.0).ToString([System.Globalization.CultureInfo]::InvariantCulture),
      "--min-valid-runs",
      $startupMinValidRuns.ToString()
    )
    $startupFailMs = Get-OptionalEnvDouble -Name "SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_MS"
    if ($null -ne $startupFailMs) {
      $startupSummaryArgs += @(
        "--fail-first-present-ms",
        $startupFailMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
      )
    }
    $startupWarnSpreadMs = Get-OptionalEnvDouble -Name "SEMPAL_PERF_WARN_STARTUP_FIRST_PRESENT_SPREAD_MS"
    if ($null -ne $startupWarnSpreadMs) {
      $startupSummaryArgs += @(
        "--warn-first-present-spread-ms",
        $startupWarnSpreadMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
      )
    }
    $startupFailSpreadMs = Get-OptionalEnvDouble -Name "SEMPAL_PERF_FAIL_STARTUP_FIRST_PRESENT_SPREAD_MS"
    if ($null -ne $startupFailSpreadMs) {
      $startupSummaryArgs += @(
        "--fail-first-present-spread-ms",
        $startupFailSpreadMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
      )
    }
    if ($startupRequireValidRuns) {
      $startupSummaryArgs += "--require-min-valid-runs"
    }
    $startupSummaryArgs += $startupLogPaths
    python @startupSummaryArgs
    if ($LASTEXITCODE -ne 0) {
      throw "[perf_guard] ERROR: startup summary parsing failed with exit code $LASTEXITCODE."
    }

    if (-not [string]::IsNullOrWhiteSpace($startupLockEnvOut)) {
      if ($null -eq $startupLockMinValidRuns) {
        $startupLockMinValidRuns = $startupMinValidRuns
      }
      python scripts/perf_startup_lock_thresholds.py `
        --summary $startupSummaryOut `
        --out $startupLockEnvOut `
        --min-valid-runs $startupLockMinValidRuns
      if ($LASTEXITCODE -ne 0) {
        throw "[perf_guard] ERROR: startup threshold lock generation failed with exit code $LASTEXITCODE."
      }
    }
  }
} finally {
  Pop-Location
}
