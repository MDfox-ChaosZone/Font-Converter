import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess as sp
import sys
import time

root=Path.cwd()
temp=Path(os.environ["RUNNER_TEMP"])/"native-workflows"
temp.mkdir(exist_ok=True)
mac=platform.system()=="Darwin"
sample=root/"tests/fixtures"
exe=Path(sys.argv[2])
def directory(name):
    p=temp/name
    p.mkdir()
    return p

def prepare(p,small):
    if small:
        for i in range(6):
            src=sample/("Abel-Regular.ttf" if i%2==0 else "SourceCodePro-Regular.otf")
            shutil.copyfile(src,p/f"font {i}{src.suffix}")
    else:
        first=p/"face 0.otf"
        shutil.copyfile(sample/"SourceCodePro-Regular.otf",first)
        for i in range(1,512):
            os.link(first,p/f"face {i}.otf")

def wait(check, seconds, label):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        value=check()
        if value:return value
        time.sleep(.1)
    raise AssertionError("Timeout: "+label)

def run_cli():
    p=directory("cli-mixed")
    prepare(p,True)
    res=sp.run([exe,"--json",p],check=True,capture_output=True)
    assert json.loads(res.stdout)["summary"]["succeeded"]==6
    print("Packaged CLI mixed batch: 6/6",flush=True)
    p=directory("cli-cancel")
    prepare(p,False)
    with (temp/"cli-report.json").open("wb") as out:
        proc=sp.Popen([exe,"--json",p],stdout=out)
        wait(lambda:list(p.glob("*.woff2")),60,"first CLI output")
        proc.send_signal(signal.SIGINT)
        assert proc.wait(timeout=60)==130
    data=json.loads((temp/"cli-report.json").read_text())
    print("Packaged CLI interrupt", data["summary"],flush=True)
    s=data["summary"]
    assert data["interrupted"] and 0<s["cancelled"]<512 and s["succeeded"]+s["cancelled"]==512

def xd(*args,**kwargs):
    return sp.check_output(["xdotool",*map(str,args)],text=True,**kwargs).strip()

def window(proc):
    if proc.poll() is not None:raise RuntimeError(f"GUI exited {proc.returncode}")
    if mac:
        r=sp.run([temp/"driver","window",str(proc.pid)],capture_output=True,text=True)
        return r.stdout.strip() if r.returncode==0 else None
    r=sp.run(["xdotool","search","--onlyvisible","--pid",str(proc.pid)],capture_output=True,text=True)
    for wid in r.stdout.splitlines():
        if "Font Converter" in xd("getwindowname",wid): return wid
    return None

def geometry(wid):
    lines=xd("getwindowgeometry","--shell",wid).splitlines()
    d=dict(line.split("=",1) for line in lines if "=" in line)
    return [int(d[key]) for key in ("X","Y","WIDTH","HEIGHT")]

def drag(proc,wid,paths):
    if mac:
        sp.run([temp/"driver","drag",str(proc.pid),*paths],check=True,timeout=15)
        return
    source=sp.Popen(["/usr/bin/python3","qa/linux_drag.py",*paths])
    try:
        sid=xd("search","--sync","--onlyvisible","--name","^Font QA drag source$",timeout=15).splitlines()[-1]
        # Keep native source and destination physically separated on the Xvfb display.
        xd("windowmove",wid,300,120)
        xd("windowmove",sid,15,40)
        xd("windowactivate","--sync",sid)
        time.sleep(.4)
        x,y,w,h=geometry(wid)
        sx,sy,sw,sh=geometry(sid)
        start=(sx+sw//2,sy+sh//2); end=(x+w//2,y+h//2)
        xd("mousemove",*start,"mousedown",1)
        for step in range(1,26):
            xd("mousemove",*[a+(b-a)*step//25 for a,b in zip(start,end)])
            time.sleep(.04)
        time.sleep(.4)
        xd("mouseup",1)
        time.sleep(.5)
    finally:
        source.terminate();source.wait(timeout=5)

def click(proc,wid):
    if mac:
        sp.run([temp/"driver","click",str(proc.pid)],check=True)
    else:
        xd("windowactivate","--sync",wid)
        x,y,w,h=geometry(wid)
        xd("mousemove",x+w-65,y+h-32,"click",1)

def gui(mode):
    folder=directory(mode)
    small=mode!="cancel"
    prepare(folder,small)
    startup=mode!="drop"
    args=[exe,folder] if startup else [exe]
    with (temp/(mode+".log")).open("w") as log:
        proc=sp.Popen(args,stdout=log,stderr=sp.STDOUT)
        try:
            wid=wait(lambda:window(proc),35,"native desktop window")
            print(mode,"native window",wid,flush=True)
            if not startup:
                drag(proc,wid,sorted(folder.glob("font *")))
                assert not list(folder.glob("*.woff2"))
            time.sleep(2)
            click(proc,wid)
            outputs=lambda:sorted(folder.glob("*.woff2"))
            if small:
                wait(lambda:len(outputs())==6,50,"all six WOFF2 files")
                for p in outputs():
                    data=p.read_bytes()
                    assert data[:4]==b"wOF2" and int.from_bytes(data[8:12],"big")==len(data)
                print(mode,"6/6 verified WOFF2 outputs from packaged GUI",flush=True)
            else:
                wait(outputs,50,"first GUI output")
                click(proc,wid)
                time.sleep(4)
                a=len(outputs());time.sleep(4);b=len(outputs())
                assert 0<a==b<512 and proc.poll() is None,(a,b)
                print(f"Packaged GUI cancel: {a}/512 complete, count stable over 4s, app alive",flush=True)
        except Exception:
            time.sleep(.3)
            dest=temp/(mode+".png")
            sp.run(["screencapture","-x",dest] if mac else ["import","-window","root",dest],check=False)
            raise
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:proc.wait(timeout=3)
                except sp.TimeoutExpired:proc.kill();proc.wait()
            log.flush()
            print((temp/(mode+".log")).read_text(errors="replace")[-2500:],flush=True)
if sys.argv[1]=="cli":
    run_cli()
else:
    gui(sys.argv[1])
