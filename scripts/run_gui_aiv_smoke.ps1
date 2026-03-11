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

function Find-ArtifactNode {
    param([string]$ArtifactPath, [string]$NodeId)
    $artifact = Read-GuiArtifact -ArtifactPath $ArtifactPath
    $stack = [System.Collections.Generic.Stack[object]]::new()
    $stack.Push($artifact.automation_snapshot.root)
    while ($stack.Count -gt 0) {
        $node = $stack.Pop()
        if ($node.id -eq $NodeId) {
            return $node
        }
        foreach ($child in $node.children) {
            $stack.Push($child)
        }
    }
    throw "node $NodeId not found in $ArtifactPath"
}

function Resolve-NodeTarget {
    param([string]$ArtifactPath, [string]$NodeId)
    $json = & $guiCliPath resolve-node-target $ArtifactPath $NodeId 2>$null
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
        $null = Find-ArtifactNode -ArtifactPath $ArtifactPath -NodeId $NodeId
        return $true
    } catch {
        return $false
    }
}

function Get-NodeValue {
    param([string]$ArtifactPath, [string]$NodeId)
    return (Find-ArtifactNode -ArtifactPath $ArtifactPath -NodeId $NodeId).value
}

function Test-ActionRecorded {
    param([string]$ArtifactPath, [string]$ActionId)
    $artifact = Read-GuiArtifact -ArtifactPath $ArtifactPath
    return $artifact.action_trace | Where-Object { $_.action_id -eq $ActionId } | Select-Object -First 1
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
            try {
                if (& $Predicate) {
                    return
                }
            } catch {
                Start-Sleep -Milliseconds 100
            }
        }
        Start-Sleep -Milliseconds 250
    }
    throw "timed out waiting for $Label"
}

function Click-Node {
    param([string]$ArtifactPath, [string]$NodeId)
    $target = Resolve-NodeTarget -ArtifactPath $ArtifactPath -NodeId $NodeId
    $offsetX = [int][Math]::Round($target.center_x)
    $offsetY = [int][Math]::Round($target.center_y)
    $point = Get-NodeScreenPoint -ArtifactPath $ArtifactPath -NodeId $NodeId
    Write-Host "[gui-aiv] click $NodeId at offset=($offsetX,$offsetY) screen=($($point.x),$($point.y))"
    aiv workflow click-window --title $windowTitle --anchor top-left --offset-x $offsetX --offset-y $offsetY | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to click automation node $NodeId"
    }
}

function Capture-Screenshot {
    param([string]$Filename)
    aiv screenshot --output (Join-Path $artifactsRoot $Filename) | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to capture screenshot $Filename"
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

    Ensure-WindowForeground -Title $windowTitle

    Wait-ForGuiArtifact -Path $guiArtifactPath -TimeoutMs 30000
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "browser search field" -Predicate {
        Test-NodePresent -ArtifactPath $guiArtifactPath -NodeId "browser.search_field"
    }

    Capture-Screenshot -Filename "startup.png"
    Start-Sleep -Milliseconds 1000

    $optionsOpened = $false
    for ($attempt = 1; $attempt -le 3 -and -not $optionsOpened; $attempt++) {
        Click-Node -ArtifactPath $guiArtifactPath -NodeId "shell.top_bar.options_button"
        Start-Sleep -Milliseconds 250
        Capture-Screenshot -Filename ("options-click-attempt-{0}.png" -f $attempt)
        try {
            Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "options panel open" -TimeoutMs 3000 -Predicate {
                Test-NodePresent -ArtifactPath $guiArtifactPath -NodeId "overlay.options_panel"
            }
            $optionsOpened = $true
        } catch {
            if ($attempt -ge 3) {
                throw
            }
            Ensure-WindowForeground -Title $windowTitle
            Start-Sleep -Milliseconds 500
        }
    }
    Capture-Screenshot -Filename "options-open.png"

    Click-Node -ArtifactPath $guiArtifactPath -NodeId "overlay.options_panel.close"
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "options panel close" -Predicate {
        -not (Test-NodePresent -ArtifactPath $guiArtifactPath -NodeId "overlay.options_panel")
    }
    Start-Sleep -Milliseconds 300

    aiv keyboard key --key f | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to focus browser search with keyboard" }
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "browser search focus action" -Predicate {
        Test-ActionRecorded -ArtifactPath $guiArtifactPath -ActionId "focus_browser_search"
    }
    Start-Sleep -Milliseconds 200
    aiv keyboard type --text "snare" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to type browser search text" }
    Wait-ForNodeState -ArtifactPath $guiArtifactPath -Label "browser search value" -Predicate {
        (Get-NodeValue -ArtifactPath $guiArtifactPath -NodeId "browser.search_field") -like "*snare*"
    }
    Capture-Screenshot -Filename "search-snare.png"

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
