# GPU solver (gpubox, RTX 2080 8 GB)

Same DCFR as `solver/` (the oracle), batched over runouts in PyTorch: the betting tree is built once per
street shape, tensors carry [49] (turn) / [49,49] (river) batch dims. Matches the Rust solver's exploitability
trajectory to 3 digits; every GPU result is re-verified by the Rust best response on import before export.

    cd solver && ./target/release/fold-cli spec btn_bb Ks7d2c          # -> gpu/spec/<name>.json + .bin
    cd gpu && gpubox run -d -n fold-queue -- bash -lc "./run_queue.sh btn_bb_Ks7d2c ..."
    ./collect.sh btn_bb_Ks7d2c ...                                      # pull, verify, export, repack (Mac)

Measured (BTN vs BB, Ks7d2c, to <0.3% pot, 225 iterations): GPU 127 s vs Mac Rust ~225 s. Peak VRAM 4.3 GB
(SB vs BB: 6.2 GB — the ceiling on this card).

What made it fast: per-card strength-sorted groups for card removal (not an [R,H,K] gather); cumsum over a
short dim replaced by a triangular matmul (torch.cumsum is ~50x slower there); int32 flat index_select;
torch.compile for the elementwise regret/strategy/payoff kernels. It is GPU-bound now (gathers ~22%,
matmul ~15%); next levers are suit isomorphism and 16-bit storage, which help both solvers.
