#!/usr/bin/env bash
# Smoke-test all ncnn backends using Loku's unified model paths.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/tools/upscale"
IN="/tmp/loku-test-in.png"
python3 -c "from PIL import Image; Image.new('RGB',(64,64),(128,64,200)).save('$IN')" 2>/dev/null || {
  # fallback: copy from esrgan zip sample if present
  cp "$ROOT/tools/.cache/esrgan-extract/input.jpg" "$IN" 2>/dev/null || exit 1
}

pass=0 fail=0 skip=0
GPU="${UPSCALE_GPU:-0}"

run() {
  local label="$1"; shift
  local out="/tmp/loku-smoke-${label}.png"
  rm -f "$out"
  if "$@" -i "$IN" -o "$out" -t 64 -g "$GPU" -j 1:1:1 -f png 2>/dev/null && [ -f "$out" ]; then
    echo "PASS $label ($(file -b "$out" | cut -d, -f1-2))"
    pass=$((pass+1))
  else
    echo "FAIL $label"
    fail=$((fail+1))
  fi
}

M="$DEST/models"
E="$DEST/realesrgan-ncnn-vulkan"
W="$DEST/waifu2x-ncnn-vulkan"
C="$DEST/realcugan-ncnn-vulkan"
R="$DEST/realsr-ncnn-vulkan"

echo "=== Real-ESRGAN ==="
for spec in "x4plus:realesrgan-x4plus:2" "x4plus-4x:realesrgan-x4plus:4" "x4-anime:realesrgan-x4plus-anime:4" "animev3:realesr-animevideov3:2" "x4net:realesrnet-x4plus:4"; do
  IFS=: read -r name model scale <<< "$spec"
  run "esrgan-$name" "$E" -n "$model" -s "$scale" -m "$M/realesrgan"
done

echo ""
echo "=== waifu2x ==="
for spec in "cunet:models-cunet:2" "anime:models-upconv_7_anime_style_art_rgb:2" "photo:models-upconv_7_photo:4"; do
  IFS=: read -r name dir scale <<< "$spec"
  run "waifu-$name" "$W" -s "$scale" -n 0 -m "$M/$dir"
done

echo ""
echo "=== Real-CUGAN ==="
run "cugan-se-2x" "$C" -s 2 -n -1 -m "$M/models-se"
run "cugan-se-4x" "$C" -s 4 -n 1 -m "$M/models-se"

echo ""
echo "=== RealSR ==="
run "realsr-df2k-4x" "$R" -s 4 -m "$M/models-DF2K"

echo ""
echo "=== Paths discovery (Rust) ==="
cd "$ROOT/ui" && cargo test paths::tests::unified_layout --quiet 2>&1 | tail -2

echo ""
echo "SUMMARY: pass=$pass fail=$fail skip=$skip"
