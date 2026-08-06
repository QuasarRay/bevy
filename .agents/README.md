# Sequential Max Codex Agent System — v3.1

A task-independent orchestration package for difficult Codex work where correctness matters more than speed.

## What it can enforce

When a repository is trusted and no higher-precedence setting overrides it, the generated project configuration:

- pins the parent and managed custom roles to `gpt-5.6-sol` with `max` reasoning;
- caps concurrently open spawned-agent threads at one;
- uses uniquely prefixed `seqmax_*` role names;
- gives every child a no-delegation rule;
- isolates mutable task state below ignored `.agents/runtime/`.

It cannot override CLI `--config` or model flags, managed organization policy, an untrusted-project decision, unavailable model access, explicit spawn overrides, or live permission changes. Static validation therefore never claims to prove the effective runtime configuration. Max is also not Ultra: this package uses deliberate sequential delegation rather than proactive parallel delegation.

## Install safely

Run from any working directory:

```powershell
python <repo>/.agents/bootstrap/install.py --dry-run
python <repo>/.agents/bootstrap/install.py
```

or use `install.ps1` / `install.sh` beside the installer.

The installer:

- merges one delimited block into `AGENTS.md`;
- preserves unrelated TOML, custom agents, hooks, MCP configuration, and comments where its conservative line merge can do so;
- fails closed on conflicting values, malformed markers, non-UTF-8 files, unsupported dotted/inline target keys, symlinks, or unmanaged filename collisions;
- plans all changes before writes and serializes install/uninstall with a local lock;
- backs up every existing target;
- detects destination drift before writes, writes files atomically, and rolls back the applied set when a write fails without overwriting later external edits;
- records hashes, changes, conflicts, and backup paths under ignored `.agents/runtime/install-reports/`.

`--force-conflicts` intentionally replaces only conflicting Sequential Max keys/tables after backup. Review the dry-run first.

## Validate

```powershell
python .agents/scripts/regenerate_managed_templates.py --check
python .agents/scripts/audit_package.py
python .agents/scripts/verify_manifest.py
python .agents/scripts/test_installer.py
python .agents/scripts/verify_configuration.py
```

Start Codex with strict configuration checking where supported. In a new trusted session, inspect:

```text
/status
/debug-config
```

Confirm the active model, Max effort, project layer, managed role declarations, and a one-thread cap. Higher-precedence runtime layers remain authoritative.

## Uninstall

Surgical removal deletes only marked lines, the managed AGENTS block, managed role tables, and marked agent files:

```powershell
python .agents/bootstrap/uninstall.py --dry-run
python .agents/bootstrap/uninstall.py
```

After a forced conflict replacement, exact restoration is safer. Use the corresponding install report:

```powershell
python .agents/bootstrap/uninstall.py --restore-report .agents/runtime/install-reports/<run>/install-report.json
```

Exact restore refuses when a target changed after installation.

## Runtime task state

```powershell
python .agents/scripts/new_task.py --title "Describe the task" --complexity L
python .agents/scripts/current_task.py
python .agents/scripts/list_tasks.py
python .agents/scripts/close_task.py --result SUCCESS
```

If a crashed state command leaves a lock, inspect it before removal:

```powershell
python .agents/scripts/unlock_state.py
```

If a process is killed and leaves `.agents/runtime/.install.lock`, inspect and remove it with `python .agents/scripts/unlock_install.py`; use `--force-live` only after confirming no installer-related process is active.

The parent is the sole writer of canonical task state. Children return handoffs and may not edit orchestration/configuration state.

## Upgrading from v2

Run `python .agents/scripts/cleanup_legacy_v2.py` before installation. It reports generic v2 roles that can shadow built-ins and removes only exact known v2 artifacts when `--remove-exact` is supplied.
