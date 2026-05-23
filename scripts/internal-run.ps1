param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $AppArgs
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot

if ($AppArgs.Count -gt 0 -and $AppArgs[0] -eq "--") {
  $AppArgs = @($AppArgs | Select-Object -Skip 1)
}

$forwardedArgs = @("--log") + $AppArgs
$hadInternalBuildEnv = Test-Path Env:\WAVECRATE_INTERNAL_BUILD
$previousInternalBuildEnv = [Environment]::GetEnvironmentVariable("WAVECRATE_INTERNAL_BUILD", "Process")

Push-Location $repoRoot
try {
  $env:WAVECRATE_INTERNAL_BUILD = "1"
  cargo run -r -- @forwardedArgs
  exit $LASTEXITCODE
}
finally {
  if ($hadInternalBuildEnv) {
    $env:WAVECRATE_INTERNAL_BUILD = $previousInternalBuildEnv
  } else {
    Remove-Item Env:\WAVECRATE_INTERNAL_BUILD -ErrorAction SilentlyContinue
  }
  Pop-Location
}
