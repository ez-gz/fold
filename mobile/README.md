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
1. **App Store Connect**: app already created — `com.foldpoker.app` under
   team `JL39GTJ62X` (confirm this matches `fastlane/Appfile` / `Matchfile`;
   update both if the app's actual team id differs).
2. **Certs repo**: `ez-gz/fold-private` (private, deploy-keys enabled at the
   org level). A write-enabled deploy key titled "fastlane match (fold CI)"
   is already added there, and its private half is the `FOLD_MATCH_DEPLOY_KEY`
   secret on `ez-gz/fold` — done.
3. **ASC API key**: still needed — `FOLD_ASC_KEY_ID`, `FOLD_ASC_ISSUER_ID`,
   `FOLD_ASC_KEY_CONTENT` secrets are not yet set. Mint one at App Store
   Connect → Users and Access → Integrations → Keys (App Manager role is
   enough), or confirm an existing key (e.g. from sleep-tune, if same team)
   is still Active and reuse its Key ID / Issuer ID / `.p8` content.
4. **Match password**: pick any passphrase, set it as `FOLD_MATCH_PASSWORD`.
5. Run `bundle exec fastlane bootstrap_certs` once (locally, needs the ASC
   key + `MATCH_PASSWORD` in your env and push access to `fold-private`) to
   generate certs/profiles into the certs repo.
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
