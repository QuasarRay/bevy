from runtime_state import active_task
p=active_task()
if not p: raise SystemExit('No active task.')
print(p)
