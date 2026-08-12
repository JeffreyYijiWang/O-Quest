param(
    [string]$Commit = "ffb70c357e89466fc7d7b0dbfcd3e9d679e2c67a"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent
$baselineDir = Join-Path $PSScriptRoot ".baseline-src"
$archive = Join-Path ([System.IO.Path]::GetTempPath()) "quest-baseline-$PID.tar"

function Replace-Exact([string]$Path, [string]$Old, [string]$New) {
    $content = [System.IO.File]::ReadAllText($Path)
    if (-not $content.Contains($Old)) {
        throw "Expected baseline text was not found in $Path"
    }
    $content = $content.Replace($Old, $New)
    [System.IO.File]::WriteAllText($Path, $content, [System.Text.UTF8Encoding]::new($false))
}

if (Test-Path -LiteralPath $baselineDir) {
    Remove-Item -LiteralPath $baselineDir -Recurse -Force
}
New-Item -ItemType Directory -Path $baselineDir | Out-Null

try {
    git -C $repoRoot archive --format=tar --output=$archive "$Commit`:backend"
    if ($LASTEXITCODE -ne 0) { throw "git archive failed" }
    tar -xf $archive -C $baselineDir
    if ($LASTEXITCODE -ne 0) { throw "archive extraction failed" }

    $cargoPath = Join-Path $baselineDir "Cargo.toml"
    $mainPath = Join-Path $baselineDir "src\main.rs"
    $authPath = Join-Path $baselineDir "src\auth.rs"

    Replace-Exact $cargoPath 'utoipa-swagger-ui = { version = "9.0.2", features = ["axum"] }' ''
    Replace-Exact $mainPath 'use utoipa_swagger_ui::SwaggerUi;' ''
    Replace-Exact $mainPath 'if cfg!(debug_assertions) && dotenvy::var("ENABLE_DEV_AUTH").is_ok() {' 'if dotenvy::var("ENABLE_DEV_AUTH").is_ok() {'
    Replace-Exact $mainPath '    let app = router.merge(SwaggerUi::new("/swagger").url("/openapi.json", api));' @'
    let app = router.route(
        "/openapi.json",
        axum::routing::get(move || {
            let api = api.clone();
            async move { axum::Json(api) }
        }),
    );
'@

    Replace-Exact $authPath '    // Mock the JWT claims that would normally come from the gateway' @'
    // Benchmark-only identity injection. Request handling, SQL, and cache logic
    // remain identical to the frozen baseline.
    let user_id = request
        .headers()
        .get("x-quest-user-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("devuser")
        .to_string();
    let user_name = request
        .headers()
        .get("x-quest-user-name")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("Load Test User")
        .to_string();

    // Mock the JWT claims that would normally come from the gateway.
'@
    Replace-Exact $authPath '        sub: "devuser".to_string(),' '        sub: user_id.clone(),'
    Replace-Exact $authPath '        email: "dev@example.com".to_string(),' '        email: format!("{user_id}@load.invalid"),'
    Replace-Exact $authPath '        name: "Dev User".to_string(),' '        name: user_name.clone(),'
    Replace-Exact $authPath '        given_name: "Dev User".to_string(),' '        given_name: user_name,'
    Replace-Exact $authPath '        preferred_username: "devuser".to_string(),' '        preferred_username: user_id.clone(),'
    Replace-Exact $authPath '        nickname: "devuser".to_string(),' '        nickname: user_id,'

    cargo build --manifest-path (Join-Path $baselineDir "Cargo.toml") --release --offline
    if ($LASTEXITCODE -ne 0) { throw "baseline build failed" }
}
finally {
    if (Test-Path -LiteralPath $archive) {
        Remove-Item -LiteralPath $archive -Force
    }
}

$binary = Join-Path $baselineDir "target\release\backend.exe"
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary).Hash
Write-Host "Baseline binary: $binary"
Write-Host "SHA256: $hash"
