# 建置並發佈新版。用法：
#   1. 把 src-tauri/tauri.conf.json 的 "version" 改成新版本號並 commit
#   2. powershell -ExecutionPolicy Bypass -File scripts\release.ps1 -Notes "這一版改了什麼"
param([Parameter(Mandatory = $true)][string]$Notes)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$conf = Get-Content (Join-Path $root 'src-tauri\tauri.conf.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$version = $conf.version
$tag = "v$version"
$owner = (gh api user --jq .login).Trim()
if ($owner -ne 'junchen741') { throw "gh 目前登入的是 $owner，請先切換成 junchen741（gh auth switch）再發佈" }
$repo = "$owner/ytgrab"
# 檔名固定、不含版本號：releases/latest/download/YTGrab-Setup.exe 這個連結就永遠指向最新版
$asset = 'YTGrab-Setup.exe'

if (git -C $root status --porcelain) { throw '還有未提交的變更，請先 commit 再發佈' }
if (git -C $root tag -l $tag) { throw "$tag 已經發佈過，請先把 tauri.conf.json 的 version 改成新版本號" }

if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    $env:TAURI_SIGNING_PRIVATE_KEY = Join-Path $HOME '.tauri\ytgrab.key'
}
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
    $secure = Read-Host '請輸入更新簽署金鑰的密碼' -AsSecureString
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure))
}

Push-Location $root
try {
    # 每次發佈都附上最新版的 yt-dlp / ffmpeg / deno
    powershell -ExecutionPolicy Bypass -File scripts\fetch-binaries.ps1
    if ($LASTEXITCODE -ne 0) { throw '下載外部工具失敗' }
    npm run tauri build
    if ($LASTEXITCODE -ne 0) { throw 'tauri build 失敗' }
}
finally {
    Pop-Location
}

$nsis = Join-Path $root 'src-tauri\target\release\bundle\nsis'
$setup = Get-ChildItem $nsis -Filter "*_${version}_x64-setup.exe" | Select-Object -First 1
if (-not $setup) { throw "找不到 v$version 的安裝檔" }
$signature = (Get-Content "$($setup.FullName).sig" -Raw).Trim()

# 安裝檔原名含中文、空白和版本號，上傳前改成固定的英文檔名；簽章是針對檔案內容，改名不影響
$out = Join-Path $root "dist-release\$tag"
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item $setup.FullName (Join-Path $out $asset) -Force

$latest = [ordered]@{
    version   = $version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = @{
        'windows-x86_64' = @{
            signature = $signature
            # 指向這一版自己的網址（不是 latest），所以每一版用同一個檔名也不會混淆
            url       = "https://github.com/$repo/releases/download/$tag/$asset"
        }
    }
}
# 一定要寫成沒有 BOM 的 UTF-8，否則 updater 解析 JSON 會失敗
[IO.File]::WriteAllText((Join-Path $out 'latest.json'), ($latest | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))

# 先推程式碼和版本標籤，再建立 Release。順序不能反過來：先建 Release 的話，
# GitHub 會在遠端還沒更新的舊 commit 上自動建立標籤，跟本機的標籤對不起來。
git -C $root push origin HEAD
if ($LASTEXITCODE -ne 0) { throw 'git push 失敗；遠端有新的 commit 的話，先 git pull 再發佈' }
git -C $root tag $tag
git -C $root push origin $tag
if ($LASTEXITCODE -ne 0) { throw "推送標籤 $tag 失敗" }

gh release create $tag (Join-Path $out $asset) (Join-Path $out 'latest.json') --repo $repo --verify-tag --title "YTGrab 音樂抓抓 $tag" --notes $Notes
if ($LASTEXITCODE -ne 0) { throw '建立 Release 失敗；標籤已經推上去了，修好問題後只要重跑上面這行 gh release create' }

Write-Host "已發佈 ${tag}：https://github.com/$repo/releases/tag/$tag"
Write-Host "固定下載連結（永遠指向最新版）：https://github.com/$repo/releases/latest/download/$asset"
