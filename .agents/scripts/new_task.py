from __future__ import annotations
import argparse,re,secrets
from datetime import datetime,timezone
from pathlib import Path
from runtime_state import AGENTS_DIR,TASKS,ACTIVE,active_task,atomic_write,state_lock

def slug(s):
    value=re.sub(r'[^a-z0-9]+','-',s.lower()).strip('-')
    return value[:48] or 'task'

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--title',required=True); ap.add_argument('--complexity',choices=['S','M','L','XL'])
    args=ap.parse_args()
    with state_lock():
        current=active_task()
        if current: raise SystemExit(f'active task already exists: {current}')
        ident=datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')+'-'+slug(args.title)+'-'+secrets.token_hex(2)
        task=TASKS/ident;
        from runtime_state import ensure_local
        ensure_local(task); task.mkdir(parents=True); (task/'HANDOFFS').mkdir(); (task/'evidence').mkdir()
        templates=AGENTS_DIR/'templates'
        ledger=(templates/'task-ledger.md').read_text(encoding='utf-8').replace('## Resolved objective\n','## Resolved objective\n\n'+args.title+'\n',1)
        if args.complexity: ledger=ledger.replace('## Complexity class\n','## Complexity class\n\n'+args.complexity+'\n',1)
        atomic_write(task/'TASK.md',ledger)
        for src,dst in [('current.md','CURRENT.md'),('decisions.md','DECISIONS.md'),('risks.md','RISKS.md'),('evidence.md','EVIDENCE.md')]: atomic_write(task/dst,(templates/src).read_text(encoding='utf-8'))
        rel=task.relative_to(AGENTS_DIR).as_posix(); atomic_write(ACTIVE,rel+'\n')
    print(task)
if __name__=='__main__': main()
