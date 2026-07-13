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
echo "Done. Installed:"
ls -1 "$DEST" | grep -v models
echo "Models:"
ls -1 "$DEST/models"
echo ""
echo "Run: cd ui && cargo run --release"
