# 下載 yt-dlp、ffmpeg（BtbN LGPL 正式版分支）、deno 到 src-tauri/binaries，供開發與打包使用。
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'  # Windows PowerShell 5.1 顯示進度條會讓下載慢很多
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$root = Split-Path $PSScriptRoot -Parent
$bin = Join-Path $root 'src-tauri\binaries'
$tmp = Join-Path $env:TEMP "ytgrab-fetch-$(Get-Random)"
New-Item -ItemType Directory -Force $bin, $tmp | Out-Null

try {
    Write-Host '下載 yt-dlp...'
    Invoke-WebRequest -UseBasicParsing 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe' -OutFile (Join-Path $bin 'yt-dlp.exe')

    Write-Host '下載 ffmpeg（LGPL）...'
    $rel = Invoke-RestMethod 'https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/tags/latest' -Headers @{ 'User-Agent' = 'ytgrab' }
    $asset = $rel.assets |
        Where-Object { $_.name -match '^ffmpeg-n[\d.]+-latest-win64-lgpl-[\d.]+\.zip$' } |
        Sort-Object { [version]($_.name -replace '^ffmpeg-n([\d.]+)-.*$', '$1') } -Descending |
        Select-Object -First 1
    if (-not $asset) { throw '找不到 BtbN 的 win64 LGPL 版 ffmpeg' }
    Write-Host "  使用 $($asset.name)"
    $zip = Join-Path $tmp 'ffmpeg.zip'
    Invoke-WebRequest -UseBasicParsing $asset.browser_download_url -OutFile $zip
    Expand-Archive $zip -DestinationPath (Join-Path $tmp 'ffmpeg')
    Get-ChildItem (Join-Path $tmp 'ffmpeg') -Recurse -Filter 'ffmpeg.exe' |
        Select-Object -First 1 |
        Copy-Item -Destination (Join-Path $bin 'ffmpeg.exe')

    Write-Host '下載 deno...'
    $dz = Join-Path $tmp 'deno.zip'
    Invoke-WebRequest -UseBasicParsing 'https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip' -OutFile $dz
    Expand-Archive $dz -DestinationPath (Join-Path $tmp 'deno')
    Copy-Item (Join-Path $tmp 'deno\deno.exe') (Join-Path $bin 'deno.exe')
}
finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "yt-dlp: $(& (Join-Path $bin 'yt-dlp.exe') --version)"
Write-Host "deno:   $(& (Join-Path $bin 'deno.exe') --version | Select-Object -First 1)"
$mp3 = & (Join-Path $bin 'ffmpeg.exe') -hide_banner -encoders | Select-String 'libmp3lame'
if (-not $mp3) { throw 'ffmpeg 沒有 libmp3lame 編碼器，無法輸出 MP3' }
Write-Host 'ffmpeg: 有 libmp3lame 編碼器'
