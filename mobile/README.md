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

## Setup status (as of 2026-10-04)
All done — first TestFlight build (1.0, build 2) uploaded and processed
successfully via a manual `fastlane beta` run:
- App Store Connect: `com.foldpoker.app`, team `JL39GTJ62X` (same team as
  sleep-tune; confirmed by the cert match bootstrap_certs issued).
- Certs repo: `ez-gz/fold-private` (private; org-level deploy-keys-enabled
  flipped on for `ez-gz` to allow this — that's an org-wide setting, not
  scoped to this one repo). Write-enabled deploy key "fastlane match (fold
  CI)" installed there; its private half is the `FOLD_MATCH_DEPLOY_KEY`
  secret on `ez-gz/fold`.
- ASC API key (`FOLD_ASC_KEY_ID` / `FOLD_ASC_ISSUER_ID` / `FOLD_ASC_KEY_CONTENT`)
  and `FOLD_MATCH_PASSWORD` are set as secrets on `ez-gz/fold`.
- Signing cert + appstore provisioning profile generated and pushed to
  `fold-private` via `bundle exec fastlane bootstrap_certs`.

**Remaining gap: no self-hosted runner registered for `ez-gz/fold` yet**
(`gh api repos/ez-gz/fold/actions/runners` returns 0). The workflow targets
`runs-on: self-hosted` — a push to `main` touching `mobile/**` will queue
and hang until one exists. Either register a runner (same pattern as
sleep-tune's `g-mac`: install the actions-runner, register it against this
repo specifically — a runner can't silently be shared across repos without
re-registering), or switch the workflow to `runs-on: macos-latest`
(GitHub-hosted — simpler, no machine to maintain, but slower and consumes
Actions minutes).

## Running manually (no CI)
Needed once to bootstrap certs, and anytime you want to ship without CI:
```
cd mobile
bundle install
export ASC_KEY_ID="8QR6XCJXRU"
export ASC_ISSUER_ID="4ac28e33-89df-4cb6-8298-ab82ee09465d"
export ASC_KEY_CONTENT="$(cat /path/to/AuthKey_8QR6XCJXRU.p8)"
export MATCH_PASSWORD="<see memory: fold-poker-match-password>"
export MATCH_GIT_URL="git@github.com:ez-gz/fold-private.git"
bundle exec fastlane beta              # or: bootstrap_certs, to regenerate certs
```
Don't leave these exported in a saved shell script or dotfile — they're
live credentials. Export them in the shell you're running from and let
them die with that shell.

## Local dev (simulator/device, no build/upload)
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
