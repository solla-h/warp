# smoke-test.ps1 — Build and run the BYOP smoke test.
$ErrorActionPreference = "Stop"

Write-Host "[smoke-test] Building warp-oss (release)..."
cargo build --release -p warp
if ($LASTEXITCODE -ne 0) { Write-Host "FAIL: build failed"; exit 1 }

Write-Host "[smoke-test] Running warp-oss --smoke-test..."
& ".\target\release\warp-oss.exe" --smoke-test
$code = $LASTEXITCODE

if ($code -eq 0) {
    Write-Host "[smoke-test] PASS"
} else {
    Write-Host "[smoke-test] FAIL (exit code $code)"
}
exit $code
