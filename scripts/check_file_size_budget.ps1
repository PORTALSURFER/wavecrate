
<#
.SYNOPSIS
Enforces a per-file line budget for Rust sources.

.DESCRIPTION
Checks Rust files under `src/`, `tests/`, and `vendor/radiant/src` and fails if
any non-allowlisted file exceeds the line budget.

By default, checks files added/modified in the supplied git diff range (if any),
plus staged/unstaged working tree edits. Known legacy exceptions live in
`docs/file_size_budget_allowlist.txt`.
#>

param(
  [string]$Base,
  [string]$Head = "HEAD",
  [int]$Limit = 400,
  [switch]$All
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"


$rootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $rootDir
try {
  $allowlistPath = Join-Path $rootDir "docs/file_size_budget_allowlist.txt"
  $allowlist = New-Object "System.Collections.Generic.HashSet[string]"
  if (Test-Path -LiteralPath $allowlistPath) {
    foreach ($line in Get-Content -LiteralPath $allowlistPath) {
      if ([string]::IsNullOrWhiteSpace($line)) { continue }
      if ($line.TrimStart().StartsWith("#")) { continue }
      [void]$allowlist.Add($line.Trim())
    }
  }

  $projectScopePaths = @("src", "tests")
  $vendorRepoPath = "vendor/radiant"
  $vendorScopePath = "src"
  $files = New-Object "System.Collections.Generic.HashSet[string]"

  function Test-GitCommit([string]$Ref) {
    if ([string]::IsNullOrWhiteSpace($Ref)) { return $false }
    git rev-parse --verify --quiet "$Ref^{commit}" | Out-Null
    return ($LASTEXITCODE -eq 0)
  }

  function Test-GitCommitInRepo([string]$RepoPath, [string]$Ref) {
    if ([string]::IsNullOrWhiteSpace($Ref)) { return $false }
    git -C $RepoPath rev-parse --verify --quiet "$Ref^{commit}" | Out-Null
    return ($LASTEXITCODE -eq 0)
  }

  function Test-GitRepo([string]$RepoPath) {
    if (-not (Test-Path -LiteralPath $RepoPath -PathType Container)) { return $false }
    git -C $RepoPath rev-parse --is-inside-work-tree | Out-Null
    return ($LASTEXITCODE -eq 0)
  }

  function Add-GitFileList([string[]]$Lines, [string]$Prefix = "") {
    foreach ($line in $Lines) {
      $path = $line.Trim()
      if ([string]::IsNullOrWhiteSpace($path)) { continue }
      $path = $path.Replace("\", "/")
      if (-not [string]::IsNullOrWhiteSpace($Prefix)) {
        $path = ($Prefix.TrimEnd("/", "\") + "/" + $path.TrimStart("/", "\")).Replace("\", "/")
      }
      if (-not $path.EndsWith(".rs")) { continue }
      [void]$files.Add($path)
    }
  }

  function Add-WorkingTreeFiles([string]$RelativePath) {
    if (-not (Test-Path -LiteralPath $RelativePath -PathType Container)) { return }
    $basePath = (Resolve-Path -LiteralPath $RelativePath).Path
    foreach ($entry in Get-ChildItem -LiteralPath $basePath -Recurse -Filter *.rs -File) {
      $relative = [System.IO.Path]::GetRelativePath($rootDir, $entry.FullName).Replace("\", "/")
      [void]$files.Add($relative)
    }
  }

  function Get-RepoTrackedFiles([string]$RepoPath, [string]$ScopePath) {
    if (Test-GitRepo $RepoPath) {
      return @(git -C $RepoPath ls-files -- $ScopePath)
    }
    return @()
  }

  function Get-RepoChangedFiles([string]$RepoPath, [string]$ScopePath, [string]$BaseRef, [string]$HeadRef) {
    if (-not (Test-GitRepo $RepoPath)) { return @() }

    $result = @()
    if (-not [string]::IsNullOrWhiteSpace($BaseRef) -and (Test-GitCommitInRepo $RepoPath $BaseRef) -and (Test-GitCommitInRepo $RepoPath $HeadRef)) {
      $result += @(git -C $RepoPath diff --name-only --diff-filter=AM "$BaseRef...$HeadRef" -- $ScopePath)
    } elseif (Test-GitCommitInRepo $RepoPath $HeadRef) {
      $result += @(git -C $RepoPath show --name-only --pretty=format: $HeadRef -- $ScopePath)
    }

    $result += @(git -C $RepoPath diff --name-only --diff-filter=AM --cached -- $ScopePath)
    $result += @(git -C $RepoPath diff --name-only --diff-filter=AM -- $ScopePath)
    return $result
  }

  function Test-VendorPointerChanged([string]$BaseRef, [string]$HeadRef) {
    if (-not (Test-GitRepo $vendorRepoPath)) { return $false }
    if (-not [string]::IsNullOrWhiteSpace($BaseRef) -and (Test-GitCommit $BaseRef) -and (Test-GitCommit $HeadRef)) {
      return (@(git diff --name-only --diff-filter=AM "$BaseRef...$HeadRef" -- $vendorRepoPath).Count -gt 0)
    }
    if (Test-GitCommit $HeadRef) {
      return (@(git show --name-only --pretty=format: $HeadRef -- $vendorRepoPath).Count -gt 0)
    }
    return $false
  }

  if ($All) {
    Add-GitFileList (git ls-files -- $projectScopePaths)
    if (Test-GitRepo $vendorRepoPath) {
      Add-GitFileList (Get-RepoTrackedFiles -RepoPath $vendorRepoPath -ScopePath $vendorScopePath) -Prefix "$vendorRepoPath/"
    } else {
      Add-WorkingTreeFiles -RelativePath (Join-Path $vendorRepoPath $vendorScopePath)
    }
  } else {
    if ([string]::IsNullOrWhiteSpace($Base)) {
      if (Test-GitCommit "origin/main") { $Base = "origin/main" }
      elseif (Test-GitCommit "main") { $Base = "main" }
    }

    if (-not [string]::IsNullOrWhiteSpace($Base) -and (Test-GitCommit $Base) -and (Test-GitCommit $Head)) {
      Add-GitFileList (git diff --name-only --diff-filter=AM "$Base...$Head" -- $projectScopePaths)
    } elseif (Test-GitCommit $Head) {
      Add-GitFileList (git show --name-only --pretty=format: $Head -- $projectScopePaths)
    }

    Add-GitFileList (git diff --name-only --diff-filter=AM --cached -- $projectScopePaths)
    Add-GitFileList (git diff --name-only --diff-filter=AM -- $projectScopePaths)

    if (Test-VendorPointerChanged -BaseRef $Base -HeadRef $Head) {
      Add-GitFileList (Get-RepoTrackedFiles -RepoPath $vendorRepoPath -ScopePath $vendorScopePath) -Prefix "$vendorRepoPath/"
    } elseif (Test-GitRepo $vendorRepoPath) {
      Add-GitFileList (Get-RepoChangedFiles -RepoPath $vendorRepoPath -ScopePath $vendorScopePath -BaseRef $Base -HeadRef $Head) -Prefix "$vendorRepoPath/"
    }
  }

  if ($files.Count -eq 0) {
    Write-Host "[file_budget] No changed Rust files detected."
    exit 0
  }

  $violations = @()
  $checked = 0
  foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { continue }
    $checked++

    if ($allowlist.Contains($file)) { continue }

    $lineCount = (Get-Content -LiteralPath $file | Measure-Object -Line).Lines
    if ($lineCount -gt $Limit) {
      $violations += ("{0}: {1}" -f $file, $lineCount)
    }
  }

  if ($checked -eq 0) {
    Write-Host "[file_budget] No matching Rust files found to check."
    exit 0
  }

  if ($violations.Count -gt 0) {
    Write-Host ("[file_budget] File size budget violations (limit: {0} lines):" -f $Limit)
    foreach ($v in ($violations | Sort-Object)) {
      Write-Host (" - {0}" -f $v)
    }
    Write-Host ("[file_budget] Fix by splitting files into focused modules, or (temporarily) add to allowlist: {0}" -f $allowlistPath)
    exit 1
  }

  Write-Host ("[file_budget] OK ({0} files checked)" -f $checked)
  exit 0
} finally {
  Pop-Location
}

