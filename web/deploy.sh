#!/bin/bash
# Build and publish the site to Cloudflare Pages. First time: npx wrangler login
set -e
cd "$(dirname "$0")/.."
python3 web/build.py "$@"
npx -y wrangler pages deploy web/dist --project-name fold --branch main --commit-dirty=true
