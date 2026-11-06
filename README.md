# Loku

A minimal desktop image upscaler. Drag images in, pick an engine and model, hit
run. Loku is a native [egui](https://github.com/emilk/egui) front-end that drives
ncnn-vulkan upscalers under the hood — batch queue, live per-image progress, and
a before/after preview.

## Install

**macOS / Linux (Homebrew):**
```bash
brew install lasaths/tap/loku
```

**macOS / Linux (script):**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/lasaths/upscale/releases/latest/download/upscale-ui-installer.sh | sh
```

**Windows (recommended — binary + engines + shortcuts):**
```powershell
irm https://raw.githubusercontent.com/lasaths/upscale/master/tools/install-windows.ps1 | iex
```

**Windows (binary only):**
```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/lasaths/upscale/releases/latest/download/upscale-ui-installer.ps1 | iex"
```
Then run setup once so engines land under `%LOCALAPPDATA%\loku` (see Setup).

**Uninstall (Windows):**
```powershell
& "$env:LOCALAPPDATA\loku\uninstall.ps1"
# or from a clone:
.\tools\install-windows.ps1 -Uninstall
```

Or grab zips from [Releases](https://github.com/lasaths/upscale/releases).

> winget: not in the official catalog yet — use the PowerShell installer above for now.

## Requirements

- Windows 10/11 or macOS with a Vulkan-capable GPU (Linux ok for development)
- [Rust](https://rustup.rs/) only if building from source (MSVC toolchain on Windows)
- ncnn-vulkan binaries + models (not committed — see Setup)

## Setup

Run the setup script for your platform — it downloads all four ncnn-vulkan engines
and arranges them under `tools/upscale/`:

```powershell
# Windows (PowerShell)
.\tools\setup.ps1
```

```bash
# macOS / Linux
chmod +x tools/setup.sh
./tools/setup.sh
```

The ncnn binaries and models are not committed to git. After setup:

```
upscale/
├─ tools/upscale/
│  ├─ realesrgan-ncnn-vulkan.exe
│  ├─ waifu2x-ncnn-vulkan.exe
│  ├─ realcugan-ncnn-vulkan.exe
│  ├─ realsr-ncnn-vulkan.exe
│  └─ models/
│     ├─ realesrgan/
│     ├─ models-cunet/
│     ├─ models-upconv_7_anime_style_art_rgb/
│     ├─ models-upconv_7_photo/
│     ├─ models-se/
│     ├─ models-DF2K/
│     └─ onnx/                    # optional ONNX models (.onnx)
└─ ui/                            # this Rust app
```

On macOS/Linux the binaries have no `.exe` suffix. Model folder names must match
what each ncnn binary expects — the setup scripts handle this.

You do not need all four ncnn engines — Loku shows only installed backends. At least one
ncnn engine **or** one ONNX model is required.

Verify with `./tools/smoke-test.sh` (macOS/Linux) after setup.

### Manual download (optional)

| Engine | Release |
|--------|---------|
| Real-ESRGAN | [xinntao/Real-ESRGAN releases](https://github.com/xinntao/Real-ESRGAN/releases) — `realesrgan-ncnn-vulkan-*-windows.zip` (v0.2.5.0) |
| waifu2x | [nihui/waifu2x-ncnn-vulkan releases](https://github.com/nihui/waifu2x-ncnn-vulkan/releases) |
| Real-CUGAN | [nihui/realcugan-ncnn-vulkan releases](https://github.com/nihui/realcugan-ncnn-vulkan/releases) |
| RealSR | [nihui/realsr-ncnn-vulkan releases](https://github.com/nihui/realsr-ncnn-vulkan/releases) |

For `realesrnet-x4plus`, also grab model files from the v0.2.3.0 ncnn zip.

### Legacy layout

If `tools/upscale/` is missing, Loku falls back to `tools/realesrgan-full/` with
the Real-ESRGAN binary and `models/` at that level (Real-ESRGAN only).

Loku looks for engines in this order: `UPSCALE_ROOT`, a parent of the exe /
cwd that contains `tools/upscale`, then the user data root
(`%LOCALAPPDATA%\loku` on Windows, `~/Library/Application Support/loku` on
macOS, `~/.local/share/loku` on Linux). Setup mirrors engines into that data
root so a PATH-installed `loku` works without the git checkout.

```powershell
$env:UPSCALE_ROOT = "C:\path\to\upscale"   # optional override
$env:LOKU_DATA = "D:\loku-data"            # optional data-root override
```

## Build & run

```powershell
cd ui
cargo run --release
```

> `build.ps1` is a personal helper that pins a specific MSVC/SDK version before
> calling cargo. If your standard `cargo` build works, ignore it.

## Usage

- **Drop** images (or a folder) onto the window, or click to open a file picker.
- Supported inputs: `jpg`, `jpeg`, `png`, `webp`.
- Pick an **engine**, **model**, **scale**, optional **denoise** (waifu2x /
  Real-CUGAN), **TTA**, and **output format** — or click **Suggest** to classify
  the preview image (anime vs photo) and apply a preset.
- Press **Run** (or `Enter`). Outputs are written next to each input as
  `<name>_upscaled.<ext>`.

### Suggest

Uses a deepghs ONNX cascade (OpenRAIL) downloaded by setup into
`models/suggest/`:
1. [`anime_real_cls`](https://huggingface.co/deepghs/anime_real_cls) — anime vs real
2. [`anime_classification`](https://huggingface.co/deepghs/anime_classification) — if anime, subtype (`illustration` / `bangumi` / `comic` → anime preset; `3d` / `not_painting` → photo preset)

Classifies the **previewed** queue item only; does not start an upscale.

### Real-ESRGAN models

| Model | Best for |
|-------|----------|
| `animev3` | anime / line art (video v3) |
| `x4plus` | general photos |
| `x4-anime` | anime stills |
| `x4net` | smoother output, fewer GAN artifacts |

Scales: 2×, 3×, 4×.

### Real-CUGAN

| Model | Best for |
|-------|----------|
| `se` | anime / illustrations (Bilibili Real-CUGAN) |

Scales: 2×, 3×, 4×. Denoise: -1 (off) through 3.

### waifu2x

| Model | Best for |
|-------|----------|
| `cunet` | general anime (best quality) |
| `anime` | anime style art RGB |
| `photo` | photos |

Scales: 2×, 4×. Denoise: -1 through 3.

### RealSR

| Model | Best for |
|-------|----------|
| `df2k` | real-world photos (DF2K) |

Scale: 4× only.

### ONNX (ONNX Runtime)

Higher-quality models via the embedded `ort` crate. Uses **CoreML** on macOS and
**DirectML** on Windows; CPU fallback elsewhere.

| Model | Best for |
|-------|----------|
| `4x-UltraSharpV2` | General default — photos and mixed content |
| `4xPurePhoto-RealPLSKR` | Photos / portraits |
| Any other `.onnx` in `models/onnx/` | Depends on model (HAT, DAT, SwinIR, etc.) |

Setup downloads UltraSharp V2 and PurePhoto by default. Add more from
[OpenModelDB](https://openmodeldb.io/) or
[huggingworld/onnx-image-models](https://huggingface.co/huggingworld/onnx-image-models) —
use **FP32** exports for transformer models (HAT, DAT). Keep external `.data`
files next to their `.onnx` (same basename). Models with external weights use
CPU on macOS/Windows; single-file models use CoreML/DirectML. Tile size is read
from the model (typically 128×128). Scale is detected automatically (typically 4×).

### TTA

Test-time augmentation (`-x`) improves quality at the cost of speed on ncnn engines.
Not available for ONNX.

## Not included

Loku uses portable ncnn-vulkan CLIs and optional ONNX Runtime inference. These are **not** bundled or supported:

- SUPIR, FlowSR, ODTSR, VARestorer, VOSR, LinearSR (diffusion / PyTorch SOTA)
- PyTorch-only Swin2SR / Artisan workflows (ONNX SwinIR exports are supported)
- `realesr-general-x4v3` (requires a community-patched ncnn binary)

For those, use ComfyUI or the upstream Python projects.

## Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `UPSCALE_ROOT` | (auto) | Override root detection (must contain `tools/upscale`). |
| `LOKU_DATA` | `%LOCALAPPDATA%\loku` (Windows) | User data root for engines when installed via cargo-dist. |
| `UPSCALE_GPU` | ncnn: `1` (Windows) / `0` (macOS); ONNX: auto | ncnn Vulkan device id (`-g`). For ONNX/DirectML, leave unset to auto-pick the high-performance GPU; set to force a DXGI adapter index. |

## License

MIT — see [LICENSE](LICENSE).
