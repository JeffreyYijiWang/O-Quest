param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("before", "after")]
    [string]$Variant,
    [int]$PeakVus = 600,
    [string]$RampDuration = "5m",
    [string]$HoldDuration = "5m",
    [string]$RampDownDuration = "1m"
)

$ErrorActionPreference = "Stop"
$scriptPath = (Join-Path $PSScriptRoot "k6")
$resultsPath = (Join-Path $PSScriptRoot "results")
New-Item -ItemType Directory -Path $resultsPath -Force | Out-Null

docker run --rm --add-host host.docker.internal:host-gateway `
    -v "${scriptPath}:/scripts:ro" `
    -v "${resultsPath}:/results" `
    -e "BASE_URL=http://host.docker.internal:3000" `
    -e "RESULT_NAME=$Variant" `
    -e "PEAK_VUS=$PeakVus" `
    -e "RAMP_DURATION=$RampDuration" `
    -e "HOLD_DURATION=$HoldDuration" `
    -e "RAMP_DOWN_DURATION=$RampDownDuration" `
    grafana/k6:0.57.0 run /scripts/journey.js

if ($LASTEXITCODE -ne 0) {
    Write-Warning "k6 thresholds failed for $Variant; the summary artifact was still retained."
}
