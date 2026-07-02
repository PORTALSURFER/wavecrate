$ErrorActionPreference = "Stop"

git lfs install --local
git clone --depth 1 --recurse-submodules --shallow-submodules https://github.com/audiosdk/asio "$env:GITHUB_WORKSPACE/asio"
git -C "$env:GITHUB_WORKSPACE/asio" submodule update --init --recursive
git -C "$env:GITHUB_WORKSPACE/asio" lfs pull

$asioRoot = Join-Path $env:GITHUB_WORKSPACE "asio"
$asioHeader = Get-ChildItem -Path $asioRoot -Recurse -Filter "asiodrivers.h" -File | Select-Object -First 1
if (-not $asioHeader) {
  throw "ASIO header not found under $asioRoot"
}

$sdkDir = Split-Path -Parent (Split-Path -Parent $asioHeader.FullName)
if (-not (Test-Path (Join-Path $sdkDir "host")) -or -not (Test-Path (Join-Path $sdkDir "common"))) {
  throw "ASIO SDK directory not found for $($asioHeader.FullName)"
}

"CPAL_ASIO_DIR=$sdkDir" >> $env:GITHUB_ENV
