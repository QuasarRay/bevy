# State directory

v3 no longer stores mutable task state in tracked files. Use `.agents/runtime/tasks/`, initialized by `scripts/new_task.py`.

This directory exists only to prevent old v2 state files from being mistaken for current canonical state.
