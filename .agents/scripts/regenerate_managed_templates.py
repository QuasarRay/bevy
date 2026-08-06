from __future__ import annotations
import argparse, hashlib, json, tomllib
from pathlib import Path

AG=Path(__file__).resolve().parents[1]
REG=AG/'registry'/'agents.toml'
ROLE_DIR=AG/'roles'
TEMPLATE=AG/'bootstrap'/'templates'/'.codex'
MODEL='gpt-5.6-sol'
EFFORT='max'
MANAGED='# seqmax-v3-managed'

def q(value: str) -> str:
    # JSON strings are valid TOML basic strings for the characters emitted here.
    return json.dumps(value,ensure_ascii=False)

def generated() -> dict[Path,str]:
    reg=tomllib.loads(REG.read_text(encoding='utf-8'))
    global_policy=(ROLE_DIR/'GLOBAL_CHILD_POLICY.md').read_text(encoding='utf-8').strip()
    result={}
    roles=[]
    for spec in reg['agent']:
        role=(AG/spec['role']).read_text(encoding='utf-8').strip()
        read_only=spec['sandbox_mode']=='read-only'
        mutation_rule=(
            '- This role is read-only. Do not modify repository files, generated files, configuration, task state, or external systems.'
            if read_only else
            '- Repository writes are allowed only within the explicit in-scope paths and approved plan in the spawn packet.'
        )
        dev=role+'\n\n'+global_policy+'\n'+mutation_rule+'\n'
        rh=hashlib.sha256(role.encode()).hexdigest()
        body='\n'.join([
            '# seqmax-managed-agent: v3.1',
            f'# role-source: {spec["role"]}',
            f'# role-sha256: {rh}',
            f'name = {q(spec["name"])}',
            f'description = {q(spec["description"])}',
            f'model = {q(MODEL)}',
            f'model_reasoning_effort = {q(EFFORT)}',
            f'sandbox_mode = {q(spec["sandbox_mode"])}',
            f'developer_instructions = {q(dev)}',
            '',
        ])
        result[TEMPLATE/'agents'/f'seqmax-{spec["slug"]}.toml']=body
        roles.append('\n'.join([
            f'[agents.{spec["name"]}]',
            f'description = {q(spec["description"])}',
            f'config_file = {q("./agents/seqmax-"+spec["slug"]+".toml")}',
        ]))
    fresh='\n'.join([
        '# seqmax-managed-config: v3.1',
        f'model = {q(MODEL)} {MANAGED}',
        f'model_reasoning_effort = {q(EFFORT)} {MANAGED}',
        '',
        f'[agents] {MANAGED}-section',
        f'enabled = true {MANAGED}',
        f'max_concurrent_threads_per_session = 1 {MANAGED}',
        f'default_subagent_model = {q(MODEL)} {MANAGED}',
        f'default_subagent_reasoning_effort = {q(EFFORT)} {MANAGED}',
        f'interrupt_message = true {MANAGED}',
        '',
        '\n\n'.join(roles),
        '',
    ])
    snippet='\n'.join([
        '# Merge these settings into a trusted project `.codex/config.toml`.',
        f'model = {q(MODEL)}',
        f'model_reasoning_effort = {q(EFFORT)}',
        '',
        '[agents]',
        'enabled = true',
        'max_concurrent_threads_per_session = 1',
        f'default_subagent_model = {q(MODEL)}',
        f'default_subagent_reasoning_effort = {q(EFFORT)}',
        'interrupt_message = true',
        '',
        '# Managed `[agents.seqmax_*]` role tables are in `config.fresh.toml`.',
        '',
    ])
    result[TEMPLATE/'config.fresh.toml']=fresh
    result[TEMPLATE/'config.required-snippet.toml']=snippet
    return result

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--check',action='store_true'); args=ap.parse_args()
    drift=[]
    for path,content in generated().items():
        if args.check:
            if not path.is_file() or path.read_text(encoding='utf-8')!=content: drift.append(str(path.relative_to(AG)))
        else:
            path.parent.mkdir(parents=True,exist_ok=True); path.write_text(content,encoding='utf-8',newline='\n'); print(path)
    if drift:
        print('GENERATED TEMPLATE DRIFT'); print('\n'.join('- '+x for x in drift)); raise SystemExit(1)
    if args.check: print('GENERATED TEMPLATES CURRENT')
if __name__=='__main__': main()
