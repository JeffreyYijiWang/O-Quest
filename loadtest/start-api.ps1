param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("before", "after")]
    [string]$Variant
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent

$env:DATABASE_URL = "postgres://quest:quest@127.0.0.1:55432/quest_$Variant"
$env:MINIO_ENDPOINT = "http://127.0.0.1:59000"
$env:MINIO_ACCESS_KEY = "quest-minio"
$env:MINIO_SECRET_KEY = "quest-minio-secret"
$env:MINIO_BUCKET = "quest"

if ($Variant -eq "before") {
    $env:ENABLE_DEV_AUTH = "true"
    $env:OIDC_ISSUER_URL = "http://127.0.0.1/disabled"
    $env:OIDC_CLIENT_ID = "quest-loadtest"
    $binary = Join-Path $PSScriptRoot ".baseline-src\target\release\backend.exe"
} else {
    $env:REDIS_URL = "redis://127.0.0.1:56379"
    $env:REDIS_POOL_SIZE = "24"
    $env:DATABASE_MAX_CONNECTIONS = "64"
    $env:DATABASE_MIN_CONNECTIONS = "8"
    $env:SESSION_SECRET = "local-load-test-secret-at-least-32-bytes"
    $env:LOAD_TEST_MODE = "true"
    $env:ALLOW_DEV_AUTH = "false"
    $binary = Join-Path $repoRoot "backend\target\release\backend.exe"
}

if (-not (Test-Path -LiteralPath $binary)) {
    throw "Missing $binary. Build the selected variant first."
}

Write-Host "Starting $Variant API from $binary"
& $binary
