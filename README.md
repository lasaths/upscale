# Loku

A minimal desktop image upscaler for Windows. Drag images in, pick an engine and
model, hit run. Loku is a native [egui](https://github.com/emilk/egui) front-end
that drives ncnn-vulkan upscalers under the hood — batch queue, live per-image
progress, and a before/after preview.

## Requirements

- Windows 10/11 with a Vulkan-capable GPU (macOS/Linux supported for development)
- [Rust](https://rustup.rs/) (MSVC toolchain on Windows)
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
│     └─ models-DF2K/
└─ ui/                            # this Rust app
```

On macOS/Linux the binaries have no `.exe` suffix. Model folder names must match
what each ncnn binary expects — the setup scripts handle this.

You do not need all four engines — Loku shows only installed backends. At least one
is required.

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

Loku finds the repo by walking up from the executable / working directory. Override
with:

```powershell
$env:UPSCALE_ROOT = "C:\path\to\upscale"
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
  Real-CUGAN), **TTA**, and **output format**.
- Press **Run** (or `Enter`). Outputs are written next to each input as
  `<name>_upscaled.<ext>`.

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

### TTA

Test-time augmentation (`-x`) improves quality at the cost of speed on all engines.

## Not included

Loku uses portable ncnn-vulkan CLIs only. These are **not** bundled or supported:

- SUPIR, FlowSR, ODTSR, VARestorer, VOSR, LinearSR (diffusion / PyTorch SOTA)
- Swin2SR, HAT, Artisan (PyTorch transformer upscalers)
- `realesr-general-x4v3` (requires a community-patched ncnn binary)

For those, use ComfyUI or the upstream Python projects.

## Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `UPSCALE_ROOT` | (auto) | Override repo-root detection. |
| `UPSCALE_GPU` | `1` (Windows), `0` (macOS) | GPU device id passed to backends (`-g`). |

## License

MIT — see [LICENSE](LICENSE).
