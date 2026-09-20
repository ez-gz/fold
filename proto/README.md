# Backbone prototype

De-risks the pipeline end to end before any iOS work: Rust solver -> drill-spot export -> trainer UI.

    cd solver
    cargo test --release                      # exhaustive 5- and 7-card evaluator checks
    ./target/release/fold-cli solve btn_bb Ks7d2c   # formations: btn_bb co_bb sb_bb utg_btn; DCFR to <0.3% pot, writes out/<flop>_r0.json (--rake 1 for 5% cap 1bb)
    ./target-v1/release/fold-cli pack --dir out_v1 --out ../proto/pack-v1.foldpack   # default pack; serve proto/ over http (fetch needs it)
    open ../proto/index.html                  # works from file://, no server

What is real: full 1326-combo DCFR over flop/turn/river, exact best-response exploitability,
per-hand per-action EVs, one ply of villain responses per hero action, equities vs continuing ranges.

What is placeholder: preflop ranges (hand-written), one formation (BTN vs BB SRP), a small bet tree
(flop 33/100, turn 66, river 50/125, one 60% raise per street, no donk bets), JSON instead of .foldpack,
no suit isomorphism (3.75 GB, ~1 s/iteration on 10 cores).
