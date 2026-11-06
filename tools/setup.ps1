# Download ncnn-vulkan upscalers into tools/upscale/ (Windows).
# Also mirrors into %LOCALAPPDATA%\loku so an installed loku.exe (cargo-dist) finds engines.
param(
    [string]$InstallRoot = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
if (-not $InstallRoot) { $InstallRoot = $RepoRoot }
$DataRoot = if ($env:LOKU_DATA) { $env:LOKU_DATA } else { Join-Path $env:LOCALAPPDATA "loku" }
$Dest = Join-Path $InstallRoot "tools\upscale"
$Cache = Join-Path $RepoRoot "tools\.cache"
New-Item -ItemType Directory -Force -Path $Dest, (Join-Path $Dest "models"), $Cache | Out-Null

function Fetch($Url, $Out) {
    if (Test-Path $Out) { Write-Host "  cached $(Split-Path $Out -Leaf)"; return }
    Write-Host "  downloading $(Split-Path $Out -Leaf)..."
    Invoke-WebRequest -Uri $Url -OutFile $Out -UseBasicParsing
}

function Find-Binary($Dir, $Name) {
    Get-ChildItem -Path $Dir -Recurse -File -Filter $Name | Select-Object -First 1
}

function Extract($Zip, $Dir) {
    if (Test-Path $Dir) { Remove-Item -Recurse -Force $Dir }
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    Expand-Archive -Path $Zip -DestinationPath $Dir -Force
}

function Copy-Exe($Src, $Name) {
    Copy-Item $Src (Join-Path $Dest $Name) -Force
}

function Copy-ModelsDir($Src, $Name) {
    $Target = Join-Path $Dest "models\$Name"
    if (Test-Path $Target) { Remove-Item -Recurse -Force $Target }
    Copy-Item $Src $Target -Recurse -Force
}

Write-Host "==> Loku ncnn setup (windows) -> $Dest"

# Real-ESRGAN
$EsrganZip = Join-Path $Cache "realesrgan-ncnn-vulkan-20220424-windows.zip"
Fetch "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-windows.zip" $EsrganZip
$EsrganEx = Join-Path $Cache "esrgan-extract"
Extract $EsrganZip $EsrganEx
$EsrganBin = Find-Binary $EsrganEx "realesrgan-ncnn-vulkan.exe"
$EsrganDir = $EsrganBin.DirectoryName
Copy-Exe $EsrganBin.FullName "realesrgan-ncnn-vulkan.exe"
Copy-ModelsDir (Join-Path $EsrganDir "models") "realesrgan"

$EsrganOldZip = Join-Path $Cache "realesrgan-ncnn-vulkan-20211212-windows.zip"
Fetch "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.3.0/realesrgan-ncnn-vulkan-20211212-windows.zip" $EsrganOldZip
$EsrganOldEx = Join-Path $Cache "esrgan-old-extract"
Extract $EsrganOldZip $EsrganOldEx
$EsrganOldBin = Find-Binary $EsrganOldEx "realesrgan-ncnn-vulkan.exe"
$EsrganOldDir = $EsrganOldBin.DirectoryName
foreach ($f in @("realesrnet-x4plus.bin", "realesrnet-x4plus.param")) {
    $src = Join-Path $EsrganOldDir "models\$f"
    if (Test-Path $src) { Copy-Item $src (Join-Path $Dest "models\realesrgan\$f") -Force }
}

# waifu2x
$WaifuZip = Join-Path $Cache "waifu2x-ncnn-vulkan-20250915-windows.zip"
Fetch "https://github.com/nihui/waifu2x-ncnn-vulkan/releases/download/20250915/waifu2x-ncnn-vulkan-20250915-windows.zip" $WaifuZip
$WaifuEx = Join-Path $Cache "waifu-extract"
Extract $WaifuZip $WaifuEx
$WaifuBin = Find-Binary $WaifuEx "waifu2x-ncnn-vulkan.exe"
$WaifuDir = $WaifuBin.DirectoryName
Copy-Exe $WaifuBin.FullName "waifu2x-ncnn-vulkan.exe"
Copy-ModelsDir (Join-Path $WaifuDir "models-cunet") "models-cunet"
Copy-ModelsDir (Join-Path $WaifuDir "models-upconv_7_anime_style_art_rgb") "models-upconv_7_anime_style_art_rgb"
Copy-ModelsDir (Join-Path $WaifuDir "models-upconv_7_photo") "models-upconv_7_photo"

# Real-CUGAN
$CuganZip = Join-Path $Cache "realcugan-ncnn-vulkan-20220728-windows.zip"
Fetch "https://github.com/nihui/realcugan-ncnn-vulkan/releases/download/20220728/realcugan-ncnn-vulkan-20220728-windows.zip" $CuganZip
$CuganEx = Join-Path $Cache "cugan-extract"
Extract $CuganZip $CuganEx
$CuganBin = Find-Binary $CuganEx "realcugan-ncnn-vulkan.exe"
$CuganDir = $CuganBin.DirectoryName
Copy-Exe $CuganBin.FullName "realcugan-ncnn-vulkan.exe"
Copy-ModelsDir (Join-Path $CuganDir "models-se") "models-se"

# RealSR
$RealsrZip = Join-Path $Cache "realsr-ncnn-vulkan-20220728-windows.zip"
Fetch "https://github.com/nihui/realsr-ncnn-vulkan/releases/download/20220728/realsr-ncnn-vulkan-20220728-windows.zip" $RealsrZip
$RealsrEx = Join-Path $Cache "realsr-extract"
Extract $RealsrZip $RealsrEx
$RealsrBin = Find-Binary $RealsrEx "realsr-ncnn-vulkan.exe"
$RealsrDir = $RealsrBin.DirectoryName
Copy-Exe $RealsrBin.FullName "realsr-ncnn-vulkan.exe"
if (Test-Path (Join-Path $RealsrDir "models-DF2K")) {
    Copy-ModelsDir (Join-Path $RealsrDir "models-DF2K") "models-DF2K"
} else {
    Copy-ModelsDir (Join-Path $RealsrDir "models-DF2K_JPEG") "models-DF2K"
}

Write-Host ""
Write-Host "==> ONNX (UltraSharp V2 + PurePhoto, optional)"
$OnnxDir = Join-Path $Dest "models\onnx"
New-Item -ItemType Directory -Force -Path $OnnxDir | Out-Null
# Drop the old Real-ESRGAN ONNX default (ncnn covers it).
Remove-Item (Join-Path $OnnxDir "real_esrgan_x4plus.onnx") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $OnnxDir "real_esrgan_x4plus.data") -Force -ErrorAction SilentlyContinue

function Install-OnnxModel([string]$DestName, [string]$Url) {
    $cached = Join-Path $Cache $DestName
    Fetch $Url $cached
    Copy-Item $cached (Join-Path $OnnxDir $DestName) -Force
    Write-Host "  installed $DestName"
}

Install-OnnxModel "4x-UltraSharpV2.onnx" `
    "https://huggingface.co/huggingworld/onnx-image-models/resolve/main/4x-UltraSharpV2_fp32_op17.onnx"
Install-OnnxModel "4xPurePhoto-RealPLSKR.onnx" `
    "https://huggingface.co/huggingworld/onnx-image-models/resolve/main/4xPurePhoto-RealPLSKR.onnx"

Write-Host ""
Write-Host "==> Suggest classifiers (deepghs cascade, optional)"
$SuggestDir = Join-Path $Dest "models\suggest"
New-Item -ItemType Directory -Force -Path $SuggestDir | Out-Null
$Legacy = Join-Path $SuggestDir "medium_classify.onnx"
if (Test-Path $Legacy) { Remove-Item $Legacy -Force }

function Install-SuggestModel([string]$Name, [string]$Url) {
    $cached = Join-Path $Cache $Name
    if (-not (Test-Path $cached)) {
        Write-Host "  downloading $Name..."
        Invoke-WebRequest -Uri $Url -OutFile $cached -UseBasicParsing
    } else {
        Write-Host "  cached $Name"
    }
    Copy-Item $cached (Join-Path $SuggestDir $Name) -Force
    Write-Host "  installed $Name"
}

try {
    Install-SuggestModel "anime_real.onnx" `
        "https://huggingface.co/deepghs/anime_real_cls/resolve/main/mobilenetv3_v1.4_dist/model.onnx"
    Install-SuggestModel "anime_cls.onnx" `
        "https://huggingface.co/deepghs/anime_classification/resolve/main/mobilenetv3_v1.5_dist/model.onnx"
} catch {
    Write-Host "  [warn] suggest classifier download failed - SUGGEST button will be disabled"
}

# Mirror into the user data root so PATH-installed loku finds engines.
if ((Resolve-Path $InstallRoot).Path -ne (Resolve-Path (New-Item -ItemType Directory -Force -Path $DataRoot)).Path) {
    $DataDest = Join-Path $DataRoot "tools\upscale"
    Write-Host ""
    Write-Host "==> Mirror engines -> $DataDest"
    if (Test-Path $DataDest) { Remove-Item -Recurse -Force $DataDest }
    New-Item -ItemType Directory -Force -Path (Split-Path $DataDest) | Out-Null
    Copy-Item $Dest $DataDest -Recurse -Force
    Write-Host "  mirrored"
}

Write-Host ""
Write-Host "Done. Installed:"
Get-ChildItem $Dest -File | ForEach-Object { $_.Name }
Write-Host "Models:"
Get-ChildItem (Join-Path $Dest "models") -Directory | ForEach-Object { $_.Name }
Write-Host ""
Write-Host "Data root: $DataRoot"
Write-Host "Run: cd ui; cargo run --release"
