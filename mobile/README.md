# mobile/ — native TestFlight shell

A thin Capacitor/WKWebView wrapper around the live site, not a second copy of the app.
`capacitor.config.json` points the app straight at `https://fold-poker.gtarpenning.workers.dev`
(`server.url`), so the same fast loop you already have — edit `proto/index.html`,
`web/deploy.sh` → `wrangler deploy` — shows up in the TestFlight build on next launch,
no native rebuild required. `www/index.html` is just Capacitor's required local
fallback and is never actually shown.

Modeled on `~/Desktop/projects/sleep-tune`'s CI (fastlane + match + a self-hosted
GitHub Actions runner → TestFlight); the pieces here are a port of that, not a
redesign.

## What's set up
- `ios/` — generated via `npx cap add ios` (gitignored `Pods/`, checked-in project).
  App/scheme name is `App`, bundle id `com.foldpoker.app`.
- `fastlane/` — `Fastfile` (lane `beta`), `Matchfile`, `Appfile`. Same shape as
  sleep-tune's, pointed at the fold-poker bundle id.
- `.github/workflows/mobile-beta.yml` — pushes to `main` touching `mobile/**`
  build on the same self-hosted runner (`g-mac`) and upload to TestFlight.

## One-time setup still needed (outside this repo, needs your Apple/App Store Connect access)
1. **App Store Connect**: create the app `com.foldpoker.app` ("Fold Poker") under
   your team. Confirm the team id — `fastlane/Appfile` and `fastlane/Matchfile`
   currently assume it's the same team as sleep-tune (`JL39GTJ62X`); change both
   files if fold-poker uses a different team.
2. **Certs repo**: create a private repo `gtarpenning/fold-poker-certs` (match's
   git storage) — can't reuse sleep-tune's, since match stores profiles by repo.
3. **ASC API key**: either reuse sleep-tune's App Store Connect API key or mint
   a new one (App Store Connect → Users and Access → Keys).
4. **GitHub Actions secrets** on this repo: `FOLD_MATCH_DEPLOY_KEY` (deploy key
   for the certs repo), `FOLD_ASC_KEY_ID`, `FOLD_ASC_ISSUER_ID`, `FOLD_ASC_KEY_CONTENT`,
   `FOLD_MATCH_PASSWORD`.
5. Run `bundle exec fastlane bootstrap_certs` once (locally or via
   `gh workflow run`, add a dispatch-only bootstrap workflow if you want it
   from CI) to generate certs/profiles into the certs repo.
6. First real run: `bundle exec fastlane beta`, or push to `main`.

## Local dev
```
cd mobile
npm install
npx cap sync ios
npx cap open ios       # opens Xcode; Cmd+R runs on simulator/device,
                        # loading the live Cloudflare URL
```

## Why point at the remote URL instead of bundling `web/dist`
Bundling would mean every content/puzzle change needs a new TestFlight build —
exactly the slow loop this is meant to avoid. The tradeoff: no offline play yet,
and an outage on `fold-poker.gtarpenning.workers.dev` takes the app down too.
Worth adding a bundled-pack offline fallback later if that matters; not needed
to get a TestFlight build in testers' hands now.
