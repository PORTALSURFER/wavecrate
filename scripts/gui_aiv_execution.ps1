function Test-ActionRecorded {
    param([string]$ArtifactPath, [string]$ActionId)
    $artifact = Read-JsonFile -Path $ArtifactPath
    return $artifact.action_trace | Where-Object { $_.action_id -eq $ActionId } | Select-Object -First 1
}

function Invoke-AssertionCheck {
    param([string]$ArtifactPath, $Assertion)
    switch ($Assertion.kind) {
        "assert_node_present" {
            return $null -ne (Find-ArtifactNode -ArtifactPath $ArtifactPath -NodeId $Assertion.node_id)
        }
        "assert_node_absent" {
            return $null -eq (Find-ArtifactNode -ArtifactPath $ArtifactPath -NodeId $Assertion.node_id)
        }
        "assert_node_selected" {
            $node = Find-ArtifactNode -ArtifactPath $ArtifactPath -NodeId $Assertion.node_id
            return $null -ne $node -and [bool]$node.selected -eq [bool]$Assertion.selected
        }
        "assert_node_value_contains" {
            $node = Find-ArtifactNode -ArtifactPath $ArtifactPath -NodeId $Assertion.node_id
            if ($null -eq $node) { return $false }
            $value = if ($null -eq $node.value) { "" } else { [string]$node.value }
            return $value.Contains([string]$Assertion.needle)
        }
        "assert_node_metadata_contains" {
            $node = Find-ArtifactNode -ArtifactPath $ArtifactPath -NodeId $Assertion.node_id
            if ($null -eq $node) { return $false }
            $actual = ""
            if ($null -ne $node.metadata) {
                $property = $node.metadata.PSObject.Properties[$Assertion.key]
                if ($null -ne $property) {
                    $actual = [string]$property.Value
                }
            }
            return $actual.Contains([string]$Assertion.needle)
        }
        "assert_action_recorded" {
            return $null -ne (Test-ActionRecorded -ArtifactPath $ArtifactPath -ActionId $Assertion.action_id)
        }
        default {
            throw "unsupported assertion kind $($Assertion.kind)"
        }
    }
}

function Wait-ForAssertion {
    param(
        [string]$ArtifactPath,
        $Assertion,
        [int]$TimeoutMs = 8000
    )
    $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path -LiteralPath $ArtifactPath) {
            try {
                if (Invoke-AssertionCheck -ArtifactPath $ArtifactPath -Assertion $Assertion) {
                    return
                }
            } catch {
            }
        }
        Start-Sleep -Milliseconds 200
    }
    throw "semantic assertion failed: $($Assertion.kind)"
}

function Invoke-Step {
    param(
        $Step,
        [string]$ArtifactPath,
        [string]$WindowTitle,
        [string]$ScreenshotsDir
    )
    switch ($Step.kind) {
        "wait_for_node" {
            Wait-ForAssertion -ArtifactPath $ArtifactPath -Assertion @{
                kind = "assert_node_present"
                node_id = $Step.node_id
            } -TimeoutMs ([int]$Step.timeout_ms)
        }
        "click_node" {
            $null = Ensure-WindowForeground -Title $WindowTitle
            $point = Get-NodeScreenPoint -ArtifactPath $ArtifactPath -WindowTitle $WindowTitle -NodeId $Step.node_id -XPercent $Step.x_percent -YPercent $Step.y_percent
            & aiv workflow click-window --title $WindowTitle --anchor top-left --offset-x $point.logical_x --offset-y $point.logical_y | Out-Null
            if ($LASTEXITCODE -ne 0) {
                & aiv mouse click --x $point.screen_x --y $point.screen_y | Out-Null
                if ($LASTEXITCODE -ne 0) { throw "failed to click node $($Step.node_id)" }
            }
        }
        "type_into_node" {
            $null = Ensure-WindowForeground -Title $WindowTitle
            $point = Get-NodeScreenPoint -ArtifactPath $ArtifactPath -WindowTitle $WindowTitle -NodeId $Step.node_id -XPercent $null -YPercent $null
            & aiv workflow click-window --title $WindowTitle --anchor top-left --offset-x $point.logical_x --offset-y $point.logical_y | Out-Null
            if ($LASTEXITCODE -ne 0) {
                & aiv mouse click --x $point.screen_x --y $point.screen_y | Out-Null
                if ($LASTEXITCODE -ne 0) { throw "failed to focus node $($Step.node_id) before typing" }
            }
            Start-Sleep -Milliseconds 150
            if ([bool]$Step.clear_existing) {
                & aiv keyboard key --ctrl --key a | Out-Null
                if ($LASTEXITCODE -ne 0) { throw "failed to select existing text in $($Step.node_id)" }
                & aiv keyboard key --key backspace | Out-Null
                if ($LASTEXITCODE -ne 0) { throw "failed to clear existing text in $($Step.node_id)" }
            }
            & aiv keyboard type --text ([string]$Step.text) | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "failed to type into node $($Step.node_id)" }
        }
        "press_key" {
            $null = Ensure-WindowForeground -Title $WindowTitle
            $arguments = @("keyboard", "key", "--key", [string]$Step.key)
            if ([bool]$Step.ctrl) { $arguments += "--ctrl" }
            if ([bool]$Step.alt) { $arguments += "--alt" }
            if ([bool]$Step.shift) { $arguments += "--shift" }
            & aiv @arguments | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "failed to press key $($Step.key)" }
        }
        "drag_in_node" {
            $null = Ensure-WindowForeground -Title $WindowTitle
            $start = Get-NodeScreenPoint -ArtifactPath $ArtifactPath -WindowTitle $WindowTitle -NodeId $Step.node_id -XPercent $Step.start_x_percent -YPercent $Step.start_y_percent
            $end = Get-NodeScreenPoint -ArtifactPath $ArtifactPath -WindowTitle $WindowTitle -NodeId $Step.node_id -XPercent $Step.end_x_percent -YPercent $Step.end_y_percent
            & aiv mouse drag --start-x $start.screen_x --start-y $start.screen_y --end-x $end.screen_x --end-y $end.screen_y --steps 14 --step-delay-ms 4 | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "failed to drag in node $($Step.node_id)" }
        }
        "scroll_in_node" {
            $null = Ensure-WindowForeground -Title $WindowTitle
            $point = Get-NodeScreenPoint -ArtifactPath $ArtifactPath -WindowTitle $WindowTitle -NodeId $Step.node_id -XPercent $Step.x_percent -YPercent $Step.y_percent
            & aiv mouse scroll --delta ([int]$Step.delta) --x $point.screen_x --y $point.screen_y | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "failed to scroll in node $($Step.node_id)" }
        }
        "capture_screenshot" {
            Capture-Screenshot -OutputPath (Join-Path $ScreenshotsDir "$($Step.label).png")
        }
        "assert" {
            Wait-ForAssertion -ArtifactPath $ArtifactPath -Assertion $Step.assertion
        }
        default {
            throw "unsupported step kind $($Step.kind)"
        }
    }
}

function Write-SuiteSummary {
    param(
        [string]$Path,
        [string]$PackName,
        [object[]]$CaseResults
    )
    $lines = @(
        "# GUI AIV Suite Summary",
        "",
        "- Pack: $PackName",
        "- Total cases: $($CaseResults.Count)",
        "- Passed: $(($CaseResults | Where-Object { $_.status -eq 'passed' }).Count)",
        "- Failed: $(($CaseResults | Where-Object { $_.status -eq 'failed' }).Count)",
        ""
    )
    foreach ($result in $CaseResults) {
        $lines += "- [$($result.status)] $($result.name) ($($result.duration_ms) ms)"
        if ($result.failure_message) {
            $lines += "  failure: $($result.failure_message)"
        }
    }
    Set-Content -LiteralPath $Path -Value $lines
}
