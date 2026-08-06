from __future__ import annotations
import argparse
from datetime import datetime,timezone
from runtime_state import ACTIVE,active_task,atomic_write,state_lock

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--result',required=True,choices=['SUCCESS','PARTIAL','BLOCKED','CANCELLED']); args=ap.parse_args()
    with state_lock():
        task=active_task()
        if not task: raise SystemExit('No active task.')
        current=task/'CURRENT.md'; text=current.read_text(encoding='utf-8')
        marker='## Status\n\nACTIVE'
        if marker not in text: raise SystemExit('CURRENT.md is not in ACTIVE state; refusing duplicate or ambiguous close')
        text=text.replace(marker,'## Status\n\n'+args.result,1)
        if '\n## Closed\n' in text: raise SystemExit('Task already has a Closed section')
        text+='\n## Closed\n\n'+datetime.now(timezone.utc).isoformat()+'\n'
        atomic_write(current,text); ACTIVE.unlink(missing_ok=True)
    print(task)
if __name__=='__main__': main()
