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

## Robustness notes (2026-09-23, after WSL died mid-batch)

WSL exited on its own around 16:04 during the round-2 preflop batch (PC stayed on, Ubuntu was not running; SSH reset
by peer). 142/225 flops were already collected on the Mac, so nothing was lost, but the queue sat idle for 2.5 hours.
Things that would help, none done yet unless marked:

- done: `preflop/auto_it.sh` tells "box unreachable" apart from "box idle" and logs it once instead of relaunching
  into a dead SSH every 10 minutes.
- Alert the user when the box has been unreachable for 15+ minutes (push notification from the loop), since only a
  human can restart WSL.
- VRAM headroom: round-2 one-sided solves peaked at 7.9/8 GB (wider ranges). Solves that close to the limit are the
  first suspect for instability. Options: split the widest formations' runouts into two passes, or fp16 storage.
- The box's working copy lives under `/tmp/work/gpu` (gpubox default). On WSL `/tmp` survives a restart but not a
  cleanup; a persistent work dir would be safer for multi-hour batches.
- WSL keep-alive on the Windows side (a scheduled task that runs `wsl -d Ubuntu -- true` every few minutes, or
  `wsl --shutdown` avoidance) so an idle WSL VM is not torn down by Windows.
- Every stage is already resumable (`run_it.sh` skips finished flops, the collector re-pulls); keep it that way.
