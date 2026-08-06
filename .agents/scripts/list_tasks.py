from runtime_state import TASKS,active_task
active=active_task()
if not TASKS.exists(): print('No tasks.'); raise SystemExit(0)
for p in sorted((x for x in TASKS.iterdir() if x.is_dir()),reverse=True): print(('* ' if active and p.resolve()==active.resolve() else '  ')+p.name)
