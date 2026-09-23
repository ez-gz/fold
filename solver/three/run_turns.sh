#!/bin/bash
# turn re-solves for the pilot flop: common flop lines x stratified turn cards; runs 3 at a time, nice'd
cd "$(dirname "$0")/.."
DUMP=../gpu/out3/co_btn_bb_Kh7d4d_r000
for e in 0 5 2 1 4 6 3; do for c in As Kc Td 8s 5h 2c; do echo "$e $c"; done; done | \
  xargs -P 3 -n 2 sh -c 'out=three/turn_r000_e$0_$1; [ -s $out.json ] && exit 0; nice -n 5 ./target-v1/release/fold-cli resolve3 '"$DUMP"' --gpu --entry $0 --card $1 --iters 200 --target 0.5 --dump $out > $out.log 2>&1 && ./target-v1/release/fold-cli export3 $out --out three/spots_e$0_$1.json --id e$0$1 >> $out.log 2>&1'
echo TURNS_DONE
