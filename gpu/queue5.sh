#!/usr/bin/env bash
# re-export the flops imported before villain_top / EV-spread drill sampling existed
cd "$(dirname "$0")"
N="btn_bb_Ks7d2c btn_bb_9s8s6d btn_bb_QcJd4h co_bb_Td9d4c co_bb_Ah7s2d co_bb_6s5d2c sb_bb_Jh8d3s sb_bb_Kc5c5d utg_btn_AcQd6h utg_btn_8h7h3c btn_bb_Qs8s3s co_bb_KhQhTd"
until grep -q COLLECT_DONE collect4.log 2>/dev/null; do sleep 60; done
gpubox run -d -n fold-queue5 -- bash -lc "rm -f out/*; ./run_queue.sh $N"
bash ./collect.sh $N > collect5.log 2>&1
