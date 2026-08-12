$ErrorActionPreference = "Stop"
$resultsPath = Join-Path $PSScriptRoot "results"
New-Item -ItemType Directory -Path $resultsPath -Force | Out-Null

foreach ($variant in @("before", "after")) {
    $sql = Join-Path $PSScriptRoot "postgres\explain-$variant.sql"
    $output = Join-Path $resultsPath "query-plans-$variant.txt"
    Get-Content -LiteralPath $sql -Raw |
        docker exec -i quest-load-postgres psql -X -U quest -d "quest_$variant" -v ON_ERROR_STOP=1 |
        Set-Content -LiteralPath $output
    if ($LASTEXITCODE -ne 0) { throw "Failed to collect $variant query plans" }
    Write-Host "Wrote $output"
}
