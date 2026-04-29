param(
    [string]$Selector = "",
    [int]$Limit = 1000,
    [int]$Offset = 0
)

$ErrorActionPreference = "Stop"
$env:SYMM_PERF_LOG = "1"

Write-Host "[baseline] ls table"
symm ls --limit $Limit --offset $Offset *> $null

Write-Host "[baseline] ls json"
symm ls --json --limit $Limit --offset $Offset *> $null

if ($Selector -ne "") {
    Write-Host "[baseline] show table"
    symm show $Selector *> $null

    Write-Host "[baseline] show json"
    symm show $Selector --json *> $null
}

Write-Host "[baseline] done (check stderr for [symm-perf] lines)"
