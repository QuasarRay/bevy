from __future__ import annotations
from pathlib import Path
import hashlib, os, re, subprocess, sys, tempfile, tomllib
AG=Path(__file__).resolve().parents[1]; errors=[]
for p in AG.rglob('*.toml'):
    try: tomllib.loads(p.read_text(encoding='utf-8'))
    except Exception as e: errors.append(f'invalid TOML {p.relative_to(AG)}: {e}')
skill=AG/'skills'/'sequential-max-orchestration'/'SKILL.md'; text=skill.read_text(encoding='utf-8')
parts=text.splitlines()
if len(parts)<4 or parts[0]!='---' or '---' not in parts[1:]: errors.append('invalid skill frontmatter delimiters')
else:
    end=parts[1:].index('---')+1; front='\n'.join(parts[1:end])
    if not re.search(r'^name:\s*sequential-max-orchestration\s*$',front,re.M): errors.append('skill name mismatch')
    if not re.search(r'^description:\s*\S.+$',front,re.M): errors.append('skill description missing')
reg=tomllib.loads((AG/'registry'/'agents.toml').read_text(encoding='utf-8')); fresh=tomllib.loads((AG/'bootstrap'/'templates'/'.codex'/'config.fresh.toml').read_text(encoding='utf-8'))
names=set(); slugs=set()
for spec in reg['agent']:
    if spec['name'] in names: errors.append('duplicate agent name '+spec['name'])
    if spec['slug'] in slugs: errors.append('duplicate agent slug '+spec['slug'])
    names.add(spec['name']); slugs.add(spec['slug'])
    if not spec['name'].startswith('seqmax_'): errors.append('unprefixed agent '+spec['name'])
    role=AG/spec['role']; p=AG/'bootstrap'/'templates'/'.codex'/'agents'/('seqmax-'+spec['slug']+'.toml')
    if not role.is_file(): errors.append('missing role '+spec['role']); continue
    if not p.is_file(): errors.append('missing generated agent '+str(p.relative_to(AG))); continue
    data=tomllib.loads(p.read_text(encoding='utf-8')); dev=data.get('developer_instructions','')
    if data.get('name')!=spec['name'] or data.get('description')!=spec['description'] or data.get('sandbox_mode')!=spec['sandbox_mode']: errors.append(f'{p.name}: registry drift')
    if data.get('model')!='gpt-5.6-sol' or data.get('model_reasoning_effort')!='max': errors.append(f'{p.name}: model/effort drift')
    rh=hashlib.sha256(role.read_text(encoding='utf-8').strip().encode()).hexdigest()
    if f'# role-sha256: {rh}' not in p.read_text(encoding='utf-8'): errors.append(f'{p.name}: role hash drift')
    for phrase in ['Do not spawn','Do not edit `.agents/runtime/`','Do not commit','Do not expose secrets','I did not spawn or delegate']:
        if phrase not in dev: errors.append(f'{p.name}: missing mandatory phrase {phrase}')
    if spec['sandbox_mode']=='read-only' and 'This role is read-only. Do not modify repository files' not in dev: errors.append(f'{p.name}: read-only behavior missing')
    if not isinstance(fresh.get('agents',{}).get(spec['name']),dict): errors.append('fresh config missing '+spec['name'])
if fresh.get('model')!='gpt-5.6-sol' or fresh.get('model_reasoning_effort')!='max': errors.append('fresh parent model/effort mismatch')
fa=fresh.get('agents',{})
if fa.get('max_concurrent_threads_per_session')!=1: errors.append('thread cap is not one')
if fa.get('default_subagent_reasoning_effort')!='max': errors.append('default child effort is not max')
for p in AG.rglob('*'):
    if p.is_file() and p.suffix in {'.md','.toml','.py','.ps1','.sh'}:
        s=p.read_text(encoding='utf-8',errors='replace'); stale='project_doc_max_bytes'+' = 65536'
        if stale in s: errors.append(f'stale 64KiB override in {p.relative_to(AG)}')
for rel in ['ORCHESTRATOR.md','skills/sequential-max-orchestration/SKILL.md','bootstrap/templates/AGENTS.block.md','README.md']:
    s=(AG/rel).read_text(encoding='utf-8')
    for match in re.findall(r'`(\.agents/[^`*<>]+)`',s):
        target=AG.parent/match
        if not target.exists() and not any(x in match for x in ['runtime/tasks/','runtime/install-reports/','runtime/.install.lock']): errors.append(f'{rel}: missing reference {match}')
legacy=AG/'registry'/'legacy-v2'
if legacy.exists():
    legacy_hashes=tomllib.loads((legacy/'agent-hashes.toml').read_text(encoding='utf-8')).get('file',[])
    for item in legacy_hashes:
        snap=legacy/'agents'/item['name']
        if not snap.is_file(): errors.append('missing legacy snapshot '+item['name'])
        elif hashlib.sha256(snap.read_bytes()).hexdigest()!=item['sha256']: errors.append('legacy snapshot hash drift '+item['name'])
regen=subprocess.run([sys.executable,str(AG/'scripts'/'regenerate_managed_templates.py'),'--check'],text=True,capture_output=True)
if regen.returncode: errors.append('generated template drift: '+regen.stdout+regen.stderr)
with tempfile.TemporaryDirectory() as td:
    env=os.environ.copy(); env['PYTHONPYCACHEPREFIX']=td
    py=list((AG/'scripts').glob('*.py'))+list((AG/'bootstrap').glob('*.py'))
    comp=subprocess.run([sys.executable,'-m','py_compile',*map(str,py)],env=env,text=True,capture_output=True)
    if comp.returncode: errors.append('Python compilation failed: '+comp.stderr)
if errors:
    print('PACKAGE AUDIT FAILED'); print('\n'.join('- '+e for e in errors)); raise SystemExit(1)
print('PACKAGE AUDIT PASSED'); print(f'Agents: {len(names)}; registry, generated files, TOML, skill frontmatter, references, and Python validated.')
