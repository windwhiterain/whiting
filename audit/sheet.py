"""H1's instrument: one sheet of every species' silhouette, side by side, for a reader who has seen
the photographs and not the models.

Usage:
    python sheet.py <out.png> <name>=<authored_id.png> [<name>=<authored_id.png> …]

Each identity mask is cut to the plant's own bounding box, scaled to a common height and set as
black on white in one row, named underneath by its key. The reader's job is to name each shape;
fewer than seven of ten correct is a fail, and the pairs that are confused are the fact that tells
two species apart badly (proposal section 2.1, H1).
"""

import sys

import numpy as np
from PIL import Image, ImageDraw

CELL_HEIGHT = 420
MARGIN = 24
LABEL = 34


def mask_of(path: str) -> np.ndarray:
    rgb = np.asarray(Image.open(path).convert("RGB"), dtype=np.uint8)
    return rgb[..., 0] > 4


def cell(mask: np.ndarray) -> Image.Image:
    rows = np.where(mask.any(axis=1))[0]
    cols = np.where(mask.any(axis=0))[0]
    cut = mask[rows[0] : rows[-1] + 1, cols[0] : cols[-1] + 1]
    image = Image.fromarray(np.where(cut, 0, 255).astype(np.uint8), mode="L")
    scale = CELL_HEIGHT / image.height
    return image.resize(
        (max(1, int(image.width * scale)), CELL_HEIGHT), Image.NEAREST
    )


def main(argv: list[str]) -> int:
    entries = [part.split("=", 1) for part in argv[1:]]
    cells = [(name, cell(mask_of(path))) for name, path in entries]
    width = sum(image.width for _, image in cells) + MARGIN * (len(cells) + 1)
    sheet = Image.new("L", (width, CELL_HEIGHT + MARGIN * 2 + LABEL), 255)
    draw = ImageDraw.Draw(sheet)
    at = MARGIN
    for name, image in cells:
        sheet.paste(image, (at, MARGIN))
        draw.text((at, MARGIN + CELL_HEIGHT + 6), name, fill=0)
        at += image.width + MARGIN
    sheet.save(argv[0])
    print(f"{argv[0]}: {len(cells)} silhouettes, {width}x{sheet.height}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
