function New-CleanDirectory {
    param([string]$Path)
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Ensure-Directory {
    param([string]$Path)
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Get-ResolvedBinaryPath {
    param([string]$BasePath, [string]$RelativeOrAbsolutePath)
    if ([System.IO.Path]::IsPathRooted($RelativeOrAbsolutePath)) {
        return (Resolve-Path $RelativeOrAbsolutePath).Path
    }
    return (Resolve-Path (Join-Path $BasePath $RelativeOrAbsolutePath)).Path
}

function Read-JsonFile {
    param([string]$Path)
    return (Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Write-JsonFile {
    param(
        [string]$Path,
        [Parameter(Mandatory = $true)]$Value
    )
    $json = $Value | ConvertTo-Json -Depth 32
    Set-Content -LiteralPath $Path -Value $json
}

function Wait-ForFile {
    param([string]$Path, [int]$TimeoutMs)
    $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path -LiteralPath $Path) {
            return
        }
        Start-Sleep -Milliseconds 200
    }
    throw "timed out waiting for file $Path"
}

function Wait-ForArtifactChange {
    param(
        [string]$Path,
        [Nullable[datetime]]$PreviousWriteTimeUtc,
        [int]$TimeoutMs = 8000
    )
    if ($null -eq $PreviousWriteTimeUtc) {
        Wait-ForFile -Path $Path -TimeoutMs $TimeoutMs
        return
    }
    $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path -LiteralPath $Path) {
            $currentWriteTimeUtc = (Get-Item -LiteralPath $Path).LastWriteTimeUtc
            if ($currentWriteTimeUtc -gt $PreviousWriteTimeUtc) {
                return
            }
        }
        Start-Sleep -Milliseconds 200
    }
    throw "timed out waiting for GUI artifact update at $Path"
}

function Wait-ForWindow {
    param([string]$Title, [int]$TimeoutMs)
    & aiv window wait --title $Title --timeout-ms $TimeoutMs | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed waiting for window $Title"
    }
}

function Get-WindowBounds {
    param([string]$Title)
    $windowJson = & aiv window list --title $Title
    if ($LASTEXITCODE -ne 0) {
        throw "failed listing window $Title"
    }
    $windowResult = $windowJson | ConvertFrom-Json
    if ($windowResult.count -lt 1) {
        throw "window $Title not found"
    }
    return $windowResult.windows[0].bounds
}

function Test-WindowForeground {
    param([string]$Title)
    & aiv assert window --title $Title --foreground --visible | Out-Null
    return ($LASTEXITCODE -eq 0)
}

function Ensure-WindowForeground {
    param([string]$Title, [int]$Attempts = 4)
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        if (Test-WindowForeground -Title $Title) {
            return $true
        }
        & aiv window focus --title $Title | Out-Null
        Start-Sleep -Milliseconds 250
        if (Test-WindowForeground -Title $Title) {
            return $true
        }
        & aiv workflow click-window --title $Title --anchor center | Out-Null
        Start-Sleep -Milliseconds 350
        if (Test-WindowForeground -Title $Title) {
            return $true
        }
        & aiv keyboard key --alt --key tab | Out-Null
        Start-Sleep -Milliseconds 500
    }
    Write-Warning "best-effort foreground recovery failed for window $Title"
    return $false
}

function Ensure-WindowForegroundOrThrow {
    param(
        [string]$Title,
        [string]$Context = "desktop-aiv",
        [int]$WaitTimeoutMs = 5000,
        [int]$Attempts = 4
    )
    Wait-ForWindow -Title $Title -TimeoutMs $WaitTimeoutMs
    if (Ensure-WindowForeground -Title $Title -Attempts $Attempts) {
        return
    }
    throw "focus recovery failed ($Context): unable to activate window $Title"
}

function Get-DesktopAivFailureCategory {
    param([string]$Message)
    if ($Message -like "focus recovery failed*") {
        return "focus_recovery"
    }
    if ($Message -like "*failed waiting for window*" -or $Message -like "*window*not found*") {
        return "window_lifecycle"
    }
    if ($Message -like "semantic assertion failed*") {
        return "app_assertion"
    }
    return "step_execution"
}

function Find-ArtifactNode {
    param([string]$ArtifactPath, [string]$NodeId)
    $artifact = Read-JsonFile -Path $ArtifactPath
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
    return $null
}

function Resolve-NodeTarget {
    param([string]$ArtifactPath, [string]$NodeId)
    $json = & $script:GuiCliPath resolve-node-target $ArtifactPath $NodeId 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to resolve automation node $NodeId"
    }
    return $json | ConvertFrom-Json
}

function Get-NodeLogicalPoint {
    param(
        [string]$ArtifactPath,
        [string]$NodeId,
        [Nullable[byte]]$XPercent,
        [Nullable[byte]]$YPercent
    )
    $target = Resolve-NodeTarget -ArtifactPath $ArtifactPath -NodeId $NodeId
    if ($null -eq $XPercent -or $null -eq $YPercent) {
        return @{
            x = [int][Math]::Round([double]$target.center_x)
            y = [int][Math]::Round([double]$target.center_y)
        }
    }
    $left = [double]$target.center_x - ([double]$target.width / 2.0)
    $top = [double]$target.center_y - ([double]$target.height / 2.0)
    return @{
        x = [int][Math]::Round($left + ([double]$target.width * ([double]$XPercent / 100.0)))
        y = [int][Math]::Round($top + ([double]$target.height * ([double]$YPercent / 100.0)))
    }
}

function Get-NodeScreenPoint {
    param(
        [string]$ArtifactPath,
        [string]$WindowTitle,
        [string]$NodeId,
        [Nullable[byte]]$XPercent,
        [Nullable[byte]]$YPercent
    )
    $logicalPoint = Get-NodeLogicalPoint -ArtifactPath $ArtifactPath -NodeId $NodeId -XPercent $XPercent -YPercent $YPercent
    $bounds = Get-WindowBounds -Title $WindowTitle
    return @{
        logical_x = $logicalPoint.x
        logical_y = $logicalPoint.y
        screen_x = [int][Math]::Round([double]$bounds.x + [double]$logicalPoint.x)
        screen_y = [int][Math]::Round([double]$bounds.y + [double]$logicalPoint.y)
    }
}

function Capture-Screenshot {
    param([string]$OutputPath)
    Ensure-Directory -Path (Split-Path -Parent $OutputPath)
    & aiv screenshot --output $OutputPath | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to capture screenshot $OutputPath"
    }
}

function Start-CaseProcess {
    param(
        [string]$ResolvedBinary,
        [string]$RepoRoot,
        [string]$SandboxDir,
        [string]$RuntimeArtifactsDir,
        $Case
    )
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $ResolvedBinary
    $startInfo.WorkingDirectory = $RepoRoot
    $startInfo.UseShellExecute = $false
    $startInfo.Environment["SEMPAL_CONFIG_HOME"] = $SandboxDir
    $startInfo.Environment["SEMPAL_GUI_TEST_MODE"] = "1"
    $startInfo.Environment["SEMPAL_GUI_TEST_ARTIFACT_DIR"] = $RuntimeArtifactsDir
    $startInfo.Environment["SEMPAL_GUI_TEST_FIXTURE"] = [string]$Case.fixture_tag
    $startInfo.Environment["SEMPAL_GUI_TEST_VIEWPORT"] = "$($Case.viewport[0])x$($Case.viewport[1])"
    $startInfo.Environment["SEMPAL_GUI_TEST_SCENARIO"] = [string]$Case.name
    return [System.Diagnostics.Process]::Start($startInfo)
}

function Stop-CaseProcess {
    param($Process)
    if ($null -eq $Process) { return }
    if (-not $Process.HasExited) {
        try {
            $null = $Process.CloseMainWindow()
            Start-Sleep -Seconds 2
        } catch {
        }
    }
    if (-not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
    }
}
