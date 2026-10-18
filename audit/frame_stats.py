"""Read a rendered frame the way the bar reads it.

Usage:
    python frame_stats.py <frame.png> <authored_id.png> --crown
    python frame_stats.py <frame.png> [--box x0 y0 x1 y1]

`--crown` derives the crown's box from the identity mask instead of a hand-picked rectangle: the
bounding box of every pixel whose authored_id is nonzero, cropped to the upper part of the plant so
that a trunk-only column does not count as crown. Inside that box it reports

  coverage   the fraction of the box the plant draws, which is H1's raw material: a silhouette that
             is the right species is a mask that is the right shape, and coverage says how much of
             its own box the crown fills;
  p10/p50/p90 and p90/p10 of luminance over the plant's OWN pixels, which is H3: a crown with a lit
             outside and a self-shadowed inside spans a wide ratio, and a bag of leaves lit from one
             side spans a narrow one. Sky is excluded, because sky in the box says nothing about the
             crown's depth.

A plain `--box` reports the same percentiles over everything in the box, for frames with no identity
image or for a region a reader wants by hand.
"""

import sys

import numpy as np
from PIL import Image


def luminance(rgb: np.ndarray) -> np.ndarray:
    return 0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2]


def ids_of(path: str) -> np.ndarray:
    """The authored_id png holds the id normalised into eight bits of red, so it is a mask rather than
    a table: ids one and two round to zero and are read as sky, which costs the trunk and nothing in
    the crown, where every id is large enough to survive the quantisation."""
    rgb = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64) / 255.0
    return rgb[..., 0]


def report(lum: np.ndarray, mask: np.ndarray | None, label: str) -> None:
    own = lum if mask is None else lum[mask]
    if own.size == 0:
        print(f"  {label}: no pixels")
        return
    lo, mid, hi = (np.percentile(own, p) for p in (10, 50, 90))
    print(f"  {label}: p10 = {lo:.3f}  p50 = {mid:.3f}  p90 = {hi:.3f}"
          f"  p90/p10 = {hi / max(lo, 1e-6):.2f}")


def crown_depth(lum: np.ndarray, mask: np.ndarray) -> None:
    """H3, measured: a deep crown has plant pixels wholly surrounded by plant, and they are in the
    shadow of the foliage above them, while the crown's rim is lit. A bag of separated leaves has
    almost no interior at all, so the interior's share of the plant is the first number and the
    interior's darkness against the rim is the second. The bar (proposal section 2.1, H3) is an
    interior share of at least 0.25 and an interior-over-rim mean of at most 0.75."""
    plant = np.pad(mask, 1)
    neighbours = sum(
        plant[dy : dy + mask.shape[0], dx : dx + mask.shape[1]]
        for dy in (0, 1, 2)
        for dx in (0, 1, 2)
        if (dy, dx) != (1, 1)
    )
    interior = mask & (neighbours == 8)
    rim = mask & (neighbours <= 4)
    share = interior.mean() / max(mask.mean(), 1e-9)
    print(f"  H3 interior share of the plant = {share:.3f}"
          f"  (bar: at least 0.25)")
    if interior.any() and rim.any():
        ratio = lum[interior].mean() / max(lum[rim].mean(), 1e-6)
        print(f"  H3 interior over rim luminance = {ratio:.3f}"
              f"  (bar: at most 0.75)")
    else:
        print("  H3 interior over rim luminance = n/a (no interior or no rim)")


def main(argv: list[str]) -> int:
    frame = np.asarray(Image.open(argv[0]).convert("RGB"), dtype=np.float64) / 255.0
    lum = luminance(frame)
    height, width = lum.shape
    if "--crown" in argv:
        mask_all = ids_of(argv[1]) > 0.004
        rows = np.where(mask_all.any(axis=1))[0]
        top = rows[0]
        bottom = rows[-1]
        # The crown is the upper part of the plant: from its top down to two thirds of its height,
        # which is where a tree's foliage is and where a trunk-only column cannot reach.
        crown_bottom = top + int(0.66 * (bottom - top))
        cols = np.where(mask_all[:crown_bottom].any(axis=0))[0]
        box = (cols[0], top, cols[-1] + 1, crown_bottom)
        region = frame[box[1] : box[3], box[0] : box[2]]
        region_ids = mask_all[box[1] : box[3], box[0] : box[2]]
        print(f"frame {argv[0]} {width}x{height} crown box {box}")
        print(f"  coverage = {region_ids.mean():.3f}")
        crown_depth(luminance(region), region_ids)
        report(luminance(region), region_ids, "crown plant pixels")
        report(luminance(region), None, "crown all pixels")
        return 0
    box = None
    if "--box" in argv:
        at = argv.index("--box")
        box = tuple(int(part) for part in argv[at + 1 : at + 5])
    region = frame if box is None else frame[box[1] : box[3], box[0] : box[2]]
    print(f"frame {argv[0]} {width}x{height}" + (f" box {box}" if box else ""))
    report(luminance(region), None, "region")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
