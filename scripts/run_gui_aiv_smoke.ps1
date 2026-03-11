[CmdletBinding()]
param(
    [string]$ArtifactsDir = "artifacts/gui-aiv-smoke",
    [string]$BinaryPath = "target/debug/sempal.exe",
    [int]$WindowX = 120,
    [int]$WindowY = 80,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$artifactsRoot = Join-Path $repoRoot $ArtifactsDir
$runtimeArtifacts = Join-Path $artifactsRoot "runtime"
$sandboxDir = Join-Path $artifactsRoot "sandbox"
$suitePath = Join-Path $artifactsRoot "aiv-suite.json"
$guiArtifactPath = Join-Path $runtimeArtifacts "gui_test_latest.json"
$guiCliPath = Join-Path $repoRoot "target/debug/gui-test-cli.exe"
$windowTitle = "Sempal GUI Test"

function New-CleanDirectory {
    param([string]$Path)
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Wait-ForGuiArtifact {
    param([string]$Path, [int]$TimeoutMs)
    $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path -LiteralPath $Path) {
            return
        }
        Start-Sleep -Milliseconds 250
    }
    throw "timed out waiting for GUI artifact $Path"
}

function Read-GuiArtifact {
    param([string]$ArtifactPath)
    return (Get-Content -LiteralPath $ArtifactPath -Raw | ConvertFrom-Json)
}

function Resolve-NodeTarget {
    param([string]$ArtifactPath, [string]$NodeId)
    $json = & $guiCliPath resolve-node-target $ArtifactPath $NodeId
    if ($LASTEXITCODE -ne 0) {
        throw "failed to resolve automation node $NodeId"
    }
    return $json | ConvertFrom-Json
}

function Get-WindowBounds {
    param([string]$Title)
    $windowJson = aiv window list --title $Title
    if ($LASTEXITCODE -ne 0) {
        throw "failed to list window $Title"
    }
    $windowResult = $windowJson | ConvertFrom-Json
    if ($windowResult.count -lt 1) {
        throw "window $Title not found"
    }
    return $windowResult.windows[0].bounds
}

function Test-WindowForeground {
    param([string]$Title)
    aiv assert window --title $Title --foreground --visible | Out-Null
    return ($LASTEXITCODE -eq 0)
}

function Ensure-WindowForeground {
    param([string]$Title, [int]$Attempts = 4)
    for ($attempt = 0; $attempt -lt $Attempts; $attempt++) {
        if (Test-WindowForeground -Title $Title) {
            return
        }
        aiv window focus --title $Title | Out-Null
        if (Test-WindowForeground -Title $Title) {
            return
        }
        aiv keyboard key --alt --key tab | Out-Null
        Start-Sleep -Milliseconds 500
    }
    throw "failed to foreground window $Title"
}

function Get-NodeScreenPoint {
    param([string]$ArtifactPath, [string]$NodeId)
    $target = Resolve-NodeTarget -ArtifactPath $ArtifactPath -NodeId $NodeId
    $bounds = Get-WindowBounds -Title $windowTitle
    return @{
        x = [int][Math]::Round($bounds.x + (($bounds.width * $target.x_percent) / 100.0))
        y = [int][Math]::Round($bounds.y + (($bounds.height * $target.y_percent) / 100.0))
    }
}

function Test-NodePresent {
    param([string]$ArtifactPath, [string]$NodeId)
    try {
        $null = Resolve-NodeTarget -ArtifactPath $ArtifactPath -NodeId $NodeId
        return $true
    } catch {
        return $false
    }
}

function Get-NodeValue {
    param([string]$ArtifactPath, [string]$NodeId)
    $artifact = Read-GuiArtifact -ArtifactPath $ArtifactPath
    $stack = [System.Collections.Generic.Stack[object]]::new()
    $stack.Push($artifact.automation_snapshot.root)
    while ($stack.Count -gt 0) {
        $node = $stack.Pop()
        if ($node.id.0 -eq $NodeId) {
            return $node.value
        }
        foreach ($child in $node.children) {
            $stack.Push($child)
        }
    }
    throw "node $NodeId not found in $ArtifactPath"
}

function Wait-ForNodeState {
    param(
        [string]$ArtifactPath,
        [scriptblock]$Predicate,
        [string]$Label,
        [int]$TimeoutMs = 15000
    )
    $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path -LiteralPath $ArtifactPath) {
            if (& $Predicate) {
                return
            }
        }
        Start-Sleep -Milliseconds 250
    }
    throw "timed out waiting for $Label"
}

function Click-Node {
    param([string]$ArtifactPath, [string]$NodeId)
    $point = Get-NodeScreenPoint -ArtifactPath $ArtifactPath -NodeId $NodeId
    aiv mouse click --x $point.x --y $point.y | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to click automation node $NodeId"
    }
}

function Type-IntoNode {
    param([string]$ArtifactPath, [string]$NodeId, [string]$Text)
    Click-Node -ArtifactPath $ArtifactPath -NodeId $NodeId
    Start-Sleep -Milliseconds 150
    aiv keyboard clear-field | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to clear automation node $NodeId"
    }
    aiv keyboard type --text $Text | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to type into automation node $NodeId"
    }
}

New-CleanDirectory -Path $artifactsRoot
New-CleanDirectory -Path $runtimeArtifacts
New-CleanDirectory -Path $sandboxDir

if (-not $SkipBuild) {
    Write-Host "[gui-aiv] cargo build --bin sempal"
    cargo build --bin sempal
    if ($LASTEXITCODE -ne 0) { throw "failed to build sempal binary" }

    Write-Host "[gui-aiv] cargo build -p gui-test-cli"
    cargo build -p gui-test-cli
    if ($LASTEXITCODE -ne 0) { throw "failed to build gui-test-cli" }
}

Write-Host "[gui-aiv] cargo run -p gui-test-cli -- export-aiv-suite $suitePath"
cargo run -p gui-test-cli -- export-aiv-suite $suitePath
if ($LASTEXITCODE -ne 0) { throw "failed to export AIV suite metadata" }

$resolvedBinary = (Resolve-Path (Join-Path $repoRoot $BinaryPath)).Path
$startInfo = New-Object System.Diagnostics.ProcessStartInfo
$startInfo.FileName = $resolvedBinary
$startInfo.WorkingDirectory = $repoRoot
$startInfo.UseShellExecute = $false
$startInfo.Environment["SEMPAL_CONFIG_HOME"] = $sandboxDir
$startInfo.Environment["SEMPAL_GUI_TEST_MODE"] = "1"
$startInfo.Environment["SEMPAL_GUI_TEST_ARTIFACT_DIR"] = $runtimeArtifacts
$startInfo.Environment["SEMPAL_GUI_TEST_VIEWPORT"] = "1440x810"
$startInfo.Environment["SEMPAL_GUI_TEST_SCENARIO"] = "aiv_smoke"
$startInfo.Environment["SEMPAL_GUI_TEST_FIXTURE"] = "browser"

$process = [System.Diagnostics.Process]::Start($startInfo)
if ($null -eq $process) {
    throw "failed to start sempal process"
}

try {
    Write-Host "[gui-aiv] aiv window wait --title $windowTitle --timeout-ms 30000"
    aiv window wait --title $windowTitle --timeout-ms 30000 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed waiting for Sempal window" }

    Write-Host "[gui-aiv] aiv window move-resize --title $windowTitle --x $WindowX --y $WindowY --width 1440 --height 810"
    aiv window move-resize --title $windowTitle --x $WindowX --y $WindowY --width 1440 --height 810 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to move/resize Sempal window" }
    Ensure-WindowForeground -Title $windowTitle

    Wait-ForGuiArtifact -Path $guiArtifactPath -TimeoutMs 30000
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "browser search field" -Predicate {
        Test-NodePresent -ArtifactPath $guiArtifactPath -NodeId "browser.search_field"
    }

    aiv screenshot --output (Join-Path $artifactsRoot "startup.png") | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to capture startup screenshot" }

    Click-Node -ArtifactPath $guiArtifactPath -NodeId "shell.top_bar.options_button"
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "options panel open" -Predicate {
        Test-NodePresent -ArtifactPath $guiArtifactPath -NodeId "overlay.options_panel"
    }
    aiv screenshot --output (Join-Path $artifactsRoot "options-open.png") | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to capture options-open screenshot" }

    Click-Node -ArtifactPath $guiArtifactPath -NodeId "overlay.options_panel.close"
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "options panel close" -Predicate {
        -not (Test-NodePresent -ArtifactPath $guiArtifactPath -NodeId "overlay.options_panel")
    }

    Type-IntoNode -ArtifactPath $guiArtifactPath -NodeId "browser.search_field" -Text "snare"
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "browser search value" -Predicate {
        (Get-NodeValue -ArtifactPath $guiArtifactPath -NodeId "browser.search_field") -like "*snare*"
    }
    aiv screenshot --output (Join-Path $artifactsRoot "search-snare.png") | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to capture search screenshot" }

    Click-Node -ArtifactPath $guiArtifactPath -NodeId "browser.tab.map"
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "map canvas" -Predicate {
        Test-NodePresent -ArtifactPath $guiArtifactPath -NodeId "browser.map_canvas"
    }
    aiv screenshot --output (Join-Path $artifactsRoot "map-tab.png") | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to capture map screenshot" }

    Click-Node -ArtifactPath $guiArtifactPath -NodeId "browser.tab.samples"
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "samples table" -Predicate {
        Test-NodePresent -ArtifactPath $guiArtifactPath -NodeId "browser.table"
    }

    Write-Host "[gui-aiv] semantic AIV smoke passed"
} finally {
    if (-not $process.HasExited) {
        try {
            $null = $process.CloseMainWindow()
            Start-Sleep -Seconds 2
        } catch {
        }
    }
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}
