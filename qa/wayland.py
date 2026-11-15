import json, os, subprocess as sp, sys, time
from pathlib import Path
sys.path.insert(0,str(Path(__file__).parent))
import verify

def command(*args):
    return sp.check_output(["swaymsg","-r",*map(str,args)],text=True).strip()
def surface(proc):
    if proc.poll() is not None: raise RuntimeError(f"Process exited {proc.returncode}")
    def find(node):
        if node.get("pid")==proc.pid and node.get("shell")=="xdg_shell":
            return node["rect"]
        for child in node.get("nodes",[])+node.get("floating_nodes",[]):
            found=find(child)
            if found:return found
    return find(json.loads(command("-t","get_tree")))
def cursor(*args):
    reply=json.loads(command("seat","seat0","cursor",*args))
    assert reply[0]["success"],reply
def click(proc,wid):
    r=surface(proc)
    cursor("set",r["x"]+r["width"]-65,r["y"]+r["height"]-32)
    cursor("press","button1");time.sleep(.1);cursor("release","button1")
def drag(proc,wid,paths):
    src=sp.Popen(["/usr/bin/python3","qa/linux_drag.py",*paths])
    try:
        r=verify.wait(lambda:surface(src),10,"Wayland GTK drag source")
        t=surface(proc)
        start=(r["x"]+r["width"]//2,r["y"]+r["height"]//2)
        end=(t["x"]+t["width"]//2,t["y"]+t["height"]//2)
        cursor("set",*start)
        cursor("press","button1")
        for step in range(1,31):
            cursor("set",*[a+(b-a)*step//30 for a,b in zip(start,end)])
            time.sleep(.04)
        time.sleep(.4);cursor("release","button1");time.sleep(.5)
    finally:
        src.terminate();src.wait(timeout=5)

verify.window=surface
verify.click=click
verify.drag=drag
verify.gui(sys.argv[1])
