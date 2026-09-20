#!/usr/bin/env bash
# GPU box: coarse-tree EV-measurement solves (spec_pre0 -> out_pre), 1% pot is plenty: flop sampling noise dominates
set -u; mkdir -p out_pre
while read -r form flop; do
  name="${form}_${flop}"; [ -s "out_pre/$name.f16" ] && continue
  uv run python solver.py "spec_pre0/$name" --iters 200 --target 1.0 --every 20 --dump "out_pre/$name.f16.tmp" 2>&1 | grep -E "iter|storage|rror|emory" | tail -3 > "out_pre/$name.log"
  [ -s "out_pre/$name.f16.tmp" ] && mv "out_pre/$name.f16.tmp" "out_pre/$name.f16" || echo "$name" >> out_pre/failed.txt
done < "$1"
