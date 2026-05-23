param(
  [string] $PortalSurferRoot = "X:\portalsurfer.org",
  [string] $Server = "188.245.106.212",
  [string] $KeyPath = "$env:USERPROFILE\.ssh\portalsurfer_org_codex",
  [string] $RemotePath = "/opt/portalsurfer",
  [string] $Version = "",
  [string] $BuildId = "",
  [switch] $SkipDeploy,
  [switch] $NoRun,
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $AppArgs
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$portalRootPath = Resolve-Path -LiteralPath $PortalSurferRoot
$portalRoot = $portalRootPath.Path
$signingEnv = Join-Path $portalRoot ".deploy\wavecrate-signing.env"
$stageScript = Join-Path $portalRoot "scripts\stage-wavecrate-release.ps1"
$deployScript = Join-Path $portalRoot "scripts\deploy.ps1"
$counterFile = Join-Path $portalRoot "hosting\wavecrate-build-counter.json"

if (-not (Test-Path -LiteralPath $signingEnv)) {
  throw "Missing Wavecrate signing env file: $signingEnv. Deploy the website once, or copy WAVECRATE_SIGNING_PUBLIC_KEY_B64 into that file."
}
if (-not (Test-Path -LiteralPath $stageScript)) {
  throw "Missing stage script: $stageScript"
}
if (-not (Test-Path -LiteralPath $deployScript)) {
  throw "Missing deploy script: $deployScript"
}

function Get-EnvValue([string] $Path, [string] $Name) {
  $line = Get-Content -LiteralPath $Path | Where-Object { $_ -like "$Name=*" } | Select-Object -First 1
  if (-not $line) {
    throw "Missing $Name in $Path"
  }
  return $line.Substring($Name.Length + 1)
}

function New-Base64UrlToken([int] $ByteCount) {
  $bytes = [byte[]]::new($ByteCount)
  $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
  try {
    $rng.GetBytes($bytes)
  }
  finally {
    $rng.Dispose()
  }
  return [Convert]::ToBase64String($bytes).TrimEnd("=").Replace("+", "-").Replace("/", "_")
}

function ConvertTo-SafeBuildId([string] $Value) {
  $safe = $Value.ToLowerInvariant() -replace "[^a-z0-9._-]+", "-"
  $safe = $safe.Trim("-._")
  if (-not $safe) {
    throw "Build id cannot be empty after sanitization."
  }
  return $safe
}

function Read-BuildCounterJson([string] $Json, [string] $Source) {
  if ([string]::IsNullOrWhiteSpace($Json)) {
    return 1
  }
  $parsed = $Json | ConvertFrom-Json
  $next = [int]$parsed.next_build_number
  if ($next -lt 1) {
    throw "Invalid next_build_number in ${Source}: $next"
  }
  return $next
}

function Read-LocalNextBuildNumber([string] $Path) {
  if (-not (Test-Path -LiteralPath $Path)) {
    return 1
  }
  return Read-BuildCounterJson (Get-Content -LiteralPath $Path -Raw) $Path
}

function Read-RemoteNextBuildNumber() {
  if ($SkipDeploy) {
    return 1
  }
  $remoteCounterPath = "$RemotePath/hosting/wavecrate-build-counter.json"
  $sshArgs = @()
  if ($KeyPath) {
    $sshArgs = @("-i", $KeyPath)
  }
  $raw = ssh @sshArgs "root@$Server" "test -f '$remoteCounterPath' && cat '$remoteCounterPath' || true"
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to read remote Wavecrate build counter from root@${Server}:$remoteCounterPath"
  }
  return Read-BuildCounterJson ($raw -join "`n") "root@${Server}:$remoteCounterPath"
}

function Write-BuildCounter([string] $Path, [int] $NextBuildNumber) {
  $payload = [pscustomobject]@{
    next_build_number = $NextBuildNumber
    updated_at = ([DateTimeOffset]::UtcNow.ToString("o"))
  }
  $json = $payload | ConvertTo-Json -Depth 4
  $encoding = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText($Path, $json, $encoding)
}

if (-not $Version) {
  Push-Location $repoRoot
  try {
    $Version = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages |
      Where-Object { $_.name -eq "wavecrate" } |
      Select-Object -ExpandProperty version -First 1
  }
  finally {
    Pop-Location
  }
}

if (-not $BuildId) {
  $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddHHmmss")
  $shortSha = (git -C $repoRoot rev-parse --short HEAD).Trim()
  $remoteNextBuildNumber = Read-RemoteNextBuildNumber
  $localNextBuildNumber = Read-LocalNextBuildNumber $counterFile
  $BuildNumber = [Math]::Max($remoteNextBuildNumber, $localNextBuildNumber)
  $BuildId = "wavecrate-b$BuildNumber-$stamp-$shortSha"
} else {
  $BuildNumber = 0
}
$BuildId = ConvertTo-SafeBuildId $BuildId
$BuildSignature = New-Base64UrlToken 32
$PublicKey = Get-EnvValue $signingEnv "WAVECRATE_SIGNING_PUBLIC_KEY_B64"
if ($AppArgs.Count -gt 0 -and $AppArgs[0] -eq "--") {
  $AppArgs = @($AppArgs | Select-Object -Skip 1)
}

Write-Host "Wavecrate registered run"
Write-Host "  Build id:        $BuildId"
if ($BuildNumber -gt 0) {
  Write-Host "  Build number:    b$BuildNumber"
}
Write-Host "  Build signature: $BuildSignature"
Write-Host "  Version:         $Version"

Push-Location $repoRoot
try {
  $env:WAVECRATE_BUILD_ID = $BuildId
  $env:WAVECRATE_BUILD_SIGNATURE = $BuildSignature
  $env:WAVECRATE_SIGNING_PUBLIC_KEY_B64 = $PublicKey
  cargo build -r
}
finally {
  Remove-Item Env:\WAVECRATE_BUILD_ID -ErrorAction SilentlyContinue
  Remove-Item Env:\WAVECRATE_BUILD_SIGNATURE -ErrorAction SilentlyContinue
  Remove-Item Env:\WAVECRATE_SIGNING_PUBLIC_KEY_B64 -ErrorAction SilentlyContinue
  Pop-Location
}

$exe = Join-Path $repoRoot "target\release\wavecrate.exe"
if (-not (Test-Path -LiteralPath $exe)) {
  throw "Release binary was not produced: $exe"
}

& powershell -NoProfile -ExecutionPolicy Bypass -File $stageScript `
  -File $exe `
  -BuildId $BuildId `
  -BuildSignature $BuildSignature `
  -BuildNumber $BuildNumber `
  -Version $Version

if (-not $SkipDeploy) {
  if ($BuildNumber -gt 0) {
    Write-BuildCounter $counterFile ($BuildNumber + 1)
  }
  & powershell -NoProfile -ExecutionPolicy Bypass -File $deployScript `
    -Server $Server `
    -KeyPath $KeyPath `
    -RemotePath $RemotePath
}

if (-not $NoRun) {
  Write-Host "Launching $exe $($AppArgs -join ' ')"
  & $exe @AppArgs
}
