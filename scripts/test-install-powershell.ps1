# Offline PowerShell parser/installer fixtures for Unix x64 CI hosts.
# Native Windows execution-policy enforcement requires a Windows acceptance host.
$ErrorActionPreference = 'Stop'
Import-Module Microsoft.PowerShell.Archive
$root = Join-Path ([IO.Path]::GetTempPath()) ('mirrorproxy-ps-test-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$source = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'install.ps1'))
$installer = [scriptblock]::Create($source)
function global:Invoke-WebRequest {
    param([switch]$UseBasicParsing, [string]$Uri, [string]$OutFile)
    if ($Uri.EndsWith('.sha256')) { Copy-Item $global:FixtureChecksum $OutFile } else { Copy-Item $global:FixtureArchive $OutFile }
}
function global:Expand-Archive {
    param([string]$Path, [string]$DestinationPath, [switch]$Force)
    Microsoft.PowerShell.Archive\Expand-Archive -Path $Path -DestinationPath $DestinationPath -Force:$Force
    & chmod +x (Join-Path $DestinationPath 'mirrorproxy.exe')
}
try {
    foreach ($case in @('supported', 'setup-failure', 'old-client', 'bad-checksum', 'empty-checksum')) {
        $caseRoot = Join-Path $root $case
        $payload = Join-Path $caseRoot 'payload'
        $installed = Join-Path $caseRoot 'installed'
        New-Item -ItemType Directory -Path $payload, $installed | Out-Null
        $env:MIRRORPROXY_SETUP_CAPTURE = Join-Path $caseRoot 'captured-args'
        $body = if ($case -eq 'old-client') { "#!/bin/sh`nexit 2`n" } elseif ($case -eq 'setup-failure') { '#!/bin/sh' + "`n" + 'if [ "$2" = "--help" ]; then exit 0; fi' + "`nexit 3`n" } else { '#!/bin/sh' + "`n" + 'printf ''%s\n'' "$@" > "$MIRRORPROXY_SETUP_CAPTURE"' + "`n" }
        $binary = Join-Path $payload 'mirrorproxy.exe'
        [IO.File]::WriteAllText($binary, $body, [Text.UTF8Encoding]::new($false))
        $global:FixtureArchive = Join-Path $caseRoot 'client.zip'
        Compress-Archive -Path $binary -DestinationPath $global:FixtureArchive
        $hash = if ($case -eq 'bad-checksum') { '0' * 64 } else { (Get-FileHash $global:FixtureArchive -Algorithm SHA256).Hash }
        $global:FixtureChecksum = Join-Path $caseRoot 'client.sha256'
        [IO.File]::WriteAllText($global:FixtureChecksum, $(if ($case -eq 'empty-checksum') { '' } else { $hash + '  client.zip' }))
        $destination = Join-Path $installed 'mirrorproxy.exe'
        [IO.File]::WriteAllText($destination, 'previous client')
        $failure = $null
        $inputCodePage = [Console]::InputEncoding.CodePage
        $outputCodePage = [Console]::OutputEncoding.CodePage
        try { & $installer -Mirror 'https://mirror.example' -Version fixture -InstallDir $installed -Setup -BaseUrl 'https://personal.mirror.example' } catch { $failure = $_.Exception.Message }
        if ([Console]::InputEncoding.CodePage -ne $inputCodePage -or [Console]::OutputEncoding.CodePage -ne $outputCodePage) { throw 'Console encoding was not restored' }
        if ($case -eq 'supported') {
            if ($failure) { throw $failure }
            $argsText = [IO.File]::ReadAllText($env:MIRRORPROXY_SETUP_CAPTURE)
            if ($argsText -ne "setup`n--base-url`nhttps://personal.mirror.example`n") { throw "unexpected setup arguments: $argsText" }
        } elseif ($case -eq 'setup-failure') {
            if (-not $failure -or -not $failure.Contains('setup did not complete')) { throw "unexpected setup failure: $failure" }
        } else {
            $expected = if ($case -eq 'old-client') { 'does not support setup' } elseif ($case -eq 'empty-checksum') { 'checksum file is empty or invalid' } else { 'Checksum verification failed' }
            if (-not $failure -or -not $failure.Contains($expected)) { throw "unexpected failure: $failure" }
            if ([IO.File]::ReadAllText($destination) -ne 'previous client') { throw 'existing client was overwritten' }
            if (Test-Path $env:MIRRORPROXY_SETUP_CAPTURE) { throw 'client was executed unexpectedly' }
        }
        Write-Host "PASS PowerShell offline fixture: $case"
    }
} finally { Remove-Item -Recurse -Force $root }

# Expected negative native exits were asserted above; report successful fixture execution.
$global:LASTEXITCODE = 0
