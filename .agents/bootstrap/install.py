from __future__ import annotations

import argparse
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import socket
import stat
import tempfile
import tomllib

if tuple(__import__('sys').version_info) < (3,11):
    raise SystemExit('Python 3.11+ is required.')

SCRIPT=Path(__file__).resolve()
AGENTS_DIR=SCRIPT.parents[1]
ROOT=AGENTS_DIR.parent
TEMPLATES=AGENTS_DIR/'bootstrap'/'templates'
RUNTIME=AGENTS_DIR/'runtime'
INSTALL_LOCK=RUNTIME/'.install.lock'
MARKER_START='<!-- BEGIN SEQMAX MANAGED BLOCK v3 -->'
MARKER_END='<!-- END SEQMAX MANAGED BLOCK v3 -->'
LINE_MARKER='# seqmax-v3-managed'
SECTION_MARKER='# seqmax-v3-managed-section'

TOP={'model':'gpt-5.6-sol','model_reasoning_effort':'max'}
AGENT_SCALARS={
    'enabled':True,
    'max_concurrent_threads_per_session':1,
    'default_subagent_model':'gpt-5.6-sol',
    'default_subagent_reasoning_effort':'max',
    'interrupt_message':True,
}

class InstallError(RuntimeError): pass

@dataclass
class Change:
    path: Path
    old: bytes|None
    new: bytes|None
    reason: str
    new_mode: int|None = None


def sha(data: bytes|None): return hashlib.sha256(data).hexdigest() if data is not None else None

def reject_symlink_path(path: Path) -> None:
    root=ROOT.resolve()
    try: path.resolve(strict=False).relative_to(root)
    except ValueError as exc: raise InstallError(f'destination escapes repository: {path}') from exc
    try: rel=path.relative_to(ROOT)
    except ValueError as exc: raise InstallError(f'destination is not below repository root: {path}') from exc
    current=ROOT
    for part in rel.parts:
        current=current/part
        if current.exists() and current.is_symlink():
            raise InstallError(f'refusing symlinked destination component: {current}')

def fsync_parent(path: Path) -> None:
    try:
        fd=os.open(path.parent,os.O_RDONLY)
        try: os.fsync(fd)
        finally: os.close(fd)
    except OSError:
        # Directory fsync is unavailable on some platforms/filesystems.
        pass

def atomic_write(path: Path, data: bytes, mode: int|None=None) -> None:
    reject_symlink_path(path)
    path.parent.mkdir(parents=True,exist_ok=True)
    if mode is None:
        mode=stat.S_IMODE(path.stat().st_mode) if path.exists() else 0o644
    fd,tmp=tempfile.mkstemp(prefix=f'.{path.name}.',dir=path.parent)
    try:
        try: os.fchmod(fd,mode)
        except (AttributeError,OSError): pass
        with os.fdopen(fd,'wb') as fh:
            fh.write(data); fh.flush(); os.fsync(fh.fileno())
        os.replace(tmp,path); fsync_parent(path)
    finally:
        if os.path.exists(tmp): os.unlink(tmp)

def remove_file(path: Path) -> None:
    reject_symlink_path(path)
    if path.exists():
        if not path.is_file(): raise InstallError(f'refusing to remove non-file: {path}')
        path.unlink(); fsync_parent(path)

def unique_run_dir(kind: str) -> Path:
    stamp=datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    path=RUNTIME/kind/f'{stamp}-{secrets.token_hex(3)}'
    reject_symlink_path(path)
    path.mkdir(parents=True,exist_ok=False)
    return path

def read_current(path: Path) -> bytes|None:
    reject_symlink_path(path)
    if not path.exists(): return None
    if not path.is_file(): raise InstallError(f'expected a regular file: {path}')
    return path.read_bytes()

@contextmanager
def install_lock():
    reject_symlink_path(INSTALL_LOCK); RUNTIME.mkdir(parents=True,exist_ok=True)
    payload=(json.dumps({'pid':os.getpid(),'host':socket.gethostname(),'created':datetime.now(timezone.utc).isoformat()})+'\n').encode()
    try: fd=os.open(INSTALL_LOCK,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600)
    except FileExistsError as exc: raise InstallError(f'installer/uninstaller lock already exists: {INSTALL_LOCK}') from exc
    try:
        with os.fdopen(fd,'wb') as fh: fh.write(payload); fh.flush(); os.fsync(fh.fileno())
        yield
    finally: INSTALL_LOCK.unlink(missing_ok=True)

def backup(change: Change, backup_root: Path) -> None:
    if change.old is None: return
    dest=backup_root/change.path.relative_to(ROOT)
    atomic_write(dest,change.old)

def apply_changes(changes: list[Change], backup_root: Path) -> None:
    seen=set(); old_modes={}
    for c in changes:
        if c.path in seen: raise InstallError(f'duplicate destination in one transaction: {c.path}')
        seen.add(c.path)
        if read_current(c.path)!=c.old: raise InstallError(f'destination changed after planning: {c.path}')
        old_modes[c.path]=stat.S_IMODE(c.path.stat().st_mode) if c.path.exists() else None
        backup(c,backup_root)
    applied=[]
    try:
        for c in changes:
            if read_current(c.path)!=c.old: raise InstallError(f'destination changed before write: {c.path}')
            if c.new is None: remove_file(c.path)
            else: atomic_write(c.path,c.new,c.new_mode if c.new_mode is not None else old_modes[c.path])
            applied.append(c)
    except Exception as original:
        rollback=[]
        for c in reversed(applied):
            try:
                if read_current(c.path)!=c.new:
                    raise InstallError('destination changed externally after package write; refusing to overwrite during rollback')
                if c.old is None: remove_file(c.path)
                else: atomic_write(c.path,c.old,old_modes[c.path])
            except Exception as exc: rollback.append(f'{c.path}: {exc}')
        if rollback:
            raise InstallError(f'write failed ({original}); rollback conflict/failure: '+ '; '.join(rollback)) from original
        raise InstallError(f'write failed and all applied changes were rolled back: {original}') from original

def merge_agents_md(existing: str, block: str) -> str:
    starts=[m.start() for m in re.finditer(re.escape(MARKER_START),existing)]
    ends=[m.start() for m in re.finditer(re.escape(MARKER_END),existing)]
    if len(starts)!=len(ends) or len(starts)>1: raise InstallError('AGENTS.md contains malformed or duplicate Sequential Max markers')
    nl='\r\n' if '\r\n' in existing else '\n'
    block=block.strip().replace('\n',nl)+nl
    if starts:
        start=starts[0]; end=ends[0]+len(MARKER_END)
        return existing[:start].rstrip()+nl+nl+block+existing[end:].lstrip('\r\n')
    if not existing.strip(): return block
    return existing.rstrip()+nl+nl+block

def toml_literal(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    return json.dumps(v)

TABLE_RE=re.compile(r'^\s*\[([^\[\]]+)\]\s*(?:#.*)?$')
ARRAY_RE=re.compile(r'^\s*\[\[([^\[\]]+)\]\]\s*(?:#.*)?$')

def headers(lines):
    out=[]
    for i,line in enumerate(lines):
        m=ARRAY_RE.match(line)
        if m: out.append(('array',m.group(1).strip(),i)); continue
        m=TABLE_RE.match(line)
        if m: out.append(('table',m.group(1).strip(),i))
    return out

def table_range(lines,name):
    hs=headers(lines); matches=[(kind,n,i) for kind,n,i in hs if kind=='table' and n==name]
    if len(matches)>1: raise InstallError(f'duplicate table [{name}] prevents safe merge')
    if not matches: return None
    start=matches[0][2]; following=[i for _,_,i in hs if i>start]; end=min(following) if following else len(lines)
    return start,end

def find_key(lines,start,end,key):
    pat=re.compile(rf'^\s*{re.escape(key)}\s*='); hits=[i for i in range(start,end) if pat.match(lines[i])]
    if len(hits)>1: raise InstallError(f'duplicate simple key {key!r} prevents safe merge')
    return hits[0] if hits else None

def desired_role_blocks(fresh):
    lines=fresh.splitlines(); result={}
    for kind,name,start in headers(lines):
        if kind!='table' or not name.startswith('agents.seqmax_'): continue
        following=[i for _,_,i in headers(lines) if i>start]; end=min(following) if following else len(lines)
        result[name]=lines[start:end]
    return result

def merge_config(existing: str, fresh: str, force: bool):
    try:
        data=tomllib.loads(existing) if existing.strip() else {}
        desired=tomllib.loads(fresh)
    except tomllib.TOMLDecodeError as exc: raise InstallError(f'existing or template TOML is invalid: {exc}') from exc
    existing_agents=data.get('agents',{})
    if not isinstance(existing_agents,dict): raise InstallError('existing `agents` value is not a table')
    conflicts=[]
    for k,v in TOP.items():
        if k in data and data[k]!=v: conflicts.append(f'top-level {k}: existing={data[k]!r}, required={v!r}')
    for k,v in AGENT_SCALARS.items():
        if k in existing_agents and existing_agents[k]!=v: conflicts.append(f'agents.{k}: existing={existing_agents[k]!r}, required={v!r}')
    for name,spec in desired['agents'].items():
        if name in AGENT_SCALARS: continue
        if name in existing_agents and existing_agents[name]!=spec: conflicts.append(f'agents.{name} already exists with different values')
    if conflicts and not force: raise InstallError('configuration conflicts:\n- '+'\n- '.join(conflicts))

    nl='\r\n' if '\r\n' in existing else '\n'; lines=existing.splitlines()
    first_header=min([i for _,_,i in headers(lines)],default=len(lines))
    additions=[]
    for k,v in TOP.items():
        idx=find_key(lines,0,first_header,k)
        if k in data and idx is None: raise InstallError(f'cannot safely merge non-simple top-level key {k!r}; normalize it first')
        newline=f'{k} = {toml_literal(v)} {LINE_MARKER}'
        if idx is None: additions.append(newline)
        elif force and data.get(k)!=v: lines[idx]=newline
    if additions:
        if first_header and first_header<=len(lines) and lines[first_header-1].strip(): additions.append('')
        lines[first_header:first_header]=additions

    ar=table_range(lines,'agents')
    if any(k in existing_agents for k in AGENT_SCALARS) and ar is None:
        raise InstallError('agents scalar settings use dotted or inline TOML; normalize them into an explicit [agents] table before installation')
    if ar is None:
        hs=headers(lines); child=[i for kind,name,i in hs if kind=='table' and name.startswith('agents.')]
        insert=min(child) if child else len(lines)
        block=[]
        if insert and lines[insert-1].strip(): block.append('')
        block.append(f'[agents] {SECTION_MARKER}')
        block.extend(f'{k} = {toml_literal(v)} {LINE_MARKER}' for k,v in AGENT_SCALARS.items())
        block.append('')
        lines[insert:insert]=block
    else:
        start,end=ar; inserts=[]
        for k,v in AGENT_SCALARS.items():
            idx=find_key(lines,start+1,end,k)
            if k in existing_agents and idx is None: raise InstallError(f'cannot safely merge non-simple agents.{k}; normalize it first')
            newline=f'{k} = {toml_literal(v)} {LINE_MARKER}'
            if idx is None: inserts.append(newline)
            elif force and existing_agents.get(k)!=v: lines[idx]=newline
        if inserts: lines[end:end]=inserts

    blocks=desired_role_blocks(fresh)
    for name,block in blocks.items():
        r=table_range(lines,name)
        if r is None:
            if lines and lines[-1].strip(): lines.append('')
            lines.extend(block)
        else:
            role_key=name.removeprefix('agents.')
            if force and role_key in existing_agents and existing_agents[role_key]!=desired['agents'][role_key]:
                s,e=r; lines[s:e]=block

    merged=nl.join(lines).strip()+nl
    try: parsed=tomllib.loads(merged)
    except tomllib.TOMLDecodeError as exc: raise InstallError(f'merged TOML failed validation: {exc}') from exc
    for k,v in TOP.items():
        if parsed.get(k)!=v: raise InstallError(f'post-merge mismatch for {k}')
    pa=parsed.get('agents',{})
    for k,v in AGENT_SCALARS.items():
        if pa.get(k)!=v: raise InstallError(f'post-merge mismatch for agents.{k}')
    for name,spec in desired['agents'].items():
        if name in AGENT_SCALARS: continue
        if pa.get(name)!=spec: raise InstallError(f'post-merge mismatch for agents.{name}')
    return merged,conflicts

def read_utf8(path):
    try: return path.read_bytes().decode('utf-8')
    except UnicodeDecodeError as exc: raise InstallError(f'{path} is not UTF-8; refusing lossy merge') from exc

def plan(force,skip_agents_md):
    changes=[]; notes=[]
    if not skip_agents_md:
        path=ROOT/'AGENTS.md'; reject_symlink_path(path); old=path.read_bytes() if path.exists() else None
        new=merge_agents_md(read_utf8(path) if old is not None else '',read_utf8(TEMPLATES/'AGENTS.block.md')).encode('utf-8')
        if old!=new: changes.append(Change(path,old,new,'merge managed AGENTS block'))
    config=ROOT/'.codex'/'config.toml'; reject_symlink_path(config); old=config.read_bytes() if config.exists() else None
    fresh=read_utf8(TEMPLATES/'.codex'/'config.fresh.toml')
    if old is None: new=fresh.encode('utf-8'); conflicts=[]
    else: new_text,conflicts=merge_config(read_utf8(config),fresh,force); new=new_text.encode('utf-8')
    notes.extend(conflicts)
    if old!=new: changes.append(Change(config,old,new,'merge Sequential Max config'))
    for src in sorted((TEMPLATES/'.codex'/'agents').glob('seqmax-*.toml')):
        dst=ROOT/'.codex'/'agents'/src.name; reject_symlink_path(dst); old=dst.read_bytes() if dst.exists() else None; new=src.read_bytes()
        if old is not None and b'# seqmax-managed-agent:' not in old and old!=new and not force: raise InstallError(f'refusing to replace unmanaged custom agent: {dst}')
        if old!=new: changes.append(Change(dst,old,new,'install managed custom agent'))
    return changes,notes

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--force-conflicts',action='store_true',help='replace only conflicting Sequential Max settings after backup')
    ap.add_argument('--skip-agents-md',action='store_true')
    ap.add_argument('--dry-run',action='store_true')
    args=ap.parse_args()
    try:
        with install_lock():
            report_dir=unique_run_dir('install-reports')
            report={'root':str(ROOT),'dry_run':args.dry_run,'force_conflicts':args.force_conflicts,'changes':[],'notes':[],'status':'PLANNING'}
            try:
                changes,notes=plan(args.force_conflicts,args.skip_agents_md); report['notes']=notes
                report['changes']=[{'path':str(c.path.relative_to(ROOT)),'reason':c.reason,'had_existing':c.old is not None,'old_sha256':sha(c.old),'new_sha256':sha(c.new),'old_mode':(stat.S_IMODE(c.path.stat().st_mode) if c.path.exists() else None)} for c in changes]
                report_path=report_dir/'install-report.json'
                if args.dry_run:
                    report['status']='DRY_RUN_OK'
                    atomic_write(report_path,(json.dumps(report,indent=2)+'\n').encode())
                else:
                    backup_root=unique_run_dir('backups'); report['backup_root']=str(backup_root.relative_to(ROOT)); report['status']='INSTALLED'
                    report_change=Change(report_path,read_current(report_path),(json.dumps(report,indent=2)+'\n').encode(),'record installation transaction')
                    apply_changes(changes+[report_change],backup_root)
            except Exception as exc:
                report['status']='FAILED'; report['error']=str(exc)
                try: atomic_write(report_dir/'install-report.json',(json.dumps(report,indent=2)+'\n').encode())
                except Exception: pass
                raise
            print(json.dumps(report,indent=2))
    except Exception as exc:
        raise SystemExit(f'Sequential Max installation failed safely: {exc}')
    print('\nRun: python .agents/scripts/verify_configuration.py')
    print('Then start a new trusted Codex session with strict config checking and inspect /status and /debug-config.')
if __name__=='__main__': main()
