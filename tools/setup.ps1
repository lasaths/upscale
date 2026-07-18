# Download ncnn-vulkan upscalers into tools/upscale/ (Windows).
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Dest = Join-Path $Root "tools\upscale"
$Cache = Join-Path $Root "tools\.cache"
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
Write-Host "==> ONNX (Real-ESRGAN x4plus, optional)"
$OnnxDir = Join-Path $Dest "models\onnx"
New-Item -ItemType Directory -Force -Path $OnnxDir | Out-Null
$OnnxZip = Join-Path $Cache "real_esrgan_x4plus-onnx-float.zip"
Fetch "https://qaihub-public-assets.s3.us-west-2.amazonaws.com/qai-hub-models/models/real_esrgan_x4plus/releases/v0.50.2/real_esrgan_x4plus-onnx-float.zip" $OnnxZip
$OnnxEx = Join-Path $Cache "onnx-extract"
Extract $OnnxZip $OnnxEx
$OnnxFile = Get-ChildItem -Path $OnnxEx -Recurse -Filter "real_esrgan_x4plus.onnx" | Select-Object -First 1
if ($OnnxFile) {
    Copy-Item $OnnxFile.FullName (Join-Path $OnnxDir "real_esrgan_x4plus.onnx") -Force
    $DataFile = Join-Path $OnnxFile.DirectoryName "real_esrgan_x4plus.data"
    if (Test-Path $DataFile) {
        Copy-Item $DataFile (Join-Path $OnnxDir "real_esrgan_x4plus.data") -Force
    }
    Write-Host "  installed real_esrgan_x4plus.onnx (+ external weights if present)"
}

Write-Host ""
Write-Host "Done. Installed:"
Get-ChildItem $Dest -File | ForEach-Object { $_.Name }
Write-Host "Models:"
Get-ChildItem (Join-Path $Dest "models") -Directory | ForEach-Object { $_.Name }
Write-Host ""
Write-Host "Run: cd ui; cargo run --release"
