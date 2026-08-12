param(
    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 5000)]
    [int]$Requests,
    [Parameter(Mandatory = $true)]
    [string]$RewardName,
    [Parameter(Mandatory = $true)]
    [string]$SummaryName,
    [bool]$SameUser = $false,
    [string]$UserPrefix = "stress"
)

$ErrorActionPreference = "Stop"
$scriptPath = Join-Path $PSScriptRoot "k6"
$resultsPath = Join-Path $PSScriptRoot "results"
New-Item -ItemType Directory -Path $resultsPath -Force | Out-Null

docker run --rm --add-host host.docker.internal:host-gateway `
    -v "${scriptPath}:/scripts:ro" `
    -v "${resultsPath}:/results" `
    -e "BASE_URL=http://host.docker.internal:3000" `
    -e "REQUESTS=$Requests" `
    -e "REWARD_NAME=$RewardName" `
    -e "SAME_USER=$($SameUser.ToString().ToLowerInvariant())" `
    -e "USER_PREFIX=$UserPrefix" `
    -e "SUMMARY_NAME=$SummaryName" `
    grafana/k6:0.57.0 run /scripts/redemption-contention.js

if ($LASTEXITCODE -ne 0) {
    throw "Redemption stress run failed before reconciliation."
}
