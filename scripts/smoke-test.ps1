# smoke-test.ps1 — Build and run the BYOP smoke test.
param(
    [string]$ApiKey = $env:BYOP_API_KEY
)
$ErrorActionPreference = "Stop"

if (-not $ApiKey) {
    Write-Host "[smoke-test] ERROR: BYOP_API_KEY env var or -ApiKey parameter required"
    Write-Host "  Usage: .\scripts\smoke-test.ps1 -ApiKey 'sk-...'"
    Write-Host "  Or:    `$env:BYOP_API_KEY='sk-...'; .\scripts\smoke-test.ps1"
    exit 1
}

Write-Host "[smoke-test] Building warp-oss (release)..."
cargo build --release -p warp --bin warp-oss
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: build failed"; exit 1 }

Write-Host "[smoke-test] Running warp-oss --smoke-test..."
$env:BYOP_API_KEY = $ApiKey
& ".\target\release\warp-oss.exe" --smoke-test
$code = $LASTEXITCODE

if ($code -eq 0) {
    Write-Host "[smoke-test] PASS"
} else {
    Write-Host "[smoke-test] FAIL (exit code $code)"
}
exit $code
