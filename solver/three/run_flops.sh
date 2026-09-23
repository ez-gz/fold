#!/bin/bash
# Mac side of the 3-way pipeline: for each flop in the GPU batch, wait for its dump, export flop spots,
# re-solve the 3 commonest turn entries x 3 picked turn cards (3 at a time), export each.  run_flops.sh batch3.txt
cd "$(dirname "$0")/.."
CLI=./target-v1/release/fold-cli
while read -r n <&3; do
  [ -z "$n" ] && continue
  f=${n#co_btn_bb_}; f=${f%_r000}
  until [ -s ../gpu/out3/$n.flopev.f32 ]; do (cd ../gpu && gpubox pull out3/$n.f32 out3/$n.json out3/$n.flopev.f32 >/dev/null 2>&1); sleep 120; done
  echo "=== $f $(date +%H:%M)"
  [ -s three/spots_${f}_flop.json ] || $CLI export3 ../gpu/out3/$n --gpu --flopev ../gpu/out3/$n.flopev.f32 --out three/spots_${f}_flop.json --id ${f}f 2>&1 | tail -1
  entries=$($CLI resolve3 ../gpu/out3/$n --gpu 2>/dev/null | head -3 | awk '{print $1}')
  cards=$(python3 three/pick_cards.py $f)
  for e in $entries; do for c in $cards; do echo "$e $c"; done; done | \
    xargs -P 3 -n 2 sh -c 'out=three/turn_'$f'_e$0_$1; [ -s three/spots_'$f'_e$0_$1.json ] && exit 0; nice -n 5 '$CLI' resolve3 ../gpu/out3/'$n' --gpu --entry $0 --card $1 --iters 200 --target 0.5 --dump $out > $out.log 2>&1 && '$CLI' export3 $out --out three/spots_'$f'_e$0_$1.json --id '$f'e$0$1 >> $out.log 2>&1'
  rm -f three/turn_${f}_e*.f32
  python3 three/pack3.py ../proto/three-v1.json.gz three/spots_*.json | tail -1
done 3< "$1"
echo FLOPS_DONE
