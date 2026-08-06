from __future__ import annotations
import argparse, hashlib, json, re, tomllib
from pathlib import Path
from install import ROOT, MARKER_START, MARKER_END, LINE_MARKER, SECTION_MARKER, Change, apply_changes, reject_symlink_path, unique_run_dir, install_lock
ROLE_HEADER=re.compile(r'^\s*\[agents\.seqmax_[^\]]+\]\s*(?:#.*)?$')
ANY_HEADER=re.compile(r'^\s*\[\[?[^\]]+\]\]?\s*(?:#.*)?$')
AGENTS_HEADER=re.compile(r'^\s*\[agents\]\s*(?:#.*)?$')
def sha(data): return hashlib.sha256(data).hexdigest() if data is not None else None

def strip_role_tables(lines):
    out=[]; i=0; changed=False
    while i<len(lines):
        if ROLE_HEADER.match(lines[i]):
            changed=True; i+=1
            while i<len(lines) and not ANY_HEADER.match(lines[i]): i+=1
            continue
        out.append(lines[i]); i+=1
    return out,changed

def strip_managed_config(lines):
    lines,changed=strip_role_tables(lines); out=[]; i=0
    while i<len(lines):
        line=lines[i]
        if line.strip()=='# seqmax-managed-config: v3.1': changed=True; i+=1; continue
        if AGENTS_HEADER.match(line) and SECTION_MARKER in line:
            changed=True; j=i+1; body=[]
            while j<len(lines) and not ANY_HEADER.match(lines[j]):
                if LINE_MARKER not in lines[j]: body.append(lines[j])
                else: changed=True
                j+=1
            # Preserve the table when the user added any unmarked content/comment.
            if any(x.strip() for x in body):
                out.append('[agents]'); out.extend(body)
            i=j; continue
        if LINE_MARKER in line or SECTION_MARKER in line: changed=True; i+=1; continue
        out.append(line); i+=1
    while out and not out[-1].strip(): out.pop()
    return out,changed

def surgical_plan():
    changes=[]; p=ROOT/'AGENTS.md'; reject_symlink_path(p)
    if p.exists():
        old=p.read_bytes(); text=old.decode('utf-8'); starts=text.count(MARKER_START); ends=text.count(MARKER_END)
        if starts!=ends or starts>1: raise RuntimeError('malformed or duplicate managed AGENTS block')
        if starts:
            pat=re.compile(re.escape(MARKER_START)+r'.*?'+re.escape(MARKER_END)+r'\s*',re.S); new=pat.sub('',text).rstrip(); nl='\r\n' if b'\r\n' in old else '\n'
            changes.append(Change(p,old,(new+nl).encode() if new else None,'remove managed AGENTS block'))
    d=ROOT/'.codex'/'agents'; reject_symlink_path(d)
    if d.exists():
        for p in sorted(d.glob('seqmax-*.toml')):
            reject_symlink_path(p); old=p.read_bytes()
            if b'# seqmax-managed-agent:' in old: changes.append(Change(p,old,None,'remove managed custom agent'))
    cfg=ROOT/'.codex'/'config.toml'; reject_symlink_path(cfg)
    if cfg.exists():
        old=cfg.read_bytes(); nl='\r\n' if b'\r\n' in old else '\n'; out,changed=strip_managed_config(old.decode('utf-8').splitlines())
        if changed:
            candidate=nl.join(out).strip(); new=(candidate+nl).encode() if candidate else None
            if new is not None: tomllib.loads(new.decode())
            changes.append(Change(cfg,old,new,'remove managed config entries'))
    return changes

def restore_plan(report_path: Path):
    report=json.loads(report_path.read_text(encoding='utf-8'))
    if report.get('status')!='INSTALLED': raise RuntimeError('report does not describe a completed install')
    backup_root=ROOT/report['backup_root']; reject_symlink_path(backup_root); changes=[]
    for item in report['changes']:
        p=ROOT/item['path']; reject_symlink_path(p); current=p.read_bytes() if p.exists() else None
        if sha(current)!=item.get('new_sha256'): raise RuntimeError(f'current file drifted since install; refusing exact restore: {p}')
        if item['had_existing']:
            b=backup_root/item['path']; reject_symlink_path(b)
            if not b.is_file(): raise RuntimeError(f'missing backup: {b}')
            new=b.read_bytes()
            if sha(new)!=item.get('old_sha256'): raise RuntimeError(f'backup hash mismatch: {b}')
        else: new=None
        changes.append(Change(p,current,new,'restore exact pre-install state',item.get('old_mode')))
    return changes

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--dry-run',action='store_true'); ap.add_argument('--restore-report',type=Path); args=ap.parse_args()
    try:
        with install_lock():
            report_dir=unique_run_dir('uninstall-reports')
            changes=restore_plan(args.restore_report.resolve()) if args.restore_report else surgical_plan()
            summary=[{'path':str(c.path.relative_to(ROOT)),'reason':c.reason,'delete':c.new is None} for c in changes]
            report={'status':'DRY_RUN_OK' if args.dry_run else 'UNINSTALLED','mode':'exact-restore' if args.restore_report else 'surgical','changes':summary}
            from install import atomic_write,read_current
            report_path=report_dir/'uninstall-report.json'
            if args.dry_run:
                atomic_write(report_path,(json.dumps(report,indent=2)+'\n').encode())
            else:
                backup_root=unique_run_dir('backups'); report['backup_root']=str(backup_root.relative_to(ROOT))
                report_change=Change(report_path,read_current(report_path),(json.dumps(report,indent=2)+'\n').encode(),'record uninstall transaction')
                apply_changes(changes+[report_change],backup_root)
            print(json.dumps(report,indent=2))
    except Exception as exc: raise SystemExit(f'Sequential Max uninstall failed safely: {exc}')
if __name__=='__main__': main()
