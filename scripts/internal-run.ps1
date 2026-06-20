param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $AppArgs
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot

if ($null -eq $AppArgs) {
  $AppArgs = @()
} else {
  $AppArgs = @($AppArgs)
}

if ($AppArgs.Count -gt 0 -and $AppArgs[0] -eq "--") {
  $AppArgs = @($AppArgs | Select-Object -Skip 1)
}

$forwardedArgs = @("--log") + $AppArgs

Push-Location $repoRoot
try {
  cargo run -r -- @forwardedArgs
  exit $LASTEXITCODE
}
finally {
  Pop-Location
}
