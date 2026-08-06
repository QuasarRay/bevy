from __future__ import annotations
import argparse, json, os, socket
from runtime_state import LOCK,lock_info,ensure_local

def pid_alive(pid):
    if not isinstance(pid,int) or pid<=0: return False
    try: os.kill(pid,0); return True
    except ProcessLookupError: return False
    except PermissionError: return True
    except OSError: return False

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--force-live',action='store_true'); args=ap.parse_args(); ensure_local(LOCK)
    info=lock_info()
    if info is None: print('No state lock.'); return
    print(json.dumps(info,indent=2))
    same=info.get('host')==socket.gethostname(); alive=same and pid_alive(info.get('pid'))
    if alive and not args.force_live: raise SystemExit('Lock owner appears alive; use --force-live only after verifying no state command is running.')
    LOCK.unlink(); print('State lock removed.')
if __name__=='__main__': main()
