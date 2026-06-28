# Loku

A minimal desktop image upscaler for Windows. Drag images in, pick a model and
scale, hit run. Loku is a native [egui](https://github.com/emilk/egui) front-end
that drives [`realesrgan-ncnn-vulkan`](https://github.com/xinntao/Real-ESRGAN)
under the hood — batch queue, live per-image progress, and a before/after
preview.

## Requirements

- Windows 10/11 with a Vulkan-capable GPU
- [Rust](https://rustup.rs/) (MSVC toolchain)
- The `realesrgan-ncnn-vulkan` binary + models (not committed — see Setup)

## Setup

The NCNN binary and its models are not part of this repo. Download a release and
extract it so the layout looks like this:

```
upscale/
├─ tools/realesrgan-full/
│  ├─ realesrgan-ncnn-vulkan.exe
│  └─ models/            # *.param + *.bin for each model below
└─ ui/                   # this Rust app
```

1. Grab the latest Windows zip from the
   [Real-ESRGAN releases](https://github.com/xinntao/Real-ESRGAN/releases)
   (e.g. `realesrgan-ncnn-vulkan-*-windows.zip`).
2. Extract it into `tools/realesrgan-full/` so the `.exe` and `models/` folder
   sit at the paths shown above.

Loku finds the binary by walking up from the executable / working directory
looking for `tools/realesrgan-full`. If your layout differs, point it directly:

```powershell
$env:UPSCALE_ROOT = "C:\path\to\upscale"
```

## Build & run

```powershell
cd ui
cargo run --release
```

> `build.ps1` is a personal helper that pins a specific MSVC/SDK version before
> calling cargo. If your standard `cargo` build works, ignore it. Otherwise edit
> the paths at the top to match your install and run `.\build.ps1 run --release`.

## Usage

- **Drop** images (or a folder) onto the window, or click to open a file picker.
- Supported inputs: `jpg`, `jpeg`, `png`, `webp`.
- Pick a **model**, **scale** (2/3/4×), and **output format** (PNG/JPG/WEBP).
- Press **Run** (or `Enter`). Outputs are written next to each input as
  `<name>_upscaled.<ext>`.

| Model          | Best for                    |
| -------------- | --------------------------- |
| `animev3`      | anime / line art (video v3) |
| `x4plus`       | general photos              |
| `x4plus-anime` | anime stills                |

## Environment variables

| Variable       | Default | Purpose                                            |
| -------------- | ------- | -------------------------------------------------- |
| `UPSCALE_ROOT` | (auto)  | Override repo-root detection for the NCNN binary.  |
| `UPSCALE_GPU`  | `1`     | GPU device id passed to the backend (`-g`).        |

## License

MIT — see [LICENSE](LICENSE).
