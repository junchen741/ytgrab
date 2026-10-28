<#
  開發用：用 Edge 無頭模式拍下 preview.html 的每個畫面，並檢查有沒有橫向捲軸。
  先在另一個終端機執行 npm run dev（port 1420）。

  用法：powershell -File scripts/preview-shots.ps1 [-OutDir <資料夾>] [-Only <案例>]
  每個案例、每種尺寸輸出一行「<案例> <寬>x<高> overflow=<yes|no>」；任何一個不是 no 就以 exit code 1 結束。
#>
param(
  [string]$OutDir = (Join-Path $env:TEMP 'ytgrab-preview'),
  [string]$Only = '',
  [string]$BaseUrl = 'http://localhost:1420'
)

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$edge = Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'
if (-not (Test-Path $edge)) { Write-Error "找不到 Edge：$edge"; exit 2 }
# 用獨立的設定檔資料夾，避免和使用者正在開的 Edge 衝突
$profileDir = Join-Path $env:TEMP 'ytgrab-preview-edge'
New-Item -ItemType Directory -Force $OutDir | Out-Null
$OutDir = (Resolve-Path $OutDir).Path

$cases = @('home', 'home-long', 'progress', 'progress-long', 'progress-wait', 'editor', 'editor-long', 'done', 'done-long', 'error', 'error-long')
if ($Only) { $cases = @($cases | Where-Object { $_ -eq $Only }) }
if ($cases.Count -eq 0) { Write-Error "沒有這個案例：$Only"; exit 2 }
$sizes = @(@(720, 600), @(900, 740))
# 這兩個案例多拍一張整頁的（只看畫面，不檢查捲軸）
$fullPage = @{ 'editor' = 1700; 'editor-long' = 1700 }

function Get-CaseUrl([string]$case) {
  $view = $case -replace '-long$', ''
  $url = "$BaseUrl/preview.html?view=$view"
  if ($case -like '*-long') { $url += '&long=1' }
  return $url
}

# Edge 是視窗程式，用 & 呼叫不會等它結束也收不到輸出，所以改用 Start-Process -Wait，輸出導到暫存檔。
# Start-Process 不會幫參數加引號，含空白的參數要自己包起來。
# 截圖用真實等待（--timeout）：用虛擬時間（--virtual-time-budget）的話，wavesurfer 的區段畫不出來。
# 檢查捲軸則要用虛擬時間：--timeout 會在預覽頁的計時器觸發前就輸出 DOM，抓不到 data-ready。
$shotWait = '--timeout=3500'
$domWait = '--virtual-time-budget=4000'
$domFile = Join-Path $env:TEMP 'ytgrab-preview-dom.html'
function Invoke-Edge([string[]]$extra, [int]$w, [int]$h, [string]$url) {
  $edgeArgs = @('--headless=new', '--disable-gpu', '--hide-scrollbars', "--user-data-dir=$profileDir",
    "--window-size=$w,$h") + $extra + @($url)
  $quoted = $edgeArgs | ForEach-Object { if ($_ -match '\s') { '"' + $_ + '"' } else { $_ } }
  Start-Process -FilePath $edge -ArgumentList $quoted -Wait -NoNewWindow -RedirectStandardOutput $domFile -RedirectStandardError "$domFile.err"
  return [System.IO.File]::ReadAllText($domFile)
}

$failed = $false
foreach ($case in $cases) {
  $url = Get-CaseUrl $case
  foreach ($size in $sizes) {
    $w = $size[0]; $h = $size[1]
    Invoke-Edge @($shotWait, "--screenshot=$(Join-Path $OutDir "$case-${w}x$h.png")") $w $h $url | Out-Null
    $dom = Invoke-Edge @($domWait, '--dump-dom') $w $h $url
    if ($dom -notmatch 'data-ready="1"') { $flag = 'unknown' }
    elseif ($dom -match 'data-overflow="yes"') { $flag = 'yes' }
    else { $flag = 'no' }
    if ($flag -ne 'no') { $failed = $true }
    Write-Output "$case ${w}x$h overflow=$flag"
  }
  if ($fullPage.ContainsKey($case)) {
    $fh = $fullPage[$case]
    Invoke-Edge @($shotWait, "--screenshot=$(Join-Path $OutDir "$case-900x$fh.png")") 900 $fh $url | Out-Null
  }
}

Write-Output "截圖在：$OutDir"
if ($failed) { exit 1 }
exit 0
