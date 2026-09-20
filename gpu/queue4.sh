#!/usr/bin/env bash
# 3-bet pots, after queue3
cd "$(dirname "$0")"
N="btn_bb_3b_Ks7d2c btn_bb_3b_9s8s6d co_btn_3b_AhTd4c co_btn_3b_8h7h3c"
until grep -q COLLECT_DONE collect3.log 2>/dev/null; do sleep 60; done
gpubox run -d -n fold-queue4 -- bash -lc "./run_queue.sh $N"
bash ./collect.sh $N > collect4.log 2>&1
