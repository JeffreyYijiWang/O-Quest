$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent
$binary = Join-Path $repoRoot "backend\target\release\backend.exe"

if (-not (Test-Path -LiteralPath $binary)) {
    throw "Missing optimized backend binary at $binary"
}

$env:DATABASE_URL = "postgres://quest:quest@127.0.0.1:55432/quest_redemption_stress_20260810"
$env:DATABASE_MAX_CONNECTIONS = "64"
$env:DATABASE_MIN_CONNECTIONS = "8"
$env:REDIS_URL = "redis://127.0.0.1:56379"
$env:REDIS_POOL_SIZE = "24"
$env:MINIO_ENDPOINT = "http://127.0.0.1:59000"
$env:MINIO_ACCESS_KEY = "quest-minio"
$env:MINIO_SECRET_KEY = "quest-minio-secret"
$env:MINIO_BUCKET = "quest"
$env:SESSION_SECRET = "local-redemption-stress-secret-32-bytes"
$env:LOAD_TEST_MODE = "true"
$env:ALLOW_DEV_AUTH = "false"

& $binary
