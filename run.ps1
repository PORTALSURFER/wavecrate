param(
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $AppArgs
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$internalRunner = Join-Path $PSScriptRoot "scripts/internal-run.ps1"
if (-not (Test-Path -LiteralPath $internalRunner)) {
  throw "Internal runner not found: $internalRunner"
}

if ($null -eq $AppArgs) {
  $AppArgs = @()
} else {
  $AppArgs = @($AppArgs)
}

& $internalRunner @AppArgs
