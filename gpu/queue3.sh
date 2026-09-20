#!/usr/bin/env bash
# waits for the current collector, then solves the texture-coverage flops
cd "$(dirname "$0")"
N="btn_bb_Qs8s3s co_bb_KhQhTd sb_bb_5d4s3h utg_btn_9c9d4h btn_bb_Ad5d4c co_bb_QhQs7d sb_bb_Ts7d2h utg_btn_JsTs9d"
until grep -q COLLECT_DONE collect.log; do sleep 60; done
gpubox run -d -n fold-queue3 -- bash -lc "./run_queue.sh $N"
bash ./collect.sh $N > collect3.log 2>&1
