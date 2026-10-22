import subprocess, json, time, os, sys, statistics as st
B=os.path.dirname(os.path.abspath(__file__))
SERVERS={
 "go":[f"{B}/go/server"],
 "rust":[f"{B}/rs/target/release/bench"],
 "node":["node",f"{B}/node/server.mjs"],
 "python":[f"{B}/py/.venv/bin/python",f"{B}/py/server.py"],
}
INIT={"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"drive","version":"0"}}}
def start(cmd):
    t=time.perf_counter()
    p=subprocess.Popen(cmd,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,bufsize=0)
    p.stdin.write((json.dumps(INIT)+"\n").encode()); r=p.stdout.readline()
    dt=(time.perf_counter()-t)*1000
    assert b'"result"' in r, r[:300]
    p.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
    return p,dt
def call(p,i):
    p.stdin.write((json.dumps({"jsonrpc":"2.0","id":i,"method":"tools/call","params":{"name":"ping","arguments":{}}})+"\n").encode())
    r=p.stdout.readline(); return r
def mem(pid):
    d={}
    for l in open(f"/proc/{pid}/status"):
        k,v=l.split(":",1)
        if k in("VmRSS","VmHWM"): d[k]=int(v.split()[0])/1024
    f=open(f"/proc/{pid}/stat").read().split(")")[1].split(); cpu=(int(f[11])+int(f[12]))/os.sysconf("SC_CLK_TCK")*1000
    return d,cpu
for name,cmd in SERVERS.items():
    starts=[]
    for _ in range(15):
        p,dt=start(cmd); starts.append(dt); p.stdin.close(); p.wait()
    p,_=start(cmd)
    r=call(p,1); assert b"pong" in r, r[:300]
    idle,_=mem(p.pid)
    lat=[]
    for i in range(2,1002):
        t=time.perf_counter(); call(p,i); lat.append((time.perf_counter()-t)*1000)
    m,cpu=mem(p.pid); lat.sort()
    p.stdin.close(); p.wait()
    print(f"{name:7} start p50 {st.median(starts):7.1f} ms | idle RSS {idle['VmRSS']:6.1f} MB | after 1000 calls RSS {m['VmRSS']:6.1f} peak {m['VmHWM']:6.1f} MB | call p50 {lat[500]:.3f} p99 {lat[990]:.3f} ms | CPU total {cpu:.0f} ms")
