import sys, runpy, torch
sys.argv = ["solver.py", "spec/btn_bb_Ks7d2c", "--iters", "6", "--every", "100"]
from torch.profiler import profile, ProfilerActivity
with profile(activities=[ProfilerActivity.CPU, ProfilerActivity.CUDA]) as prof:
    runpy.run_path("solver.py", run_name="__main__")
    torch.cuda.synchronize()
print(prof.key_averages().table(sort_by="cuda_time_total", row_limit=14, max_name_column_width=40))
