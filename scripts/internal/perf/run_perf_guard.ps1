Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Runs the local runtime performance guard on Windows.

.DESCRIPTION
Executes the GUI-focused `wavecrate-bench` workspace package with the same
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

function Get-ValidationContract {
  $contractPath = Join-Path $rootDir "scripts\internal\data\validation_contract.json"
  if (-not (Test-Path -LiteralPath $contractPath)) {
    throw "[perf_guard] ERROR: shared validation contract is missing at $contractPath."
  }

  try {
    return (Get-Content -Path $contractPath -Raw | ConvertFrom-Json)
  } catch {
    throw "[perf_guard] ERROR: failed to parse shared validation contract: $($_.Exception.Message)"
  }
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

  cargo run -p wavecrate-bench-cli --bin wavecrate-bench -- `
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
    throw "[perf_guard] ERROR: wavecrate-bench failed with exit code $LASTEXITCODE."
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

  $previousStartupProfile = $env:WAVECRATE_NATIVE_STARTUP_PROFILE
  $env:WAVECRATE_NATIVE_STARTUP_PROFILE = "1"
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
      Remove-Item Env:WAVECRATE_NATIVE_STARTUP_PROFILE -ErrorAction SilentlyContinue
    } else {
      $env:WAVECRATE_NATIVE_STARTUP_PROFILE = $previousStartupProfile
    }
  }

  Join-LogFiles -OutputPath $OutputPath -InputPaths @($stdoutPath, $stderrPath)
  if (-not $timedOut -and $process.ExitCode -ne 0) {
    Write-Warning "[perf_guard] startup profiling exited with status $($process.ExitCode); see $OutputPath"
  }
}

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "../../..")).Path
$outPath = Get-EnvString -Name "WAVECRATE_PERF_GUARD_OUT" -Default "target/perf/bench.json"
$guiRows = Get-EnvInt -Name "WAVECRATE_PERF_GUARD_GUI_ROWS" -Default 2500
$guiInteractionRows = Get-EnvInt -Name "WAVECRATE_PERF_GUARD_GUI_INTERACTION_ROWS" -Default 1500
$guiInteractionIters = Get-EnvInt -Name "WAVECRATE_PERF_GUARD_GUI_INTERACTION_ITERS" -Default 24
$warmupIters = Get-EnvInt -Name "WAVECRATE_PERF_GUARD_WARMUP_ITERS" -Default 3
$measureIters = Get-EnvInt -Name "WAVECRATE_PERF_GUARD_MEASURE_ITERS" -Default 16
$runs = Get-EnvInt -Name "WAVECRATE_PERF_GUARD_RUNS" -Default 1
$startupProfileEnabled = Get-BoolEnvFlag -Name "WAVECRATE_PERF_GUARD_STARTUP_PROFILE"
$startupTimeoutSecs = Get-EnvInt -Name "WAVECRATE_PERF_GUARD_STARTUP_TIMEOUT_SECS" -Default 6
$startupRequireValidRuns = Get-BoolEnvFlag -Name "WAVECRATE_PERF_GUARD_STARTUP_REQUIRE_VALID_RUNS"
$startupLockEnvOut = [Environment]::GetEnvironmentVariable("WAVECRATE_PERF_GUARD_STARTUP_LOCK_ENV_OUT")
$startupLockEnvIn = Get-EnvString -Name "WAVECRATE_PERF_GUARD_STARTUP_LOCK_ENV_IN" -Default (Join-Path $rootDir "scripts\perf\locks\startup_thresholds.env")
$startupLockMinValidRuns = Get-OptionalEnvInt -Name "WAVECRATE_PERF_GUARD_STARTUP_LOCK_MIN_VALID_RUNS"

if ($runs -lt 1) {
  throw "[perf_guard] ERROR: WAVECRATE_PERF_GUARD_RUNS must be an integer >= 1."
}

if ($startupProfileEnabled -and (Test-Path $startupLockEnvIn)) {
  Get-Content $startupLockEnvIn | ForEach-Object {
    if ($_ -match '^\s*(?:#|$)') {
      return
    }
    if ($_ -match 'WAVECRATE_PERF_[A-Z0-9_]+=') {
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

$validationContract = Get-ValidationContract

$reportPaths = New-Object System.Collections.Generic.List[string]
$startupLogPaths = New-Object System.Collections.Generic.List[string]
$canonicalReportPath = Join-Path $rootDir $outPath
$startupSummaryOut = Get-EnvString -Name "WAVECRATE_PERF_GUARD_STARTUP_SUMMARY_OUT" -Default ([System.IO.Path]::ChangeExtension($canonicalReportPath, $null) + ".startup_summary.json")

if ($startupProfileEnabled) {
  $startupBinary = Join-Path $rootDir "target\debug\wavecrate.exe"
  Write-Host "[perf_guard] building wavecrate startup binary for profile capture"
  cargo build --bin wavecrate | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "[perf_guard] ERROR: cargo build --bin wavecrate failed with exit code $LASTEXITCODE."
  }
  if ($runs -ge 3) {
    $startupMinValidRunsDefault = 3
  } else {
    $startupMinValidRunsDefault = 1
  }
  $startupMinValidRuns = Get-EnvInt -Name "WAVECRATE_PERF_GUARD_STARTUP_MIN_VALID_RUNS" -Default $startupMinValidRunsDefault
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
    Write-Host "[perf_guard] running wavecrate-bench interaction profile (run $run/$runs)"
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
  python scripts/internal/perf/evaluate_perf_guard_report.py `
    --contract scripts/internal/data/validation_contract.json `
    @reportPaths
  if ($LASTEXITCODE -ne 0) {
    throw "[perf_guard] ERROR: shared perf report evaluation failed with exit code $LASTEXITCODE."
  }

  if ($startupProfileEnabled) {
    $startupSummaryArgs = @(
      "scripts/internal/perf/perf_startup_summary.py",
      "--output",
      $startupSummaryOut,
      "--warn-first-present-ms",
      (Get-EnvDouble -Name "WAVECRATE_PERF_WARN_STARTUP_FIRST_PRESENT_MS" -Default 800.0).ToString([System.Globalization.CultureInfo]::InvariantCulture),
      "--min-valid-runs",
      $startupMinValidRuns.ToString()
    )
    $startupFailMs = Get-OptionalEnvDouble -Name "WAVECRATE_PERF_FAIL_STARTUP_FIRST_PRESENT_MS"
    if ($null -ne $startupFailMs) {
      $startupSummaryArgs += @(
        "--fail-first-present-ms",
        $startupFailMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
      )
    }
    $startupWarnSpreadMs = Get-OptionalEnvDouble -Name "WAVECRATE_PERF_WARN_STARTUP_FIRST_PRESENT_SPREAD_MS"
    if ($null -ne $startupWarnSpreadMs) {
      $startupSummaryArgs += @(
        "--warn-first-present-spread-ms",
        $startupWarnSpreadMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
      )
    }
    $startupFailSpreadMs = Get-OptionalEnvDouble -Name "WAVECRATE_PERF_FAIL_STARTUP_FIRST_PRESENT_SPREAD_MS"
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
      python scripts/internal/perf/perf_startup_lock_thresholds.py `
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
