#!/usr/bin/env python3
"""Build the deployable site into web/dist from proto/index.html and a .foldpack.

  python3 web/build.py [path/to/pack.foldpack]      (default: proto/pack-v1.foldpack)
  cd web/dist && python3 -m http.server 8777

proto/index.html stays the single source of truth for the app; this script only
injects the web-app head tags + service-worker registration, points the default
pack at the split pack, and splits the pack into one gzip file per flop.
"""
import json, struct, sys, shutil, hashlib, pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
WEB, DIST = ROOT / "web", ROOT / "web" / "dist"
pack = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "proto" / "pack-v1.foldpack"

shutil.rmtree(DIST, ignore_errors=True)
(DIST / "pack").mkdir(parents=True)

# --- split the pack: index.json + one gzip blob per flop (blobs are copied as-is) ---
buf = pack.read_bytes()
assert buf[:8] == b"FOLDPK01", "not a foldpack"
n = struct.unpack_from("<I", buf, 8)[0]
index = json.loads(buf[12:12 + n]); base = 12 + n
for i, e in enumerate(index["flops"]):
    (DIST / "pack" / f"{i}.json.gz.bin").write_bytes(buf[base + e["offset"]: base + e["offset"] + e["length"]])
    e["file"] = f"{i}.json.gz.bin"   # .bin so hosts don't add Content-Encoding and double-decompress
(DIST / "pack" / "index.json").write_text(json.dumps(index))

# --- the page ---
html = (ROOT / "proto" / "index.html").read_text()
def rep(a, b):
    global html
    assert html.count(a) == 1, f"build anchor missing or repeated: {a[:50]}"
    html = html.replace(a, b)
rep('|| "pack-v1.foldpack"', '|| "pack/index.json"')
rep("<title>Fold — backbone prototype</title>", """<title>Fold</title>
<link rel="manifest" href="manifest.webmanifest">
<meta name="theme-color" content="#0b0f14">
<meta name="apple-mobile-web-app-capable" content="yes">
<meta name="mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
<meta name="apple-mobile-web-app-title" content="Fold">
<link rel="apple-touch-icon" href="icon-180.png">
<link rel="icon" href="icon-192.png">
<meta name="description" content="Solver-backed poker trainer. Play hands, drill decisions, solve range puzzles.">
<meta property="og:title" content="Fold — can you beat my score?">
<meta property="og:description" content="Solver-backed poker trainer. Play the exact hand or range puzzle your friend just played.">
<meta property="og:image" content="https://fold-poker.gtarpenning.workers.dev/og.png">
<meta name="twitter:card" content="summary_large_image">""")
rep("</body>", """<script src="extras.js"></script>
<script>if ("serviceWorker" in navigator) addEventListener("load", () => navigator.serviceWorker.register("sw.js"));</script>
</body>""")
(DIST / "index.html").write_text(html)

shutil.copy(ROOT / "proto" / "arena.js", DIST / "arena.js")
for f in (WEB / "static").iterdir():
    if f.name != "sw.js": shutil.copy(f, DIST / f.name)
# cache name changes whenever the app or the pack changes, so clients pick up new builds
ver = hashlib.sha1(html.encode() + (ROOT / "proto" / "arena.js").read_bytes() + buf[:12 + n] + b"".join(f.read_bytes() for f in sorted((WEB / "static").iterdir()))).hexdigest()[:10]
(DIST / "sw.js").write_text((WEB / "static" / "sw.js").read_text().replace("__VERSION__", ver))
size = sum(f.stat().st_size for f in DIST.rglob("*") if f.is_file())
print(f"web/dist: {len(index['flops'])} flops, {size/1e6:.1f} MB, version {ver}")
