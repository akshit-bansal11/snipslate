# The one quality gate. Formats in place by default; -Ci is the non-mutating twin that CI runs.
param([switch]$Ci)

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

$projects = @('src/Snipslate/Snipslate.csproj', 'tests/Snipslate.Tests/Snipslate.Tests.csproj')

function Invoke-Step([string[]]$Arguments) {
    Write-Host "> dotnet $($Arguments -join ' ')"
    & dotnet @Arguments
    if ($LASTEXITCODE -ne 0) { throw "dotnet $($Arguments -join ' ') failed with exit code $LASTEXITCODE" }
}

foreach ($project in $projects) {
    if ($Ci) { Invoke-Step @('format', $project, '--verify-no-changes') }
    else { Invoke-Step @('format', $project) }
}

Invoke-Step @('build', 'tests/Snipslate.Tests/Snipslate.Tests.csproj', '-c', 'Release', '-warnaserror')
Invoke-Step @('test', 'tests/Snipslate.Tests/Snipslate.Tests.csproj', '-c', 'Release', '--no-build')
