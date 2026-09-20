#!/usr/bin/env bash
# Runs on the GPU box: solve every "<formation>_<flop>" in the list file to target; failures are logged and skipped.
set -u
mkdir -p out1
while read -r form flop; do
  name="${form}_${flop}"
  [ -s "out1/$name.f16" ] && continue
  uv run python solver.py "spec1/$name" --iters 500 --target 0.3 --every 25 --dump "out1/$name.f16.tmp" 2>&1 | grep -E "iter .*(00|50) |storage|dumped|rror|emory" > "out1/$name.log"
  if [ -s "out1/$name.f16.tmp" ]; then
    grep -oE "iter +[0-9]+" "out1/$name.log" | tail -1 | grep -oE "[0-9]+" > "out1/$name.iters"
    mv "out1/$name.f16.tmp" "out1/$name.f16"
  else echo "$name" >> out1/failed.txt; rm -f "out1/$name.f16.tmp"; fi
done < "$1"
echo BATCH_DONE >> out1/status.txt
