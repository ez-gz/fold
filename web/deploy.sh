#!/bin/bash
# Build and publish the site to Cloudflare (Workers static assets, config in web/wrangler.jsonc). First time: npx wrangler login
set -e
cd "$(dirname "$0")"
python3 build.py "$@"
npx -y wrangler deploy
