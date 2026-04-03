[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

Write-Host "[gui-contract] cargo test app_core::actions::tests"
cargo test app_core::actions::tests
if ($LASTEXITCODE -ne 0) { throw "gui contract catalog tests failed" }

Write-Host "[gui-contract] cargo test gui_test::"
cargo test gui_test::
if ($LASTEXITCODE -ne 0) { throw "gui contract gui_test module tests failed" }

Write-Host "[gui-contract] cargo test --manifest-path vendor/radiant/Cargo.toml toolbar_hit_test_focuses_browser_search"
cargo test --manifest-path vendor/radiant/Cargo.toml toolbar_hit_test_focuses_browser_search
if ($LASTEXITCODE -ne 0) { throw "gui runtime toolbar hit-test smoke failed" }
