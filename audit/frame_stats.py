"""Read a rendered frame the way the bar reads it: luminance percentiles of a crown, and the
silhouette a whole-tree frame draws.

Usage: python frame_stats.py <frame.png> [<authored_id.png>] [--box x0 y0 x1 y1]

With a box, the percentiles are taken inside it: that is H3's crown-depth reading, the spread of
lit outside against dark inside. With an authored_id image, the fraction of the box whose id is
nonzero is the silhouette's coverage of it, which is H1's raw material.
"""

import sys

import numpy as np
from PIL import Image


def luminance(rgb: np.ndarray) -> np.ndarray:
    return 0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2]


def main(argv: list[str]) -> int:
    frame = np.asarray(Image.open(argv[0]).convert("RGB"), dtype=np.float64) / 255.0
    box = None
    if "--box" in argv:
        at = argv.index("--box")
        box = tuple(int(part) for part in argv[at + 1 : at + 5])
    region = frame if box is None else frame[box[1] : box[3], box[0] : box[2]]
    lum = luminance(region)
    print(f"frame {argv[0]} {frame.shape[1]}x{frame.shape[0]}"
          + (f" box {box}" if box else ""))
    for percent in (1, 10, 25, 50, 75, 90, 99):
        print(f"  p{percent:<2} = {np.percentile(lum, percent):.3f}")
    spread = np.percentile(lum, 90) / max(np.percentile(lum, 10), 1e-6)
    print(f"  p90/p10 = {spread:.2f}")
    if len(argv) > 1 and not argv[1].startswith("--"):
        ids = np.asarray(Image.open(argv[1]).convert("RGB"), dtype=np.float64)
        ids = ids[..., 0] * 255.0 * 255.0 + ids[..., 1] * 255.0 + ids[..., 2]
        region_ids = ids if box is None else ids[box[1] : box[3], box[0] : box[2]]
        plant = region_ids > 0.5
        covered = float(plant.mean())
        print(f"  silhouette coverage of the region = {covered:.3f}")
        # H3: the spread of the plant's own pixels, lit outside against self-shadowed inside.
        # Sky in the box says nothing about crown depth, so it is excluded rather than averaged in.
        if covered > 0.0:
            own = lum[plant]
            lo, hi = np.percentile(own, 10), np.percentile(own, 90)
            print(f"  plant p10 = {lo:.3f}  p50 = {np.percentile(own, 50):.3f}"
                  f"  p90 = {hi:.3f}  p90/p10 = {hi / max(lo, 1e-6):.2f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
