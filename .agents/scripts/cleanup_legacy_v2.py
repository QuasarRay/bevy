from __future__ import annotations
import argparse, hashlib, json, tomllib
from pathlib import Path
import sys
BOOT=Path(__file__).resolve().parents[1]/'bootstrap'; sys.path.insert(0,str(BOOT))
from install import ROOT, AGENTS_DIR, MARKER_START, Change, apply_changes, install_lock, reject_symlink_path, table_range, unique_run_dir
LEGACY=AGENTS_DIR/'registry'/'legacy-v2'
EXPECTED={
'explorer': {'description':'Read-only repository mapping with exact evidence and no delegation.','config_file':'./agents/explorer.toml'},
'researcher': {'description':'Read-only primary-source research for current external facts.','config_file':'./agents/researcher.toml'},
'architect': {'description':'Choose one evidence-backed design before broad edits.','config_file':'./agents/architect.toml'},
'implementer': {'description':'Implement one approved bounded plan and test it.','config_file':'./agents/implementer.toml'},
'diagnostician': {'description':'Reproduce and isolate a demonstrated failure.','config_file':'./agents/diagnostician.toml'},
'test_engineer': {'description':'Design or implement a focused proving test matrix.','config_file':'./agents/test-engineer.toml'},
'adversarial_reviewer': {'description':'Read-only correctness and regression review of actual changes.','config_file':'./agents/adversarial-reviewer.toml'},
'security_reviewer': {'description':'Read-only specialist review for real security and supply-chain risk.','config_file':'./agents/security-reviewer.toml'},
'performance_reviewer': {'description':'Read-only measured performance and scalability review.','config_file':'./agents/performance-reviewer.toml'},
'migration_specialist': {'description':'Plan and verify dependency, framework, schema, or protocol migrations.','config_file':'./agents/migration-specialist.toml'},
'verifier': {'description':'Independently evaluate final acceptance gates using direct evidence.','config_file':'./agents/verifier.toml'},
}
def digest(b): return hashlib.sha256(b).hexdigest()
def plan(remove):
    findings=[]; changes=[]; old_agents=(LEGACY/'AGENTS.md').read_text(encoding='utf-8').rstrip()
    p=ROOT/'AGENTS.md'; reject_symlink_path(p)
    if p.exists():
        b=p.read_bytes(); text=b.decode('utf-8')
        if digest(b)==digest((LEGACY/'AGENTS.md').read_bytes()):
            findings.append({'kind':'exact-v2-agents-md','path':'AGENTS.md'})
            if remove: changes.append(Change(p,b,None,'remove exact legacy v2 AGENTS.md'))
        elif text.startswith(old_agents+'\n\n'+MARKER_START):
            findings.append({'kind':'v2-agents-prefix-before-v3-block','path':'AGENTS.md'})
            if remove:
                new=text[len(old_agents):].lstrip('\r\n').encode(); changes.append(Change(p,b,new,'remove exact legacy v2 AGENTS prefix'))
    hashes=tomllib.loads((LEGACY/'agent-hashes.toml').read_text(encoding='utf-8'))['file']
    for item in hashes:
        p=ROOT/'.codex'/'agents'/item['name']; reject_symlink_path(p)
        if p.is_file():
            actual=digest(p.read_bytes()); exact=actual==item['sha256']; findings.append({'kind':'legacy-agent-file','path':str(p.relative_to(ROOT)),'exact_v2':exact})
            if remove and exact: changes.append(Change(p,p.read_bytes(),None,'remove exact legacy v2 custom agent'))
    cfg=ROOT/'.codex'/'config.toml'; reject_symlink_path(cfg)
    if cfg.is_file():
        b=cfg.read_bytes(); text=b.decode('utf-8'); data=tomllib.loads(text); agents=data.get('agents',{}) if isinstance(data.get('agents',{}),dict) else {}
        removable=[]
        if data.get('project_doc_max_bytes')==65536:
            findings.append({'kind':'legacy-project-doc-max','path':'.codex/config.toml','exact_value':True})
            lines=text.splitlines(); hits=[i for i,x in enumerate(lines) if x.strip().replace(' ','')=='project_doc_max_bytes=65536']
            if remove and len(hits)==1: removable.append((hits[0],hits[0]+1))
        lines=text.splitlines()
        for name,expected in EXPECTED.items():
            if name in agents:
                exact=agents[name]==expected; findings.append({'kind':'legacy-agent-table','path':f'.codex/config.toml:[agents.{name}]','exact_v2':exact})
                if remove and exact:
                    r=table_range(lines,'agents.'+name)
                    if r: removable.append(r)
        if remove and removable:
            for start,end in sorted(removable,reverse=True): del lines[start:end]
            while lines and not lines[-1].strip(): lines.pop()
            nl='\r\n' if b'\r\n' in b else '\n'; new=(nl.join(lines).strip()+nl).encode() if any(x.strip() for x in lines) else None
            if new is not None: tomllib.loads(new.decode())
            changes.append(Change(cfg,b,new,'remove exact legacy v2 config entries'))
    return findings,changes

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--remove-exact',action='store_true'); ap.add_argument('--dry-run',action='store_true'); args=ap.parse_args()
    with install_lock():
        findings,changes=plan(args.remove_exact)
        print(json.dumps({'findings':findings,'planned_changes':[str(c.path.relative_to(ROOT)) for c in changes]},indent=2))
        if args.remove_exact and not args.dry_run:
            backup=unique_run_dir('backups'); apply_changes(changes,backup); print('Removed exact legacy v2 artifacts. Backups:',backup.relative_to(ROOT))
        elif findings and not args.remove_exact: print('Audit only. Re-run with --remove-exact to delete only exact known v2 artifacts.')
if __name__=='__main__': main()
