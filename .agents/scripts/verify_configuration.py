from __future__ import annotations
import argparse,json,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]; AG=ROOT/'.agents'; cfg=ROOT/'.codex'/'config.toml'; reg=tomllib.loads((AG/'registry'/'agents.toml').read_text(encoding='utf-8')); errors=[]
if not cfg.exists(): errors.append('missing .codex/config.toml')
else:
    try: data=tomllib.loads(cfg.read_text(encoding='utf-8'))
    except Exception as e: errors.append(f'invalid config: {e}'); data={}
    if data.get('model')!='gpt-5.6-sol': errors.append('static parent model is not gpt-5.6-sol')
    if data.get('model_reasoning_effort')!='max': errors.append('static parent reasoning is not max')
    a=data.get('agents',{}); expected={'enabled':True,'max_concurrent_threads_per_session':1,'default_subagent_model':'gpt-5.6-sol','default_subagent_reasoning_effort':'max','interrupt_message':True}
    if not isinstance(a,dict): errors.append('agents is not a table'); a={}
    for k,v in expected.items():
        if a.get(k)!=v: errors.append(f'agents.{k} mismatch')
    for spec in reg['agent']:
        name=spec['name']; entry=a.get(name)
        if not isinstance(entry,dict): errors.append(f'missing role declaration {name}'); continue
        p=(cfg.parent/entry.get('config_file','')).resolve()
        try: p.relative_to(ROOT.resolve())
        except ValueError: errors.append(f'{name} config escapes repository: {p}'); continue
        if not p.is_file(): errors.append(f'{name} config file missing: {p}'); continue
        agent=tomllib.loads(p.read_text(encoding='utf-8'))
        if agent.get('name')!=name: errors.append(f'{p}: name mismatch')
        if agent.get('model')!='gpt-5.6-sol' or agent.get('model_reasoning_effort')!='max': errors.append(f'{p}: model/effort mismatch')
        dev=agent.get('developer_instructions','')
        if 'Do not spawn' not in dev or 'Do not edit `.agents/runtime/`' not in dev: errors.append(f'{p}: missing mandatory child constraints')
result={'valid':not errors,'errors':errors,'scope':'static project files only','effective_runtime_proven':False}
ap=argparse.ArgumentParser(); ap.add_argument('--json',action='store_true'); args=ap.parse_args()
if args.json: print(json.dumps(result,indent=2))
else:
    print('STATIC CONFIGURATION '+('VALID' if not errors else 'INVALID'))
    for e in errors: print('- '+e)
    print('This does not prove effective runtime settings. Start a new trusted session, use strict config checking, then inspect /status and /debug-config.')
if errors: raise SystemExit(1)
