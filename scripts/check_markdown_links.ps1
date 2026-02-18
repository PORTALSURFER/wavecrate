Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

<#
.SYNOPSIS
Diff-aware local Markdown link checker.

.DESCRIPTION
Checks only added/modified Markdown files (plus staged/unstaged edits) for
broken local file links so new link rot doesn't get introduced.

Ignored:
- HTTP(S), mailto, tel
- Absolute site links starting with `/`
- Pure anchors starting with `#`
#>

param(
  [string]$Base,
  [string]$Head = "HEAD"
)

$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  function Test-GitCommit([string]$Ref) {
    if ([string]::IsNullOrWhiteSpace($Ref)) { return $false }
    try {
      git rev-parse --verify --quiet "$Ref^{commit}" | Out-Null
      return $true
    } catch {
      return $false
    }
  }

  $files = New-Object "System.Collections.Generic.HashSet[string]"

  function Add-Files([string[]]$Lines) {
    foreach ($line in $Lines) {
      $p = $line.Trim()
      if ([string]::IsNullOrWhiteSpace($p)) { continue }
      if (-not $p.EndsWith(".md")) { continue }
      [void]$files.Add($p)
    }
  }

  if (-not [string]::IsNullOrWhiteSpace($Base) -and (Test-GitCommit $Base) -and (Test-GitCommit $Head)) {
    Add-Files (git diff --name-only --diff-filter=AM "$Base...$Head" -- "*.md")
  } elseif (Test-GitCommit $Head) {
    Add-Files (git show --name-only --pretty=format: $Head -- "*.md")
  }
  Add-Files (git diff --name-only --diff-filter=AM --cached -- "*.md")
  Add-Files (git diff --name-only --diff-filter=AM -- "*.md")

  if ($files.Count -eq 0) {
    Write-Host "[md_links] No changed Markdown files detected."
    exit 0
  }

  $linkRe = New-Object System.Text.RegularExpressions.Regex("!?\\[[^\\]]*\\]\\(([^)]+)\\)")
  $violations = @()

  function Is-Ignored([string]$Dest) {
    if ([string]::IsNullOrWhiteSpace($Dest)) { return $true }
    $d = $Dest.Trim()
    $lower = $d.ToLowerInvariant()
    if ($lower.StartsWith("http://") -or $lower.StartsWith("https://") -or $lower.StartsWith("mailto:") -or $lower.StartsWith("tel:")) { return $true }
    if ($d.StartsWith("#")) { return $true }
    if ($d.StartsWith("/")) { return $true }
    return $false
  }

  function Strip-AnchorAndQuery([string]$Dest) {
    $d = $Dest
    if ($d.Contains("#")) { $d = $d.Split("#")[0] }
    if ($d.Contains("?")) { $d = $d.Split("?")[0] }
    return $d.Trim()
  }

  foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { continue }
    $content = Get-Content -LiteralPath $file -Raw
    $dir = Split-Path -Parent $file
    foreach ($m in $linkRe.Matches($content)) {
      $destRaw = $m.Groups[1].Value.Trim()
      if (Is-Ignored $destRaw) { continue }
      $dest = Strip-AnchorAndQuery $destRaw
      if (Is-Ignored $dest) { continue }
      if ($dest.Contains('${') -or $dest.Contains('{{')) { continue }
      $resolved = Resolve-Path -LiteralPath (Join-Path $dir $dest) -ErrorAction SilentlyContinue
      if ($null -eq $resolved) {
        $violations += ("{0}: ({1})" -f $file, $destRaw)
      }
    }
  }

  if ($violations.Count -gt 0) {
    Write-Error "[md_links] Broken local file links detected:"
    foreach ($v in ($violations | Sort-Object)) {
      Write-Host (" - {0}" -f $v)
    }
    exit 1
  }

  Write-Host "[md_links] OK"
  exit 0
} finally {
  Pop-Location
}

