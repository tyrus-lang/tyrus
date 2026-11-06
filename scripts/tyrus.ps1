$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$projectDir = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$installDir = if ($env:TYRUS_INSTALL_DIR) { $env:TYRUS_INSTALL_DIR } else { Join-Path $cargoHome 'bin' }
$runtimeDir = if ($env:TYRUS_RUNTIME_DIR) { $env:TYRUS_RUNTIME_DIR } else { Join-Path $cargoHome 'lib/tyrus' }
$systemRuntimeTargetDir = Join-Path $projectDir 'target/tyrus-system-runtime'
$nativeLibsManifest = Join-Path $projectDir 'target/release/tyrus-native-libs.json'

function Invoke-CheckedCommand {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string[]]$CommandArguments
    )
    & $Command @CommandArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $Command $($CommandArguments -join ' ')"
    }
}

function Copy-Atomically {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination
    )
    $destinationDirectory = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Force -Path $destinationDirectory | Out-Null
    $temporary = Join-Path $destinationDirectory ".tyrus-$([guid]::NewGuid().ToString('N')).tmp"
    try {
        Copy-Item -LiteralPath $Source -Destination $temporary
        Move-Item -LiteralPath $temporary -Destination $Destination -Force
    } finally {
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
}

function Write-NativeLinkManifest {
    param([switch]$Release)
    $profile = if ($Release) { 'release' } else { 'debug' }
    $manifest = Join-Path $projectDir "target/$profile/tyrus-native-libs.json"
    $nativeOutput = Join-Path $env:TEMP "tyrus-native-libs-$([guid]::NewGuid().ToString('N')).txt"
    $cargoArguments = @('rustc')
    if ($Release) { $cargoArguments += '--release' }
    $cargoArguments += @('--package', 'tyrus-runtime', '--lib', '--', '--print=native-static-libs')
    try {
        $nativeOutputLines = & cargo @cargoArguments 2>&1
        $nativeStatus = $LASTEXITCODE
        $nativeOutputLines | ForEach-Object { Write-Host $_ }
        $nativeOutputLines | ForEach-Object { "$_" } | Set-Content -LiteralPath $nativeOutput -Encoding utf8
        if ($nativeStatus -ne 0) {
            throw "Could not collect $profile native linker arguments from tyrus-runtime."
        }
        $rustcVersionOutput = & rustc -vV
        if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the active Rust toolchain.' }
        $hostLine = ($rustcVersionOutput | Select-String '^host: ' | Select-Object -First 1).Line
        if (-not $hostLine) { throw 'Could not determine the active Rust target triple.' }
        $nativeTarget = ($hostLine -split '\s+', 2)[1]
        Invoke-CheckedCommand python @('scripts/ci/extract_native_static_libs.py', $nativeOutput, $manifest, '--target', $nativeTarget)
    } finally {
        Remove-Item -LiteralPath $nativeOutput -Force -ErrorAction SilentlyContinue
    }
}

Push-Location $projectDir
try {
    Write-Host '==> Building the runtime static library for tests'
    Invoke-CheckedCommand cargo @('build', '--package', 'tyrus-runtime')
    Write-Host '==> Collecting debug runtime linker arguments for tests'
    Write-NativeLinkManifest

    Write-Host '==> Running workspace tests'
    $testOutput = & cargo test --workspace 2>&1
    $testStatus = $LASTEXITCODE
    $testOutput | ForEach-Object { Write-Host $_ }
    $testResults = [regex]::Matches(($testOutput -join "`n"), 'test result:.*?([0-9]+) passed; ([0-9]+) failed')
    $testsPassed = 0
    $testsFailed = 0
    foreach ($testResult in $testResults) {
        $testsPassed += [int]$testResult.Groups[1].Value
        $testsFailed += [int]$testResult.Groups[2].Value
    }
    Write-Host "==> Test summary: $testsPassed passed, $testsFailed failed"
    if ($testStatus -ne 0) {
        throw "Workspace tests failed with exit code $testStatus."
    }

    Write-Host '==> Building the compiler and runtime in release mode'
    Invoke-CheckedCommand cargo @('build', '--release', '--package', 'tyrus', '--package', 'tyrus-runtime')

    Write-Host '==> Collecting Windows runtime linker arguments'
    Write-NativeLinkManifest -Release

    Write-Host '==> Building the system-allocator runtime for sanitizer examples'
    $previousTargetDir = $env:CARGO_TARGET_DIR
    try {
        $env:CARGO_TARGET_DIR = $systemRuntimeTargetDir
        Invoke-CheckedCommand cargo @('build', '--release', '--package', 'tyrus-runtime', '--features', 'system-allocator')
    } finally {
        $env:CARGO_TARGET_DIR = $previousTargetDir
    }

    Write-Host "==> Installing tyrus into $installDir"
    Copy-Atomically (Join-Path $projectDir 'target/release/tyrus.exe') (Join-Path $installDir 'tyrus.exe')

    Write-Host "==> Installing the runtime into $runtimeDir"
    Copy-Atomically (Join-Path $projectDir 'target/release/tyrus_runtime.lib') (Join-Path $runtimeDir 'tyrus_runtime.lib')
    Copy-Atomically $nativeLibsManifest (Join-Path $runtimeDir 'tyrus-native-libs.json')
    Copy-Atomically (Join-Path $systemRuntimeTargetDir 'release/tyrus_runtime.lib') (Join-Path $runtimeDir 'tyrus_runtime_system.lib')

    Write-Host '==> tyrus updated successfully'
    & (Join-Path $installDir 'tyrus.exe') --version
    if ($LASTEXITCODE -ne 0) { throw 'The installed Tyrus executable failed to start.' }
    Write-Host '    Reload the editor to restart any running Language Server.'
} finally {
    Pop-Location
}
