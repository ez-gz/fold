import time, torch
dev="cuda"; R,H,G=2401,550,24
reach=torch.rand(R,H,device=dev); grp=torch.randint(0,H,(R,52*G),device=dev)
p1=torch.randint(0,52*(G+1),(R,H),device=dev)
order=torch.argsort(torch.rand(R,H,device=dev),1); pos=torch.randint(0,H,(R,H),device=dev)
tri=torch.tril(torch.ones(G,G,device=dev))
def T(name,fn,n=100):
    for _ in range(3): fn()
    torch.cuda.synchronize(); t=time.time()
    for _ in range(n): fn()
    torch.cuda.synchronize(); print(f"{name}: {(time.time()-t)*1000/n:.2f} ms each")
x=torch.gather(reach,1,grp).view(R,52,G)
T("gather groups", lambda: torch.gather(reach,1,grp))
T("cumsum short dim (G=24)", lambda: x.cumsum(2))
T("cumsum via matmul tril", lambda: x @ tri.T)
T("cumsum long dim (H=550)", lambda: reach.cumsum(1))
T("pad+gather lookup", lambda: torch.gather(torch.nn.functional.pad(x,(1,0)).view(R,-1),1,p1))
T("gather sorted", lambda: torch.gather(reach,1,order))
