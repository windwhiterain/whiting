"""Cut a species out of its frame as H1 asks: the identity mask as black on white.

Usage: python silhouette.py <authored_id.png> <out.png>

The authored_id png holds the id normalised into eight bits of red, so a pixel the plant drew is a
red byte above a small threshold; ids one and two round to nought and are lost, which costs the
trunk's own outline nothing at the crown and a little at the very base of the bole.
"""

import sys

import numpy as np
from PIL import Image


def main(argv: list[str]) -> int:
    rgb = np.asarray(Image.open(argv[0]).convert("RGB"), dtype=np.uint8)
    mask = rgb[..., 0] > 4
    out = np.where(mask, 0, 255).astype(np.uint8)
    Image.fromarray(out, mode="L").save(argv[1])
    print(f"{argv[1]}: plant covers {mask.mean():.3f} of the frame")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
