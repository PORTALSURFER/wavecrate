<#
.SYNOPSIS
Configures optional local Cargo compile caching for repository scripts.

.DESCRIPTION
If `sccache` is installed and `RUSTC_WRAPPER` is not already set, this helper
points Cargo at `sccache`. Scripts can opt out with
`SEMPAL_DISABLE_SCCACHE=1`.
#>

function Enable-SempalCargoCache {
  if ($env:SEMPAL_DISABLE_SCCACHE -eq "1") {
    Write-Host "[cargo-cache] sccache disabled by SEMPAL_DISABLE_SCCACHE=1"
    return
  }

  if (-not [string]::IsNullOrWhiteSpace($env:RUSTC_WRAPPER)) {
    Write-Host "[cargo-cache] keeping existing RUSTC_WRAPPER=$($env:RUSTC_WRAPPER)"
    return
  }

  $sccache = Get-Command sccache -ErrorAction SilentlyContinue
  if ($null -eq $sccache) {
    return
  }

  $env:RUSTC_WRAPPER = $sccache.Source
  Write-Host "[cargo-cache] using sccache via RUSTC_WRAPPER=$($env:RUSTC_WRAPPER)"
}
