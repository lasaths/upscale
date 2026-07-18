#!/usr/bin/env python3
"""One-time / maintainers: export Mitchins EfficientNet-B0 → medium_classify.onnx.

Requires: torch, timm, safetensors, onnx
  uv venv && uv pip install torch timm safetensors onnx

Source: https://huggingface.co/Mitchins/image-medium-classifier-efficientnet-b0-v1
"""
from __future__ import annotations

import urllib.request
from pathlib import Path

import onnx
import timm
import torch
from safetensors.torch import load_file

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tools/upscale/models/suggest/medium_classify.onnx"
CACHE = ROOT / "tools/.cache/suggest-export"
WEIGHTS_URL = (
    "https://huggingface.co/Mitchins/image-medium-classifier-efficientnet-b0-v1"
    "/resolve/main/model.safetensors"
)


def main() -> None:
    CACHE.mkdir(parents=True, exist_ok=True)
    weights = CACHE / "model.safetensors"
    if not weights.exists():
        print(f"downloading {WEIGHTS_URL}")
        urllib.request.urlretrieve(WEIGHTS_URL, weights)

    model = timm.create_model("efficientnet_b0", num_classes=3, pretrained=False)
    model.load_state_dict(load_file(str(weights)))
    model.eval()

    OUT.parent.mkdir(parents=True, exist_ok=True)
    dummy = torch.randn(1, 3, 224, 224)
    torch.onnx.export(
        model,
        dummy,
        str(OUT),
        input_names=["input"],
        output_names=["logits"],
        opset_version=17,
        dynamo=False,
    )
    onnx.checker.check_model(onnx.load(str(OUT)))
    print(f"wrote {OUT} ({OUT.stat().st_size / 1e6:.1f} MB)")


if __name__ == "__main__":
    main()
