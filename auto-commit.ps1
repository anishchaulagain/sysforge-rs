<#
.SYNOPSIS
    Automatically generates and commits N AI-powered commit messages.

.DESCRIPTION
    Runs `cargo run` to generate a commit message via Groq LLM,
    stages all changes with `git add .`, extracts the commit message,
    and commits. Repeats for the specified number of times.

.PARAMETER Count
    Number of commits to generate (default: 1).

.PARAMETER Delay
    Seconds to wait between each commit cycle to avoid API rate limits (default: 3).

.EXAMPLE
    .\auto-commit.ps1 5
    .\auto-commit.ps1 -Count 10 -Delay 5
#>

param(
    [Parameter(Position = 0)]
    [int]$Count = 1,

    [Parameter()]
    [int]$Delay = 3
)

$ErrorActionPreference = "Continue"

# Validate
if ($Count -lt 1) {
    Write-Host "[ERROR] Count must be at least 1." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "=== Auto-Commit Generator ===" -ForegroundColor Cyan
Write-Host "    Commits to generate: $Count" -ForegroundColor DarkCyan
Write-Host "    Delay between runs:  ${Delay}s" -ForegroundColor DarkCyan
Write-Host "==============================" -ForegroundColor DarkGray
Write-Host ""

$successCount = 0
$failCount = 0

for ($i = 1; $i -le $Count; $i++) {
    Write-Host "[$i/$Count] Running cargo run..." -ForegroundColor Yellow

    # Run cargo run and capture output
    $cargoOutput = & cargo run 2>&1 | Out-String

    if ($LASTEXITCODE -ne 0) {
        Write-Host "[$i/$Count] FAILED: cargo run failed:" -ForegroundColor Red
        Write-Host $cargoOutput -ForegroundColor DarkRed
        $failCount++
        continue
    }

    # Extract commit message from the output
    # The message is printed between "--- Suggested Commit Message ---" and "----------------------------------"
    $commitMsg = $null
    $lines = $cargoOutput -split "`n"
    $capture = $false

    foreach ($line in $lines) {
        if ($line -match "--- Suggested Commit Message ---") {
            $capture = $true
            continue
        }
        if ($line -match "^----------------------------------") {
            $capture = $false
            continue
        }
        if ($capture) {
            $trimmed = $line.Trim()
            if ($trimmed -ne "") {
                $commitMsg = $trimmed
            }
        }
    }

    if (-not $commitMsg) {
        Write-Host "[$i/$Count] FAILED: Could not extract commit message from output." -ForegroundColor Red
        Write-Host $cargoOutput -ForegroundColor DarkGray
        $failCount++
        continue
    }

    Write-Host "[$i/$Count] Message: $commitMsg" -ForegroundColor Magenta

    # Stage all changes
    git add .
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[$i/$Count] FAILED: git add failed." -ForegroundColor Red
        $failCount++
        continue
    }

    # Commit with the extracted message
    git commit -m "$commitMsg"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[$i/$Count] FAILED: git commit failed." -ForegroundColor Red
        $failCount++
        continue
    }

    Write-Host "[$i/$Count] Committed successfully!" -ForegroundColor Green
    $successCount++

    # Delay between iterations (skip on last one)
    if ($i -lt $Count) {
        Write-Host "[$i/$Count] Waiting ${Delay}s before next commit..." -ForegroundColor DarkGray
        Start-Sleep -Seconds $Delay
    }

    Write-Host ""
}

# Summary
Write-Host "==============================" -ForegroundColor DarkGray
Write-Host "Done! $successCount succeeded, $failCount failed." -ForegroundColor Cyan
Write-Host ""
