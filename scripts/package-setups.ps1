# Builds one standalone installer per app in dist\setup (Pixel-Setup.exe, Lumen-Setup.exe…).
# Heavy: compiles every app in release. Run it yourself when the PC is free:
#   powershell -ExecutionPolicy Bypass -File scripts\package-setups.ps1
# Add -Apps pixel,sound to package only some apps.
param([string[]]$Apps = @("pixel", "lumen", "vector", "cut", "motion", "page", "folio", "sound", "ethone-studio"))

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$packages = @("ethone-setup") + $Apps
cargo build --release @($packages | ForEach-Object { "-p", $_ })
if ($LASTEXITCODE -ne 0) { throw "compilation échouée" }

$out = Join-Path $root "dist\setup"
New-Item -ItemType Directory -Force $out | Out-Null
$stub = Join-Path $root "target\release\ethone-setup.exe"
foreach ($id in $Apps) {
    $manifest = Join-Path $root "apps\$($id -replace '^ethone-studio$', 'launcher')\Cargo.toml"
    $version = (Select-String -Path $manifest -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
    $name = if ($id -eq "ethone-studio") { "Ethone-Studio" } else { (Get-Culture).TextInfo.ToTitleCase($id) }
    $target = Join-Path $out "$name-Setup.exe"
    & $stub --pack (Join-Path $root "target\release\$id.exe") $id $version $target
    if ($LASTEXITCODE -ne 0) { throw "emballage de $id échoué" }
    "{0,-28} {1,8:N1} Mo" -f (Split-Path $target -Leaf), ((Get-Item $target).Length / 1MB)
}
