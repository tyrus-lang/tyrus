[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet('release', 'debug')]
    [string]$BuildProfile = 'release',

    [switch]$Check
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$projectDir = Split-Path -Parent $PSScriptRoot
$target = 'wasm32-unknown-unknown'
$library = Join-Path $projectDir "target/$target/$BuildProfile/libtyrus_runtime_wasm.a"
$compiler = Join-Path $projectDir 'target/debug/tyrus.exe'

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

Push-Location $projectDir
try {
    $installedTargets = & rustup target list --installed
    if ($LASTEXITCODE -ne 0) { throw 'Could not inspect installed Rust targets.' }
    if ($installedTargets -notcontains $target) {
        throw "The '$target' target is not installed. Install it with: rustup target add $target"
    }

    Write-Host "==> Building the WASM runtime static library ($BuildProfile)"
    $cargoArguments = @('build', '--package', 'tyrus-runtime-wasm', '--target', $target)
    if ($BuildProfile -eq 'release') { $cargoArguments += '--release' }
    Invoke-CheckedCommand cargo $cargoArguments
    if (-not (Test-Path -LiteralPath $library -PathType Leaf)) {
        throw "The build succeeded but '$library' is missing."
    }
    Write-Host "==> Built $library"

    Write-Host '==> Rebuilding the compiler so it matches the runtime'
    Invoke-CheckedCommand cargo @('build', '--package', 'tyrus')
    if (-not $Check) {
        Write-Host 'WASM runtime is current. Run .\scripts\wasm.ps1 -Check to verify a program links.'
        return
    }
    if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) {
        throw "The compiler executable '$compiler' is missing after its build."
    }

    Write-Host '==> Linking the Fibonacci example and runtime string operations'
    $workspace = Join-Path ([IO.Path]::GetTempPath()) "tyrus-wasm-link-check-$([guid]::NewGuid().ToString('N'))"
    try {
        $sourceDir = Join-Path $workspace 'src'
        New-Item -ItemType Directory -Force -Path $sourceDir | Out-Null
        $config = '{ "project": { "name": "wasm-link-check", "version": "0.1.0" } }' + "`n"
        $source = @'
function fibonacci(n: i32): i32 {
    if (n <= 1) {
        return n;
    }

    return fibonacci(n - 1) + fibonacci(n - 2);
}

const main = (): void => {
    print(`fibonacci(10) = ${fibonacci(10)}`);
    print("runtime return".toUpperCase());
};
'@
        $utf8 = [Text.UTF8Encoding]::new($false)
        [IO.File]::WriteAllText((Join-Path $workspace 'tyrus.json'), $config, $utf8)
        [IO.File]::WriteAllText((Join-Path $sourceDir 'main.tyr'), $source, $utf8)

        $previousRuntime = $env:TYRUS_WASM_RUNTIME_LIB
        try {
            $env:TYRUS_WASM_RUNTIME_LIB = $library
            Push-Location $workspace
            try {
                Invoke-CheckedCommand $compiler @('build', '--compile-target', 'wasm')
                $wasmOutput = Join-Path $workspace 'target/wasm-link-check.wasm'
                if (-not (Test-Path -LiteralPath $wasmOutput -PathType Leaf)) {
                    throw "The linker exited successfully but '$wasmOutput' was not created."
                }
            } finally {
                Pop-Location
            }
        } finally {
            if ($null -eq $previousRuntime) {
                Remove-Item Env:TYRUS_WASM_RUNTIME_LIB -ErrorAction SilentlyContinue
            } else {
                $env:TYRUS_WASM_RUNTIME_LIB = $previousRuntime
            }
        }
    } finally {
        Remove-Item $workspace -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Host 'WASM runtime is current and the program linked successfully.'
} finally {
    Pop-Location
}
