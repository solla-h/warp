# Worktree Setup Script
# Run this from the main repo (d:\workspace\open_source\warp) to create worktrees for a wave.
# Usage: .\scripts\setup-worktrees.ps1 -Wave 1

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("1", "2", "3", "4")]
    [string]$Wave
)

$basePath = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$worktreeRoot = Join-Path $basePath "worktrees"

if (-not (Test-Path $worktreeRoot)) {
    New-Item -ItemType Directory -Path $worktreeRoot | Out-Null
}

$configs = @{
    "1" = @(
        @{ branch = "wave1/delete-warp-server-client";       dir = "wave1-warp-server-client" }
        @{ branch = "wave1/delete-warp-server-auth";         dir = "wave1-warp-server-auth" }
        @{ branch = "wave1/delete-firebase-auth";            dir = "wave1-firebase-auth" }
        @{ branch = "wave1/delete-cloud-object-persistence"; dir = "wave1-cloud-object-persistence" }
        @{ branch = "wave1/delete-warp-graphql";             dir = "wave1-warp-graphql" }
        @{ branch = "wave1/delete-cloud-object-client";      dir = "wave1-cloud-object-client" }
        @{ branch = "wave1/delete-managed-secrets";          dir = "wave1-managed-secrets" }
    )
    "2" = @(
        @{ branch = "wave2/delete-drive";                dir = "wave2-drive" }
        @{ branch = "wave2/delete-workspaces";           dir = "wave2-workspaces" }
        @{ branch = "wave2/delete-server-graphql";       dir = "wave2-server-graphql" }
        @{ branch = "wave2/delete-server-iap";           dir = "wave2-server-iap" }
        @{ branch = "wave2/delete-server-sync-queue";    dir = "wave2-server-sync-queue" }
        @{ branch = "wave2/delete-server-cloud-objects"; dir = "wave2-server-cloud-objects" }
    )
    "3" = @(
        @{ branch = "wave3/ids-rewrite";              dir = "wave3-ids-rewrite" }
        @{ branch = "wave3/telemetry-stub";           dir = "wave3-telemetry-stub" }
        @{ branch = "wave3/server-api-extraction";    dir = "wave3-server-api-extraction" }
    )
    "4" = @(
        @{ branch = "wave4/delete-server"; dir = "wave4-delete-server" }
    )
}

Write-Host "Creating worktrees for Wave $Wave..." -ForegroundColor Cyan
Write-Host "Base: $(Get-Location)" -ForegroundColor Gray
Write-Host "Worktree root: $worktreeRoot" -ForegroundColor Gray
Write-Host ""

foreach ($cfg in $configs[$Wave]) {
    $wtPath = Join-Path $worktreeRoot $cfg.dir
    if (Test-Path $wtPath) {
        Write-Host "  SKIP $($cfg.dir) (already exists)" -ForegroundColor Yellow
    } else {
        Write-Host "  CREATE $($cfg.dir) -> branch $($cfg.branch)" -ForegroundColor Green
        git worktree add $wtPath -b $cfg.branch
    }
}

Write-Host ""
Write-Host "Done. Open each worktree directory in a separate Cursor window:" -ForegroundColor Cyan
foreach ($cfg in $configs[$Wave]) {
    $wtPath = Join-Path $worktreeRoot $cfg.dir
    Write-Host "  cursor $wtPath"
}

Write-Host ""
Write-Host "After all agents complete, merge from this repo:" -ForegroundColor Cyan
Write-Host "  git merge <branch-name>  # for each branch, then run cargo check"
Write-Host ""
Write-Host "To clean up worktrees after merging:" -ForegroundColor Gray
Write-Host "  git worktree prune"
Write-Host "  Remove-Item -Recurse -Force $worktreeRoot"
