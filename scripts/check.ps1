# The one quality gate. Formats in place by default; -Ci is the non-mutating twin that CI runs.
param([switch]$Ci)

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

function Invoke-Step([string[]]$Arguments) {
    Write-Host "> cargo $($Arguments -join ' ')"
    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) { throw "cargo $($Arguments -join ' ') failed with exit code $LASTEXITCODE" }
}

if ($Ci) { Invoke-Step @('fmt', '--check') }
else { Invoke-Step @('fmt') }

Invoke-Step @('clippy', '--all-targets', '--locked', '--', '-D', 'warnings')
Invoke-Step @('test', '--locked')
