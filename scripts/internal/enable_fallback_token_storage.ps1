# Enable fallback token storage for wavecrate
# This sets user-level environment variables permanently

$bytes = [byte[]]::new(32)
[System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($bytes)
$secret = ($bytes | ForEach-Object { $_.ToString("x2") }) -join ""

[System.Environment]::SetEnvironmentVariable("WAVECRATE_ALLOW_FALLBACK_TOKEN_STORAGE", "1", "User")
[System.Environment]::SetEnvironmentVariable("WAVECRATE_FALLBACK_KEY", $secret, "User")

Write-Host "✓ Fallback token storage enabled permanently for your user account"
Write-Host "✓ WAVECRATE_ALLOW_FALLBACK_TOKEN_STORAGE = 1"
Write-Host "✓ WAVECRATE_FALLBACK_KEY = (set to random secret)"
Write-Host ""
Write-Host "Note: You may need to restart wavecrate for these changes to take effect."
