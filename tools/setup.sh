#!/usr/bin/env bash
# Download ncnn-vulkan upscalers into tools/upscale/ (macOS / Linux).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/tools/upscale"
CACHE="$ROOT/tools/.cache"
mkdir -p "$CACHE" "$DEST/models"

# Remove legacy renamed model dirs from older setup runs.
rm -rf "$DEST/models/waifu2x-cunet" "$DEST/models/waifu2x-upconv-anime" \
       "$DEST/models/waifu2x-upconv-photo" "$DEST/models/realcugan-se" \
       "$DEST/models/realsr-df2k"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
  darwin) PLATFORM=macos ;;
  linux)  PLATFORM=ubuntu ;;
  *)
    echo "Unsupported OS: $OS (use setup.ps1 on Windows)"
    exit 1
    ;;
esac

fetch() {
  local url="$1" out="$2"
  if [[ -f "$out" ]]; then
    echo "  cached $(basename "$out")"
  else
    echo "  downloading $(basename "$out")..."
    curl -fsSL --retry 3 -o "$out" "$url"
  fi
}

extract_zip() {
  local zip="$1" dir="$2"
  rm -rf "$dir"
  mkdir -p "$dir"
  unzip -q -o "$zip" -d "$dir"
}

find_binary() {
  local dir="$1" name="$2"
  find "$dir" -name "$name" -type f | head -1
}

copy_exe() {
  local src="$1" name="$2"
  cp "$src" "$DEST/$name"
  chmod +x "$DEST/$name"
}

copy_models_dir() {
  local src="$1" name="$2"
  rm -rf "$DEST/models/$name"
  cp -a "$src" "$DEST/models/$name"
}

echo "==> Loku ncnn setup ($PLATFORM) -> $DEST"

# --- Real-ESRGAN (v0.2.5.0) ---
ESRGAN_ZIP="$CACHE/realesrgan-ncnn-vulkan-20220424-${PLATFORM}.zip"
fetch "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-${PLATFORM}.zip" "$ESRGAN_ZIP"
ESRGAN_EX="$CACHE/esrgan-extract"
extract_zip "$ESRGAN_ZIP" "$ESRGAN_EX"
ESRGAN_BIN="$(find_binary "$ESRGAN_EX" "realesrgan-ncnn-vulkan")"
ESRGAN_DIR="$(dirname "$ESRGAN_BIN")"
copy_exe "$ESRGAN_BIN" "realesrgan-ncnn-vulkan"
copy_models_dir "$ESRGAN_DIR/models" "realesrgan"

# realesrnet-x4plus from older ncnn pack (not in v0.2.5.0 models/)
ESRGAN_OLD_ZIP="$CACHE/realesrgan-ncnn-vulkan-20211212-${PLATFORM}.zip"
fetch "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.3.0/realesrgan-ncnn-vulkan-20211212-${PLATFORM}.zip" "$ESRGAN_OLD_ZIP"
ESRGAN_OLD_EX="$CACHE/esrgan-old-extract"
extract_zip "$ESRGAN_OLD_ZIP" "$ESRGAN_OLD_EX"
ESRGAN_OLD_BIN="$(find_binary "$ESRGAN_OLD_EX" "realesrgan-ncnn-vulkan")"
ESRGAN_OLD_DIR="$(dirname "$ESRGAN_OLD_BIN")"
for f in realesrnet-x4plus.bin realesrnet-x4plus.param; do
  if [[ -f "$ESRGAN_OLD_DIR/models/$f" ]]; then
    cp "$ESRGAN_OLD_DIR/models/$f" "$DEST/models/realesrgan/"
  fi
done

# --- waifu2x ---
WAIFU_ZIP="$CACHE/waifu2x-ncnn-vulkan-20250915-${PLATFORM}.zip"
fetch "https://github.com/nihui/waifu2x-ncnn-vulkan/releases/download/20250915/waifu2x-ncnn-vulkan-20250915-${PLATFORM}.zip" "$WAIFU_ZIP"
WAIFU_EX="$CACHE/waifu-extract"
extract_zip "$WAIFU_ZIP" "$WAIFU_EX"
WAIFU_BIN="$(find_binary "$WAIFU_EX" "waifu2x-ncnn-vulkan")"
WAIFU_DIR="$(dirname "$WAIFU_BIN")"
copy_exe "$WAIFU_BIN" "waifu2x-ncnn-vulkan"
copy_models_dir "$WAIFU_DIR/models-cunet" "models-cunet"
copy_models_dir "$WAIFU_DIR/models-upconv_7_anime_style_art_rgb" "models-upconv_7_anime_style_art_rgb"
copy_models_dir "$WAIFU_DIR/models-upconv_7_photo" "models-upconv_7_photo"

# --- Real-CUGAN ---
CUGAN_ZIP="$CACHE/realcugan-ncnn-vulkan-20220728-${PLATFORM}.zip"
fetch "https://github.com/nihui/realcugan-ncnn-vulkan/releases/download/20220728/realcugan-ncnn-vulkan-20220728-${PLATFORM}.zip" "$CUGAN_ZIP"
CUGAN_EX="$CACHE/cugan-extract"
extract_zip "$CUGAN_ZIP" "$CUGAN_EX"
CUGAN_BIN="$(find_binary "$CUGAN_EX" "realcugan-ncnn-vulkan")"
CUGAN_DIR="$(dirname "$CUGAN_BIN")"
copy_exe "$CUGAN_BIN" "realcugan-ncnn-vulkan"
copy_models_dir "$CUGAN_DIR/models-se" "models-se"

# --- RealSR ---
REALSR_ZIP="$CACHE/realsr-ncnn-vulkan-20220728-${PLATFORM}.zip"
fetch "https://github.com/nihui/realsr-ncnn-vulkan/releases/download/20220728/realsr-ncnn-vulkan-20220728-${PLATFORM}.zip" "$REALSR_ZIP"
REALSR_EX="$CACHE/realsr-extract"
extract_zip "$REALSR_ZIP" "$REALSR_EX"
REALSR_BIN="$(find_binary "$REALSR_EX" "realsr-ncnn-vulkan")"
REALSR_DIR="$(dirname "$REALSR_BIN")"
copy_exe "$REALSR_BIN" "realsr-ncnn-vulkan"
if [[ -d "$REALSR_DIR/models-DF2K" ]]; then
  copy_models_dir "$REALSR_DIR/models-DF2K" "models-DF2K"
else
  copy_models_dir "$REALSR_DIR/models-DF2K_JPEG" "models-DF2K"
fi

echo ""
echo "==> ONNX (Real-ESRGAN x4plus, optional)"
ONNX_DIR="$DEST/models/onnx"
mkdir -p "$ONNX_DIR"
ONNX_ZIP="$CACHE/real_esrgan_x4plus-onnx-float.zip"
fetch "https://qaihub-public-assets.s3.us-west-2.amazonaws.com/qai-hub-models/models/real_esrgan_x4plus/releases/v0.50.2/real_esrgan_x4plus-onnx-float.zip" "$ONNX_ZIP"
ONNX_EX="$CACHE/onnx-extract"
extract_zip "$ONNX_ZIP" "$ONNX_EX"
ONNX_SRC="$(find "$ONNX_EX" -name 'real_esrgan_x4plus.onnx' -type f | head -1)"
if [[ -n "$ONNX_SRC" ]]; then
  ONNX_SRC_DIR="$(dirname "$ONNX_SRC")"
  cp "$ONNX_SRC" "$ONNX_DIR/real_esrgan_x4plus.onnx"
  if [[ -f "$ONNX_SRC_DIR/real_esrgan_x4plus.data" ]]; then
    cp "$ONNX_SRC_DIR/real_esrgan_x4plus.data" "$ONNX_DIR/real_esrgan_x4plus.data"
  fi
  echo "  installed real_esrgan_x4plus.onnx (+ external weights if present)"
fi

echo ""
echo "==> Suggest classifiers (deepghs cascade, optional)"
SUGGEST_DIR="$DEST/models/suggest"
mkdir -p "$SUGGEST_DIR"
rm -f "$SUGGEST_DIR/medium_classify.onnx"
# anime_real → anime_cls cascade (OpenRAIL ONNX from deepghs).
download_suggest() {
  local name="$1" url="$2"
  local cached="$CACHE/$name"
  if [[ -f "$cached" ]]; then
    echo "  cached $name"
  elif curl -fsSL --retry 3 -o "$cached" "$url"; then
    echo "  downloaded $name"
  else
    rm -f "$cached"
    return 1
  fi
  cp "$cached" "$SUGGEST_DIR/$name"
  echo "  installed $name"
}

SUGGEST_OK=1
download_suggest "anime_real.onnx" \
  "https://huggingface.co/deepghs/anime_real_cls/resolve/main/mobilenetv3_v1.4_dist/model.onnx" \
  || SUGGEST_OK=0
download_suggest "anime_cls.onnx" \
  "https://huggingface.co/deepghs/anime_classification/resolve/main/mobilenetv3_v1.5_dist/model.onnx" \
  || SUGGEST_OK=0
if [[ "$SUGGEST_OK" -ne 1 ]]; then
  echo "  [warn] suggest classifier download failed — SUGGEST button will be disabled"
fi

echo ""
echo "Done. Installed:"
ls -1 "$DEST" | grep -v models
echo "Models:"
ls -1 "$DEST/models"
echo ""
echo "Run: cd ui && cargo run --release"
