#!/usr/bin/env bash
# after the v0 3-bet queue: smoke the two biggest v1 trees for VRAM, then launch the whole batch
cd "$(dirname "$0")"
until grep -q COLLECT_DONE collect4.log 2>/dev/null; do sleep 60; done
printf 'btn_bb AdJd6d\nsb_bb AcJsTh\n' > smoke1.txt
gpubox run -n fold-v1-smoke -- bash -lc "rm -rf out1; ./run_batch.sh smoke1.txt; cat out1/*.log; cat out1/failed.txt 2>/dev/null" > smoke1.log 2>&1
if gpubox run -- bash -lc 'test -s out1/failed.txt && echo HASFAIL' 2>&1 | grep -q HASFAIL; then echo SMOKE_FAILED >> smoke1.log; exit 1; fi
gpubox run -d -n fold-v1-batch -- bash -lc "./run_batch.sh batch1.txt"
bash ./collect_batch.sh batch1.txt > collect_v1.log 2>&1
