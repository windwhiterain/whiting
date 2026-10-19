"""Cut a species out of its frame as H1 asks: the identity mask as black on white.

Usage: python silhouette.py <authored_id.png> <out.png>

The authored_id png holds the id normalised into eight bits of red, so an id of one lands on a red
byte of one or two. **Any nonzero red is a pixel the plant drew**, and a threshold of four — which
this script used to apply — silently dropped the root's own id and therefore every trunk from every
cut-out on every sheet: the oak's "hole in its crown" in the ten-species sheet was its own bole,
missing. Measure the mask at `> 0` and a silhouette is a silhouette.
"""

import sys

import numpy as np
from PIL import Image


def main(argv: list[str]) -> int:
    rgb = np.asarray(Image.open(argv[0]).convert("RGB"), dtype=np.uint8)
    mask = rgb[..., 0] > 0
    out = np.where(mask, 0, 255).astype(np.uint8)
    Image.fromarray(out, mode="L").save(argv[1])
    print(f"{argv[1]}: plant covers {mask.mean():.3f} of the frame")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
