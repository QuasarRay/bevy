from __future__ import annotations
from contextlib import contextmanager
from datetime import datetime,timezone
from pathlib import Path
import json, os, socket, tempfile, time
AGENTS_DIR=Path(__file__).resolve().parents[1]; RUNTIME=AGENTS_DIR/'runtime'; TASKS=RUNTIME/'tasks'; ACTIVE=RUNTIME/'ACTIVE'; LOCK=RUNTIME/'.state.lock'

def ensure_local(path: Path):
    root=AGENTS_DIR.resolve()
    try: path.resolve(strict=False).relative_to(root)
    except ValueError as exc: raise RuntimeError(f'path escapes .agents: {path}') from exc
    current=AGENTS_DIR
    for part in path.relative_to(AGENTS_DIR).parts:
        current=current/part
        if current.exists() and current.is_symlink(): raise RuntimeError(f'symlinked runtime path refused: {current}')

def atomic_write(path: Path,text: str):
    ensure_local(path); path.parent.mkdir(parents=True,exist_ok=True)
    fd,tmp=tempfile.mkstemp(prefix='.'+path.name+'.',dir=path.parent)
    try:
        with os.fdopen(fd,'w',encoding='utf-8',newline='\n') as f: f.write(text); f.flush(); os.fsync(f.fileno())
        os.replace(tmp,path)
    finally:
        if os.path.exists(tmp): os.unlink(tmp)

def lock_info():
    if not LOCK.exists(): return None
    try: return json.loads(LOCK.read_text(encoding='utf-8'))
    except Exception: return {'unparseable':True,'age_seconds':time.time()-LOCK.stat().st_mtime}

@contextmanager
def state_lock():
    ensure_local(LOCK); RUNTIME.mkdir(parents=True,exist_ok=True)
    info={'pid':os.getpid(),'host':socket.gethostname(),'created':datetime.now(timezone.utc).isoformat()}
    try: fd=os.open(LOCK,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600)
    except FileExistsError: raise RuntimeError(f'state lock exists: {LOCK}; info={lock_info()}')
    try:
        with os.fdopen(fd,'w',encoding='utf-8') as f: json.dump(info,f); f.flush(); os.fsync(f.fileno())
        yield
    finally: LOCK.unlink(missing_ok=True)

def active_task():
    ensure_local(ACTIVE)
    if not ACTIVE.exists(): return None
    rel=ACTIVE.read_text(encoding='utf-8').strip(); path=(AGENTS_DIR/rel).resolve()
    try: path.relative_to(AGENTS_DIR.resolve())
    except ValueError: raise RuntimeError('ACTIVE pointer escapes .agents')
    ensure_local(path)
    if not path.is_dir(): raise RuntimeError(f'ACTIVE task missing: {path}')
    return path
