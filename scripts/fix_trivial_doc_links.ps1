$ErrorActionPreference = 'Stop'

function Replace-InFile {
  param(
    [Parameter(Mandatory=$true)][string]$Path,
    [Parameter(Mandatory=$true)][string]$From,
    [Parameter(Mandatory=$true)][string]$To
  )

  $text = Get-Content -LiteralPath $Path -Raw -Encoding UTF8
  $newText = $text.Replace($From, $To)
  if ($newText -ne $text) {
    Set-Content -LiteralPath $Path -Value $newText -Encoding UTF8
    return $true
  }
  return $false
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

$rewrites = @(
  @{ From = "manual/gui_migration_parity.md"; To = "docs/gui_migration_parity.md" },
  @{ From = "manual/native_shell_legacy_baseline.md"; To = "docs/native_shell_legacy_baseline.md" },
  @{ From = "manual/performance_qa.md"; To = "docs/performance_qa.md" },
  @{ From = "manual/feature_vector.md"; To = "docs/feature_vector.md" },
  @{ From = "manual/ann_index_container.md"; To = "docs/ann_index_container.md" },
  @{ From = "manual/updater-contract.md"; To = "docs/updater-contract.md" },
  @{ From = "manual/styleguide.md"; To = "docs/styleguide.md" },
  @{ From = "manual/icon_assets.md"; To = "docs/icon_assets.md" },
  @{ From = "manual/hints.md"; To = "docs/hints.md" },
  @{ From = "manual/plan.md"; To = "docs/plans/plan.md" },
  @{ From = "manual/todo.md"; To = "docs/plans/todo.md" },
  @{ From = "manual/transient_plan.md"; To = "docs/plans/transient_plan.md" },
  @{ From = "manual/transient_audit.md"; To = "docs/audits/transient_audit.md" },
  @{ From = "manual/drag_audit.md"; To = "docs/audits/drag_audit.md" }
)

$targets = @("docs") | ForEach-Object { Join-Path $root $_ }
$changed = $false

foreach ($rw in $rewrites) {
  $from = $rw.From
  $to = $rw.To
  if (-not (Test-Path -LiteralPath (Join-Path $root $to))) {
    continue
  }

  foreach ($t in $targets) {
    if (Test-Path -LiteralPath $t -PathType Leaf) {
      if ((Replace-InFile -Path $t -From $from -To $to)) { $changed = $true }
      continue
    }

    if (Test-Path -LiteralPath $t -PathType Container) {
      Get-ChildItem -LiteralPath $t -Recurse -File -Filter *.md | ForEach-Object {
        if ((Replace-InFile -Path $_.FullName -From $from -To $to)) { $changed = $true }
      }
    }
  }
}

if ($changed) {
  Write-Host "[fix_trivial_doc_links] rewrites applied"
} else {
  Write-Host "[fix_trivial_doc_links] no changes"
}
