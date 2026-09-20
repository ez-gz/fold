#!/usr/bin/env bash
# Runs on the GPU box: solve each "<formation>_<flop>" spec to target and dump the average strategy.
set -u
mkdir -p out
for name in "$@"; do
  uv run python solver.py "spec/$name" --iters 400 --target 0.3 --every 25 --dump "out/$name.f16.tmp" 2>&1 | grep -E "iter .*(00|50) |storage|dumped|rror" > "out/$name.log"
  grep -oE "iter +[0-9]+" "out/$name.log" | tail -1 | grep -oE "[0-9]+" > "out/$name.iters"
  mv "out/$name.f16.tmp" "out/$name.f16"
done
echo QUEUE_DONE
