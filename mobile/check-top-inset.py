#!/usr/bin/env python3
"""Fail if app content starts too close to the top edge of the screenshot.

Written after a real regression: the header rendered directly under the
status bar / Dynamic Island on a real iPhone 15 Pro, because the page's
top safe-area padding was too small (and, separately, because the fixed
CSS had been edited locally but never deployed to the remote URL the app
loads -- this check runs against whatever the simulator actually fetched,
so a stale deploy fails it same as a stale CSS value would).

Scans down from the top of the screenshot for the first row that isn't
(approximately) the app's background color, and fails if that content
starts above MIN_TOP_FRACTION of the screen height.
"""
import sys
from PIL import Image

BG = (0x0b, 0x0f, 0x14)
BG_TOLERANCE = 20              # per-channel -- the status bar itself is pure black
                                # in its corners, which is *closer* to BG than 10
                                # allows for, so too tight a tolerance misreads the
                                # status bar as "content" at y=0.
STATUS_BAR_FRACTION = 0.06     # status bar + Dynamic Island cluster, measured on an
                                # iPhone 17 Pro screenshot (clock/battery icons end
                                # around y=115 of 2622 px, ~4.4%); skip past it first
MIN_TOP_FRACTION = 0.07        # app content must not start above 7% of screen height.
                                # Measured content start on a correctly-padded iPhone
                                # 17 Pro screenshot: 10.1%. 7% leaves margin below that
                                # for device/content variance while still catching the
                                # real regression (content starting at the very top).

def is_bg(px, w, y, step):
    return all(
        abs(px[x, y][0] - BG[0]) <= BG_TOLERANCE and
        abs(px[x, y][1] - BG[1]) <= BG_TOLERANCE and
        abs(px[x, y][2] - BG[2]) <= BG_TOLERANCE
        for x in range(0, w, step)
    )

def first_content_row(img):
    """First row, after the status bar, that isn't the app's own background.

    Scanning from y=0 directly trips on the status bar/notch, which is
    system chrome rendered near-black -- not app content, and not something
    app padding controls. So: skip forward until we see one background row
    (proof the safe-area gap exists at all), then report the first row after
    that which differs from it. If no background row ever appears, the app
    drew content starting at/under the status bar with no gap -- that's the
    regression this check exists for, so return 0.
    """
    w, h = img.size
    px = img.load()
    step = max(1, w // 40)
    bar_end = int(h * STATUS_BAR_FRACTION)
    y = bar_end
    while y < h and not is_bg(px, w, y, step):
        y += 1
    if y >= h:
        return 0  # never found background -- content starts at/above the status bar
    while y < h and is_bg(px, w, y, step):
        y += 1
    return y if y < h else None

def main():
    if len(sys.argv) != 2:
        print("usage: check-top-inset.py <screenshot.png>", file=sys.stderr)
        return 2
    img = Image.open(sys.argv[1]).convert("RGB")
    w, h = img.size
    y = first_content_row(img)
    if y is None:
        print(f"FAIL: no non-background content found at all in {w}x{h} screenshot "
              f"-- app likely didn't render")
        return 1
    frac = y / h
    print(f"    first content row: y={y} of {h} ({frac:.1%} down)")
    if frac < MIN_TOP_FRACTION:
        print(f"FAIL: app content starts at {frac:.1%} of screen height, "
              f"below the required {MIN_TOP_FRACTION:.0%} -- it's drawing "
              f"under the status bar / notch.")
        return 1
    print(f"PASS: content starts at {frac:.1%} of screen height (>= {MIN_TOP_FRACTION:.0%})")
    return 0

if __name__ == "__main__":
    sys.exit(main())
