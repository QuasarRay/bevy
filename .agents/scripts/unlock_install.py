from __future__ import annotations
import argparse,json,os,socket,sys
from pathlib import Path
BOOT=Path(__file__).resolve().parents[1]/'bootstrap'; sys.path.insert(0,str(BOOT))
from install import INSTALL_LOCK,reject_symlink_path

def pid_alive(pid):
    if not isinstance(pid,int) or pid<=0: return False
    try: os.kill(pid,0); return True
    except ProcessLookupError: return False
    except PermissionError: return True
    except OSError: return False

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--force-live',action='store_true'); args=ap.parse_args(); reject_symlink_path(INSTALL_LOCK)
    if not INSTALL_LOCK.exists(): print('No installer lock.'); return
    try: info=json.loads(INSTALL_LOCK.read_text(encoding='utf-8'))
    except Exception: info={'unparseable':True}
    print(json.dumps(info,indent=2)); alive=info.get('host')==socket.gethostname() and pid_alive(info.get('pid'))
    if alive and not args.force_live: raise SystemExit('Lock owner appears alive; use --force-live only after verifying no install, uninstall, or legacy cleanup is running.')
    INSTALL_LOCK.unlink(); print('Installer lock removed.')
if __name__=='__main__': main()
