#!/usr/bin/env bash
# Runs on the Mac: pull each finished GPU solve, verify + export it with the Rust solver, repack.
set -u
cd "$(dirname "$0")"
for name in "$@"; do
  until gpubox pull "out/$name.f16" "out/$name.iters" >/dev/null 2>&1 && [ -s "out/$name.f16" ]; do sleep 60; done
  form="${name%_*}"; flop="${name##*_}"
  (cd ../solver && ./target/release/fold-cli import "$form" "$flop" "../gpu/out/$name.f16" --iters "$(cat ../gpu/out/$name.iters)" --out "out2/$name.json" 2>&1 | grep -E "imported|wrote" \
     && ./target/release/fold-cli pack --dir out2 2>&1)
  rm -f "out/$name.f16"
done
echo COLLECT_DONE
