$ErrorActionPreference = 'Stop'
$repository = 'tyrus-lang/tyrus'

if (-not [Environment]::Is64BitOperatingSystem) {
    throw 'Tyrus currently provides a Windows x86_64 release.'
}

$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repository/releases/latest" -Headers @{ 'Accept' = 'application/vnd.github+json' }
$tag = [string]$release.tag_name
if ([string]::IsNullOrWhiteSpace($tag)) {
    throw 'Could not determine the latest Tyrus release.'
}
if ($tag -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$') {
    throw 'GitHub returned an invalid Tyrus release tag.'
}

Write-Host "Installing Tyrus $tag. Please wait..."
$archive = "tyrus-$tag-windows-x86_64.zip"
$releaseUrl = "https://github.com/$repository/releases/download/$tag"
$temporary = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString('N'))
$installRoot = Join-Path $env:LOCALAPPDATA 'Tyrus'
$archivePath = Join-Path $temporary $archive
$checksumPath = "$archivePath.sha256"

try {
    New-Item -ItemType Directory -Force -Path $temporary, $installRoot | Out-Null
    Write-Host "Downloading $archive..."
    Invoke-WebRequest -Uri "$releaseUrl/$archive" -OutFile $archivePath
    Write-Host 'Downloading checksum...'
    Invoke-WebRequest -Uri "$releaseUrl/$archive.sha256" -OutFile $checksumPath

    Write-Host 'Verifying release package checksum...'
    $checksumLine = (Get-Content -Raw $checksumPath).Trim()
    $checksumMatch = [regex]::Match($checksumLine, '^([0-9a-f]{64})  ([^\r\n]+)$')
    if (-not $checksumMatch.Success -or $checksumMatch.Groups[2].Value -ne $archive) {
        throw 'The release checksum file has an invalid format.'
    }
    $actualHash = (Get-FileHash -Algorithm SHA256 $archivePath).Hash.ToLowerInvariant()
    if ($actualHash -ne $checksumMatch.Groups[1].Value) {
        throw 'Release package checksum verification failed.'
    }
    Write-Host 'Checksum verified.'

    $destination = Join-Path $installRoot "tyrus-$tag-windows-x86_64"
    if (Test-Path $destination) { Remove-Item -Recurse -Force $destination }
    Write-Host "Extracting and installing Tyrus $tag..."
    Expand-Archive -Path $archivePath -DestinationPath $installRoot -Force

    $binPath = Join-Path $destination 'bin'
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $pathEntries = @($userPath -split ';' | Where-Object { $_ })
    if ($pathEntries -notcontains $binPath) {
        [Environment]::SetEnvironmentVariable('Path', (($pathEntries + $binPath) -join ';'), 'User')
    }
    $env:Path = "$binPath;$env:Path"
    & (Join-Path $binPath 'tyrus.exe') --version
    if ($LASTEXITCODE -ne 0) { throw 'Tyrus was installed, but its version check failed.' }
    Write-Host "Tyrus $tag installed successfully. Restart PowerShell to use the updated user PATH in new sessions."
}
finally {
    Remove-Item -Recurse -Force $temporary -ErrorAction SilentlyContinue
}
