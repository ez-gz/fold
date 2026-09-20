#!/usr/bin/env bash
# Runs on the Mac: pull each finished solve, verify + export with the v1 Rust binary, keep the strategy in solver/cache.
set -u
exec </dev/null
cd "$(dirname "$0")"; mkdir -p out1 ../solver/cache ../solver/out_v1
CLI=../solver/target-v1/release/fold-cli
while read -r form flop <&3; do
  name="${form}_${flop}"
  [ -s "../solver/out_v1/$name.json" ] && continue
  until gpubox pull "out1/$name.f16" "out1/$name.iters" >/dev/null 2>&1 && [ -s "out1/$name.f16" ]; do
    gpubox pull out1/failed.txt >/dev/null 2>&1; grep -qx "$name" out1/failed.txt 2>/dev/null && { echo "FAILED on box: $name"; continue 2; }
    sleep 90
  done
  (cd ../solver && $CLI import "$form" "$flop" "../gpu/out1/$name.f16" --iters "$(cat ../gpu/out1/$name.iters)" --out "out_v1/$name.json" 2>&1 | grep -E "imported|wrote")
  mv "out1/$name.f16" "../solver/cache/$name.f16"; cp "out1/$name.iters" "../solver/cache/$name.iters"
  gpubox run -- rm -f "out1/$name.f16" >/dev/null 2>&1
  (cd ../solver && $CLI pack --dir out_v1 --out ../proto/pack-v1.foldpack 2>&1 | tail -1)
done 3< "$1"
echo COLLECT_DONE
