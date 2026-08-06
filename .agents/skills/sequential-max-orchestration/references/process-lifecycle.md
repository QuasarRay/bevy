# Process Lifecycle

For servers, watchers, GUI applications, and long tests:

- record PID/process handle when possible;
- define startup-success and failure signals;
- use bounded timeouts;
- capture logs;
- terminate child processes cleanly;
- verify no orphan process remains;
- distinguish controlled termination from crash;
- do not leave ports, locks, or temporary worktrees active after verification.

## Agent threads

After a child handoff is accepted or rejected, explicitly close that child thread before another spawn. Do not confuse a completed turn with a closed thread.
