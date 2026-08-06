# Changelog

## 3.1.0

- Added transactional rollback, robust TOML section handling, exact restoration, generated-template tooling, stronger integrity checks, runtime symlink safety, concurrent-edit detection, rollback conflict protection, section-safe uninstall, CRLF-preserving merges, filesystem mode preservation, stale-lock recovery, transaction-bound reports, private lock modes, expanded tests, and repeated adversarial audit corrections.

## 3.0.0

- Replaced destructive installer behavior with fail-closed, atomic merging.
- Added conflict detection, managed markers, backups, dry-run mode, and install reports.
- Prefixed every custom agent with `seqmax_` to avoid built-in and user-agent collisions.
- Stopped validating or rewriting unrelated custom agents.
- Removed the 64 KiB project-instruction override to avoid unnecessary context growth.
- Moved mutable task state into ignored per-task runtime directories.
- Added task creation/closing lifecycle, locks, and atomic writes.
- Added package, manifest, configuration, and installer tests.
- Added explicit project-trust, config-precedence, and live-permission caveats.
- Removed contradictions between read-only sandboxes and role instructions.
- Added child protections against orchestration/config/state mutation.
- Added redaction, process cleanup, generated-file, dependency, and external-source protocols.
- Added safe uninstall tooling.

## 2.0.0

Initial task-agnostic Sequential Max package. Superseded because installation and verification guarantees were too strong and several files were unsafe to merge into an existing repository.
