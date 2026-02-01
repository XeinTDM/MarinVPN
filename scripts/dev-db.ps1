$ErrorActionPreference = "Stop"

docker compose up -d marinvpn-db | Out-Null

$retries = 30
while ($retries -gt 0) {
    $status = docker compose ps --status running --format json | ConvertFrom-Json | Where-Object { $_.Name -eq "marinvpn-db" }
    if ($null -ne $status -and $status.Health -eq "healthy") {
        Write-Host "Postgres is healthy."
        exit 0
    }
    Start-Sleep -Seconds 1
    $retries--
}

Write-Error "Postgres did not become healthy in time."
exit 1
