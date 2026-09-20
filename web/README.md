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
  now use the stable key `f._k` (`<flop>-<formation slug>`, see `flopKey`) via `flopAt(key)`, plus `&s=<score>` once the sharer has finished;
  the recipient gets a "A friend scored X" banner (`challengeNote`). `state.shareScore` is set where hand and puzzle scores are recorded.
- `?pack=foo.foldpack` still loads a monolithic pack exactly as before.

Web-only layer: `static/extras.js` (injected by the build, never loaded by proto)
- First-run explainer (skipped for visitors arriving on a share link), the You sheet (account, invite, install hint, reset, delete account).
  Opened from the round button in the header, or from the score badge once one exists (keeps the header on one line at 375px).
- Optional cloud sync via Supabase: fill `static/config.json`, run `supabase.sql` once, enable Apple/Google providers.
  Without keys the sheet just says progress is saved on this device. Sync = the `fold.*` localStorage keys as one JSON row per user;
  on first sign-in the side with more answered spots wins, after that writes are pushed (debounced). Sign-out keeps local progress.

Deploy: `web/deploy.sh` (build + `wrangler deploy`, config in `web/wrangler.jsonc`). Live at https://fold-poker.gtarpenning.workers.dev
Still to do: Supabase project + OAuth provider setup (needs the owner's accounts), a real share-preview image, privacy page, analytics.
