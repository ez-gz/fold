# web/ — the deployable mobile site

`proto/index.html` remains the single source of truth for the app. Nothing here is a copy you edit by hand.

    python3 web/build.py [pack.foldpack]     # default proto/pack-v1.foldpack -> web/dist/ (gitignored)
    cd web/dist && python3 -m http.server 8777

What the build does
- Splits the .foldpack into `dist/pack/index.json` + one gzip file per flop (`<i>.json.gz.bin`, blobs copied untouched).
- Copies `proto/index.html`, swaps the default pack to `pack/index.json`, injects manifest / iOS web-app meta / icon links
  and the service-worker registration. It patches by exact string anchors and **fails loudly** if one goes missing:
  the `<title>` line, `|| "pack-v1.foldpack"`, and `</body>`. If you change those in proto, update `build.py`.
- `static/sw.js`: app shell network-first (deploys show up next load, offline falls back to cache); pack files cache-first.
  Cache name is a hash of the page + pack index, so every build invalidates cleanly.

What changed in proto/index.html to support this (keep these when editing)
- `loadSplit(url)`: used when the pack URL ends in `.json`. Loads 2 flops (plus a share-linked one) before play, then
  streams the rest into `PACK.flops` in the background and clears `state.ppool`. So `PACK.flops` **grows during a session**
  and is in random order — never cache things derived from it at startup, and never use array position as an id.
- Every flop has `f._i` = its position in the pack (set by both loaders). Share links (`#h=<_i>.<hand>` / `#p=<_i>.<spot>`)
  use `_i` via `flopAt(i)`, not `PACK.flops.indexOf`.
- `?pack=foo.foldpack` still loads a monolithic pack exactly as before.

Deploy (not done yet): Cloudflare Pages, build command `python3 web/build.py`, output dir `web/dist`.
Still to do: stable share ids (flop name + spot id instead of pack position), 375px audit, first-run explainer, optional sign-in.
