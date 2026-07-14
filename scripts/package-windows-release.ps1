param(
  [string]$TargetRoot = "src-tauri/target/x86_64-pc-windows-msvc/release"
)

$ErrorActionPreference = "Stop"

$config = Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json
$version = $config.version
$productName = $config.productName
$releaseRoot = Join-Path (Get-Location) "Release"
$releaseName = "$productName-$version-windows-x64"
$releaseDirectory = Join-Path $releaseRoot $releaseName

New-Item -ItemType Directory -Force -Path $releaseDirectory | Out-Null

$msi = Get-ChildItem (Join-Path $TargetRoot "bundle/msi/*.msi") | Select-Object -First 1
$setup = Get-ChildItem (Join-Path $TargetRoot "bundle/nsis/*-setup.exe") | Select-Object -First 1
$executable = Get-Item (Join-Path $TargetRoot "shanji.exe")

if (-not $msi) { throw "未找到 Windows MSI 产物" }
if (-not $setup) { throw "未找到 Windows NSIS Setup.exe 产物" }
if (-not $executable) { throw "未找到 Windows 独立 exe 产物" }

$msiDestination = Join-Path $releaseDirectory "$productName-$version-x64.msi"
$setupDestination = Join-Path $releaseDirectory "$productName-$version-x64-Setup.exe"
$executableDestination = Join-Path $releaseDirectory "$productName-$version-x64.exe"

Copy-Item -LiteralPath $msi.FullName -Destination $msiDestination -Force
Copy-Item -LiteralPath $setup.FullName -Destination $setupDestination -Force
Copy-Item -LiteralPath $executable.FullName -Destination $executableDestination -Force

$instructions = @"
$productName $version（Windows 10/11 x64）

推荐安装：双击 $productName-$version-x64-Setup.exe。
安装器包含 WebView2 离线运行时，不需要安装 Node.js、Python、Rust 或其他开发环境。
安装后会创建桌面快捷方式和开始菜单入口。

备用安装：$productName-$version-x64.msi。
独立程序：$productName-$version-x64.exe，数据仍保存在当前用户 AppData；它不是数据随程序移动的完全便携版。

当前版本未进行商业代码签名，Windows SmartScreen 可能显示未知发布者提示。
"@
Set-Content -LiteralPath (Join-Path $releaseDirectory "使用说明.txt") -Value $instructions -Encoding utf8

$checksumFiles = Get-ChildItem $releaseDirectory -File
$checksums = $checksumFiles | ForEach-Object {
  $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
  "$hash  $($_.Name)"
}
Set-Content -LiteralPath (Join-Path $releaseDirectory "SHA256SUMS.txt") -Value $checksums -Encoding utf8

$completeZip = Join-Path $releaseRoot "$releaseName-Release.zip"
Compress-Archive -Path $releaseDirectory -DestinationPath $completeZip -CompressionLevel Optimal -Force
$completeZipHash = (Get-FileHash $completeZip -Algorithm SHA256).Hash.ToLower()
Set-Content -LiteralPath "$completeZip.sha256" -Value "$completeZipHash  $([System.IO.Path]::GetFileName($completeZip))" -Encoding utf8

Write-Host "Windows 发布目录已生成：$releaseDirectory"
Write-Host "完整发布包已生成：$completeZip"
