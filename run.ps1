param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $AppArgs
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ($null -eq $AppArgs) {
  $AppArgs = @()
} else {
  $AppArgs = @($AppArgs)
}

$publicRunner = Join-Path $PSScriptRoot "scripts/run.ps1"
$internalRunner = Join-Path $PSScriptRoot "scripts/internal-run.ps1"
$publicCommands = @("sandbox", "clean", "logs", "bug-bundle")

if ($AppArgs.Count -gt 0 -and $publicCommands -contains $AppArgs[0]) {
  if (-not (Test-Path -LiteralPath $publicRunner)) {
    throw "Public runner not found: $publicRunner"
  }
  & $publicRunner @AppArgs
  exit $LASTEXITCODE
}

if (-not (Test-Path -LiteralPath $internalRunner)) {
  throw "Internal runner not found: $internalRunner"
}

& $internalRunner @AppArgs
