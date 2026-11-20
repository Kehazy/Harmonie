# Downloads the latest Spicetify release from GitHub and extracts it to
# %LOCALAPPDATA%\spicetify, like the official installer, without prompts.
# Prints the installed version on the last line.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$release = Invoke-RestMethod -Uri 'https://api.github.com/repos/spicetify/cli/releases/latest' -Headers @{ 'User-Agent' = 'Harmonie' }
$version = $release.tag_name.TrimStart('v')

$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
  'ARM64' { 'arm64' }
  'AMD64' { 'x64' }
  default { if ([Environment]::Is64BitOperatingSystem) { 'x64' } else { 'x32' } }
}
$name = "spicetify-$version-windows-$arch.zip"
$asset = $release.assets | Where-Object { $_.name -eq $name } | Select-Object -First 1
if (-not $asset) { throw "Spicetify $version has no download for windows-$arch." }

$target = if ($env:HARMONIE_SPICETIFY_DIR) { $env:HARMONIE_SPICETIFY_DIR } else { Join-Path $env:LOCALAPPDATA 'spicetify' }
New-Item -ItemType Directory -Force -Path $target | Out-Null

$zip = Join-Path ([IO.Path]::GetTempPath()) $name
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip -UseBasicParsing -Headers @{ 'User-Agent' = 'Harmonie' }
try {
  Expand-Archive -Path $zip -DestinationPath $target -Force
} finally {
  Remove-Item -Path $zip -Force -ErrorAction SilentlyContinue
}

# Make `spicetify` available in terminals too, as the official installer does.
if (-not $env:HARMONIE_SPICETIFY_DIR) {
  $userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
  if (($userPath -split ';') -notcontains $target) {
    [Environment]::SetEnvironmentVariable('PATH', (($userPath.TrimEnd(';'), $target) -join ';').TrimStart(';'), 'User')
  }
}

Write-Output $version
