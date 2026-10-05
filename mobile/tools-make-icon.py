#!/usr/bin/env python3
"""Generate the Fold Poker iOS app icon.

Palette from proto/index.html so the icon matches the app:
  felt green #123c2c / #06110c, gold accent #e8c547, card white.

Mark: two overlapping rounded cards, fanned, the front one showing a gold
spade. Centered, generous margins, reads at 60px. Drawn at 4x and
downsampled so the rotated edges are smooth.
"""
import sys

try:
    from PIL import Image, ImageDraw
except ImportError:
    sys.exit("Pillow not installed: python3 -m pip install Pillow")

S = 1024
SS = 4           # supersample factor
W = S * SS

FELT_HI = (0x1A, 0x57, 0x3F)
FELT_LO = (0x05, 0x0E, 0x0A)
GOLD = (0xE8, 0xC5, 0x47)
CARD = (0xFF, 0xFF, 0xFF)
CARD_BACK = (0xC8, 0xD3, 0xE0)


def rounded_card(size, fill):
    """A card-shaped RGBA tile with rounded corners."""
    w, h = size
    tile = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ImageDraw.Draw(tile).rounded_rectangle(
        [0, 0, w - 1, h - 1], radius=int(w * 0.11), fill=fill
    )
    return tile


def spade(w, h, fill):
    """A spade glyph as an RGBA tile.

    Built bottom-up so the silhouette is unambiguous at small sizes: a tall
    triangular point, two lobes tucked under its lower half, then the stem.
    The point has to clear the lobes or it reads as a club.
    """
    t = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    dd = ImageDraw.Draw(t)

    # Lobes sit low; their tops are well below the apex.
    r = w * 0.27
    cy = h * 0.56
    dd.ellipse([w * 0.5 - r * 1.86, cy - r, w * 0.5 + r * 0.14, cy + r], fill=fill)
    dd.ellipse([w * 0.5 - r * 0.14, cy - r, w * 0.5 + r * 1.86, cy + r], fill=fill)

    # Tall point from the apex down past the lobe centers.
    dd.polygon([(w * 0.5, 0), (w * 0.04, cy + r * 0.10), (w * 0.96, cy + r * 0.10)], fill=fill)

    # Flared stem.
    dd.polygon(
        [
            (w * 0.5, h * 0.60),
            (w * 0.70, h * 1.0),
            (w * 0.30, h * 1.0),
        ],
        fill=fill,
    )
    return t


img = Image.new("RGB", (W, W), FELT_LO)
d = ImageDraw.Draw(img)

# Felt gradient: lighter top-left, falling off to near-black bottom-right.
for y in range(W):
    t = y / (W - 1)
    d.line(
        [(0, y), (W, y)],
        fill=tuple(round(FELT_HI[i] + (FELT_LO[i] - FELT_HI[i]) * t) for i in range(3)),
    )

cw, ch = int(W * 0.40), int(W * 0.56)

# Back card: fanned left, muted, slightly smaller presence.
back = rounded_card((cw, ch), CARD_BACK).rotate(19, expand=True, resample=Image.BICUBIC)
img.paste(back, (int(W * 0.21), int(W * 0.21)), back)

# Front card: upright, centered-ish, carries the suit.
front = rounded_card((cw, ch), CARD)
sp = spade(int(cw * 0.52), int(ch * 0.46), GOLD)
front.paste(sp, (int(cw * 0.24), int(ch * 0.27)), sp)
front = front.rotate(-7, expand=True, resample=Image.BICUBIC)
img.paste(front, (int(W * 0.33), int(W * 0.20)), front)

img = img.resize((S, S), Image.LANCZOS)

out = sys.argv[1] if len(sys.argv) > 1 else "AppIcon-512@2x.png"
# App Store rejects icons with alpha; save as flat RGB.
img.convert("RGB").save(out, "PNG")
print(f"wrote {out} ({S}x{S})")
