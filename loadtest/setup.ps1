$ErrorActionPreference = "Stop"
$composeFile = Join-Path $PSScriptRoot "docker-compose.yml"
$schemaFile = Join-Path $PSScriptRoot "postgres\schema-and-seed.sql"
$indexFile = Join-Path $PSScriptRoot "postgres\performance-indexes.sql"

docker compose -f $composeFile up -d postgres redis minio
if ($LASTEXITCODE -ne 0) { throw "Failed to start local dependencies" }

$deadline = (Get-Date).AddSeconds(60)
do {
    $status = docker inspect --format '{{.State.Health.Status}}' quest-load-postgres 2>$null
    if ($status -eq "healthy") { break }
    Start-Sleep -Seconds 2
} while ((Get-Date) -lt $deadline)
if ($status -ne "healthy") { throw "PostgreSQL did not become healthy" }

foreach ($database in @("quest_before", "quest_after")) {
    docker exec quest-load-postgres psql -U quest -d postgres -v ON_ERROR_STOP=1 -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='$database';"
    docker exec quest-load-postgres psql -U quest -d postgres -v ON_ERROR_STOP=1 -c "DROP DATABASE IF EXISTS $database;"
    docker exec quest-load-postgres psql -U quest -d postgres -v ON_ERROR_STOP=1 -c "CREATE DATABASE $database;"
    Get-Content -LiteralPath $schemaFile -Raw | docker exec -i quest-load-postgres psql -U quest -d $database -v ON_ERROR_STOP=1
    if ($LASTEXITCODE -ne 0) { throw "Failed to seed $database" }
}

Get-Content -LiteralPath $indexFile -Raw | docker exec -i quest-load-postgres psql -U quest -d quest_after -v ON_ERROR_STOP=1
if ($LASTEXITCODE -ne 0) { throw "Failed to apply performance indexes" }

docker exec quest-load-redis redis-cli FLUSHALL | Out-Null
Write-Host "Local benchmark data is ready on PostgreSQL :55432, Redis :56379, and MinIO :59000."
