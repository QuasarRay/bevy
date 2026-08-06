# Aggressive Audit of Sequential Max v2 through v3.0

This report records defects found in the previous package and how v3 addresses them. Severity reflects the likely effect in a real repository.

## Critical and high-severity defects

| # | Defect in v2 | Severity | v3 correction |
|---:|---|---|---|
| 1 | The installer backed up and then unconditionally replaced an existing `AGENTS.md`. | Critical | Delimited managed-block merge; existing instructions are preserved. |
| 2 | The installer backed up and then unconditionally replaced `.codex/config.toml`, potentially deleting MCP, sandbox, approval, hooks, profiles, tools, and repo settings. | Critical | TOML-aware merge of only known keys; conflicting values fail closed unless `--force-conflicts` is explicit. |
| 3 | Installation could partially modify files before discovering a configuration conflict. | High | Preflight plans and validates every destination before writes; writes are atomic and reported. |
| 4 | Every file under `.codex/agents/` was required to use Sol/Max and contain the package's no-delegation phrase, invalidating unrelated user agents. | Critical | Verification checks only managed `seqmax-*` agents. |
| 5 | Generic names such as `explorer` shadowed Codex built-in agents and could collide with user roles. | High | Names and files are prefixed `seqmax_` / `seqmax-`. |
| 6 | The verifier claimed “CONFIGURATION VALID” by reading one file, ignoring project trust and higher-precedence CLI/config layers. | Critical | It now reports `STATIC CONFIGURATION VALID` and requires `/status` plus `/debug-config` for effective-runtime proof. |
| 7 | Documentation claimed hard read-only behavior, but live parent permission overrides can supersede child sandbox defaults. | High | Every role has behavioral read-only rules and the docs explicitly disclose runtime override precedence. |
| 8 | Mutable `CURRENT.md`, risks, decisions, and evidence were tracked alongside reusable instructions, polluting Git status and mixing tasks. | High | Runtime state is ignored and isolated per task under `.agents/runtime/tasks/`. |
| 9 | One global state file allowed stale facts from one task to contaminate another. | High | Unique task directories and an atomic `ACTIVE` pointer. |
| 10 | There was no task-close lifecycle or archival status. | High | Added `new_task.py`, `current_task.py`, and `close_task.py`. |
| 11 | The installer used Python `assert`, which disappears under optimized Python. | High | Explicit exceptions and validation failures. |
| 12 | Writes were non-atomic and vulnerable to interruption. | High | Temporary-file plus `os.replace` atomic writes. |
| 13 | Existing symlink destinations could redirect writes outside the repository. | Critical | Installer rejects symlinked destination paths and parents. |
| 14 | Backup names could collide and backups were scattered next to project files. | Medium | Unique timestamp/nonce backup directory under ignored runtime state. |
| 15 | There was no safe uninstall path. | Medium | Added managed-block/managed-file uninstall with backups. |

## Configuration and discovery defects

| # | Defect in v2 | Severity | v3 correction |
|---:|---|---|---|
| 16 | `project_doc_max_bytes` was doubled to 65,536 without need, increasing possible first-turn context consumption. | Medium | Removed; Codex's normal default remains in force. |
| 17 | The package did not explain that project `.codex/` files load only for trusted projects. | High | Effective-runtime preflight and README warning. |
| 18 | CLI flags and `--config` overrides were not acknowledged as higher precedence. | High | Explicit precedence check through `/debug-config`. |
| 19 | Model/effort availability was assumed; silent fallback was not prohibited. | High | Fail-closed Max requirement and explicit limitation reporting. |
| 20 | The package conflated Max with an Ultra-like guarantee. | Medium | Explicit statement that Max is single-task depth and this workflow intentionally avoids Ultra parallelism. |
| 21 | Role descriptions were duplicated without drift checks. | Medium | Registry-backed generation and source-hash validation. |
| 22 | There was no check that config role declarations pointed to existing agent files. | High | Package/config verifier cross-checks every role and path. |
| 23 | Skill YAML front matter was not validated. | Medium | Package audit validates required delimiters, name, and description. |
| 24 | No check verified that referenced protocol/template paths existed. | Medium | Package audit validates declared internal references. |

## Role and orchestration defects

| # | Defect in v2 | Severity | v3 correction |
|---:|---|---|---|
| 25 | `diagnostician` was sandboxed read-only but instructions allowed a diagnostic patch. | Medium | Diagnostic role is strictly read-only; patching returns to the implementer. |
| 26 | `security-reviewer` was read-only but said it could edit when authorized. | Medium | Removed impossible mutation exception. |
| 27 | `verifier` was read-only but allowed test-fixture edits. | Medium | Verifier never writes; missing fixtures are a failed prerequisite. |
| 28 | `migration-specialist` was write-enabled despite primarily planning and verification. | High | Replaced with read-only `seqmax_migration_analyst`; implementation stays with implementer. |
| 29 | Write-capable children were not explicitly forbidden from changing orchestration/config state. | High | Global child policy forbids those paths unless the package itself is the assigned target. |
| 30 | Child configs depended too heavily on the parent spawn packet for handoff shape. | Medium | Every custom agent includes a compact mandatory handoff contract. |
| 31 | Children lacked explicit destructive-Git and secret/logging restrictions. | High | Global child safety rules added to generated agent files. |
| 32 | The package encouraged review and verification too broadly, risking unnecessary agent spend. | Medium | Risk-based stage selection and hard default child caps. |
| 33 | Repeating a role had no machine-visible budget escalation record. | Medium | Task ledger records child budget, used count, and escalation reason. |
| 34 | No effective rule handled a completed child thread that remained open and blocked the one-thread cap. | Medium | Parent must explicitly close each completed thread before spawning another. |

## State, evidence, and operational defects

| # | Defect in v2 | Severity | v3 correction |
|---:|---|---|---|
| 35 | `new_task.py` had unused variables, no title, no active pointer, and second-level filename collision risk. | Medium | Argument parsing, slug + UTC microseconds + random suffix, atomic pointer, lock. |
| 36 | No lock protected concurrent task-state creation. | Medium | Exclusive lock file with stale-lock diagnostics. |
| 37 | No redaction policy protected secrets copied into evidence. | High | Added evidence-redaction protocol. |
| 38 | No policy governed long-running subprocess cleanup. | Medium | Added process lifecycle and cleanup rules. |
| 39 | No distinction existed between generated files, source of truth, and caches beyond a short note. | Medium | Expanded repository-safety protocol and generated-file checks. |
| 40 | No manifest-verification script existed despite shipping a hash manifest. | Medium | Added `verify_manifest.py`. |
| 41 | No package self-audit validated scripts, registry, TOML, front matter, or stale strings. | High | Added `audit_package.py`. |
| 42 | No installer regression tests covered fresh, idempotent, compatible, conflicting, forced, and unrelated-agent scenarios. | High | Added `test_installer.py`. |
| 43 | The PowerShell installer depended on the caller's current directory. | Medium | All launchers resolve the script path; Python derives repository root from its own location. |
| 44 | The installer did not protect unrelated custom-agent files with the same generic names. | High | Names are unique and managed files carry package markers. |
| 45 | No installation report recorded exactly what changed. | Medium | JSON report under `.agents/runtime/`. |
| 46 | No dry-run mode existed. | Medium | Added `--dry-run`. |
| 47 | No package version/changelog/migration guidance existed beyond a bare version file. | Low | Added changelog and v2 migration guide. |
| 48 | No license clarified reuse and modification rights for the generated package. | Low | Added MIT license. |

## Remaining inherent limitations

These cannot be solved by repository files alone:

1. A user can launch Codex with higher-precedence CLI overrides.
2. Organization-managed policy can alter or forbid settings.
3. An untrusted repository causes project `.codex/` configuration to be ignored.
4. Live parent permission changes may supersede custom-agent sandbox defaults.
5. A model may disobey a behavioral instruction; the one-thread config limits concurrency but does not mathematically prove every semantic rule.
6. Max reasoning costs more and is slower. Budget reduction comes from fewer calls and smaller contexts, not cheaper reasoning.
7. Independent final verification is still constrained by the available tools, tests, hardware, credentials, and observability.


# Second adversarial pass: defects found in v3.0 and corrected in v3.1

| # | Defect in v3.0 | Severity | v3.1 correction |
|---:|---|---|---|
| 49 | Atomic writes were per-file only; a failure on the second or later destination left a partially installed package. | Critical | Transactional apply with reverse rollback of every already-applied destination; injected-failure regression test. |
| 50 | The audit report claimed partial-install safety more strongly than the implementation provided. | High | Claims now distinguish planning, per-file atomicity, and all-file rollback; tests exercise rollback. |
| 51 | The TOML section scanner ignored array-of-table headers such as `[[hooks.PreToolUse]]`; `[agents]` insertions could land in the wrong section or fail unpredictably. | Critical | Unified table-boundary scanner handles ordinary and array headers; hook-preservation test added. |
| 52 | Compatible dotted or inline `agents.*` layouts were not safely supported or clearly rejected. | High | Fail-closed diagnostic requires normalization to explicit `[agents]`, with no file modification. |
| 53 | Non-dictionary `agents` values could trigger confusing errors. | Medium | Explicit table-type validation. |
| 54 | Installer runtime/report directories were created without the same symlink escape checks used for target files. | Critical | Every runtime, backup, report, and destination path is checked against symlink escape. |
| 55 | Uninstall had no symlink defense. | Critical | Uninstall reuses the same local-path and transactional safety primitives. |
| 56 | Uninstall was not transactional. | High | Surgical and exact restore operations use the rollback-capable change engine. |
| 57 | Uninstall removed top-level model values by value matching, which could delete equivalent user-owned settings. | High | Only installer-marked scalar/section lines are surgically removed. Pre-existing equal values remain. |
| 58 | Uninstall left managed `[agents]` scalar settings behind, so it was not a complete uninstall. | High | Freshly inserted scalar lines and section headers are marked and removed. |
| 59 | A forced installation could not automatically restore the user's displaced conflicting settings. | High | Install reports record old/new hashes and backup root; `--restore-report` performs exact hash-guarded restoration. |
| 60 | The package claimed registry-backed generation but shipped no generator. | High | Added deterministic `regenerate_managed_templates.py` with `--check`. |
| 61 | Triple-quoted TOML generation was fragile if future role text contained triple quotes or escape sequences. | Medium | Generated instructions use escaped TOML-compatible basic strings. |
| 62 | The manifest verifier checked listed hashes but ignored unexpected unmanifested files. | High | It now compares the complete allowed file set in both directions. |
| 63 | Package audit generated `__pycache__` inside the package, mutating what it audited. | Medium | Python compilation uses a temporary bytecode cache. |
| 64 | Skill frontmatter validation could accept a later `---` in the document body as the closing delimiter. | Low | Exact line-oriented frontmatter boundary validation. |
| 65 | Static configuration verification omitted `agents.interrupt_message`. | Low | All managed scalar settings are checked. |
| 66 | Static verification did not reject agent config paths escaping the repository. | High | Resolved managed paths must remain below the repository root. |
| 67 | Runtime state writes lacked symlink checks. | Critical | Runtime paths and ACTIVE targets are confined below `.agents` and reject symlinked components. |
| 68 | State locks contained only a PID and offered no recovery tool. | Medium | JSON lock metadata includes PID, host, timestamp; guarded `unlock_state.py` added. |
| 69 | Closing a task could silently append closure data even when the task was not ACTIVE. | Medium | Close is fail-closed on missing ACTIVE marker or duplicate Closed section. |
| 70 | There was no task listing command. | Low | Added `list_tasks.py`. |
| 71 | Read-only behavior was implied by sandbox config but not generated as an explicit role-specific instruction. | High | Generator injects an explicit no-repository-mutation rule into every read-only child. |
| 72 | The package had no automatic drift check between the registry, role cards, agent TOMLs, and config role tables. | High | Generator `--check`, role hashes, registry checks, and package audit now cover the complete chain. |
| 73 | Installer tests did not cover paths containing spaces, hooks arrays, malformed markers, dotted keys, symlinks, uninstall, exact restoration, or transaction rollback. | High | Expanded regression suite covers each case. |
| 74 | Exact install reports lacked old/new content hashes, weakening auditability and restoration safety. | Medium | Reports now include hashes for every destination. |
| 75 | UTF-8 decoding failures could surface as generic exceptions without an explicit no-loss policy. | Medium | Installer refuses non-UTF-8 managed targets rather than performing lossy conversion. |
| 76 | Line-ending normalization was unconditional. | Low | Existing CRLF/LF style is preserved for merged text files where practical. |
| 77 | Fresh config ownership was not distinguishable from compatible pre-existing values. | High | Installer-owned lines/sections carry narrow markers; unmarked equal settings remain user-owned. |
| 78 | Package integrity checks were not themselves tied to deterministic generated templates. | Medium | Manifest, generator check, role hashes, and package audit now form independent overlapping checks. |


## Third adversarial pass: uninstall and concurrency defects

| # | Defect in v3.1 candidate | Severity | correction |
|---:|---|---|---|
| 79 | Surgical uninstall removed an installer-created `[agents]` header even when the user later added unmarked entries beneath it, potentially changing their TOML scope. | Critical | Section-aware removal retains a clean `[agents]` header whenever unmarked user content or comments remain; regression test added. |
| 80 | Planning and writing were not protected against concurrent installer/uninstaller runs. | High | Exclusive ignored `.install.lock` shared by install and uninstall. |
| 81 | A destination could change between plan creation and write, allowing a concurrent user edit to be overwritten. | Critical | Optimistic old-content checks run before backup and immediately before each write. |
| 82 | Rollback could overwrite a concurrent edit made after the package's own write. | Critical | Rollback verifies the current content still equals the transaction's new content; otherwise it stops and reports a rollback conflict. |
| 83 | Installer tests used Python `assert`, so `python -O` could disable their checks. | Medium | Tests use explicit condition failures. |
| 84 | Uninstall lacked cross-platform launcher wrappers. | Low | Added `uninstall.ps1` and `uninstall.sh`. |


## Fourth adversarial pass: legacy migration and documentation defects

| # | Defect | Severity | correction |
|---:|---|---|---|
| 85 | The v2 migration guide left exact generic role declarations and files active, so they could continue shadowing built-in agents after v3.1 installation. | High | Added exact-signature legacy audit/cleanup for known v2 files and role tables; ambiguous modifications are report-only. |
| 86 | A v2 `AGENTS.md` followed by the v3 managed block left duplicate orchestration instructions. | Medium | Legacy cleanup recognizes the exact old document or exact old prefix before the managed block. |
| 87 | The changelog contained two top-level `# Changelog` headings. | Low | Consolidated into one heading. |
| 88 | The audit report's third-pass rows lacked their own Markdown table header. | Low | Added a distinct section and table header. |

## Fifth adversarial pass: Windows newline preservation

| # | Defect | Severity | correction |
|---:|---|---|---|
| 89 | `Path.read_text()` applied universal-newline conversion before merge logic inspected the file, so CRLF project files were silently normalized to LF despite the preservation claim. | Medium | Byte-level UTF-8 decoding preserves original line endings; install, surgical uninstall, legacy cleanup, and tests now retain CRLF. |

## Sixth adversarial pass: forced role-table replacement

| # | Defect | Severity | correction |
|---:|---|---|---|
| 90 | `--force-conflicts` compared the full TOML table name `agents.seqmax_*` against child keys stored as `seqmax_*`, so conflicting managed role tables were detected but not replaced. | High | Role keys are normalized before replacement; fail-closed and forced-replacement regression tests now cover the path. |

## Seventh adversarial pass: integrity, stale locks, and filesystem metadata

| # | Defect | Severity | correction |
|---:|---|---|---|
| 91 | The package audit did not verify that exact legacy-v2 source snapshots still matched their recorded hashes. | High | Legacy snapshot hashes are now checked during package audit. |
| 92 | A killed installer/uninstaller could leave `.install.lock` indefinitely with no guarded recovery command. | Medium | Lock metadata includes host/PID/time and `unlock_install.py` refuses a live local owner unless explicitly forced. |
| 93 | Uninstall results were printed but not persisted, weakening auditability. | Medium | Surgical and exact uninstall write JSON reports under ignored runtime state. |
| 94 | Atomic replacement inherited `mkstemp` mode `0600`, potentially changing existing Unix permissions and creating new project files too restrictive. | High | Writes preserve an existing file's mode, default new files to `0644`, carry mode through rollback/exact restore, and test permissions on Unix. |
| 95 | Rename/delete operations were not followed by a best-effort parent-directory `fsync`, weakening crash durability on filesystems that support it. | Medium | Best-effort directory sync follows replace and delete; unsupported platforms safely continue. |
| 96 | The manifest verifier did not enforce deterministic path ordering. | Low | Manifest order is now checked. |

## Eighth adversarial pass: transaction/report consistency

| # | Defect | Severity | correction |
|---:|---|---|---|
| 97 | Install or uninstall could modify project files successfully and then fail while writing its JSON report, producing a false failure result with committed changes. | Critical | The report file is now the last change in the same rollback transaction; report-write failure rolls back project changes. |
| 98 | Installer and state lock files used the platform default creation mode rather than an explicit private mode. | Low | Lock files are created with mode `0600` where mode bits apply. |

## Residual limitations after v3.1

- Conservative TOML merging intentionally rejects dotted/inline target settings instead of trying to rewrite arbitrary TOML syntax.
- Comments may move only where managed insertions occur; the package does not bundle a third-party round-trip TOML editor.
- Exact restoration requires the install report and rejects targets modified since installation.
- Runtime one-thread behavior still depends on the effective trusted-project configuration and cannot be mathematically guaranteed by prose.
- Behavioral no-delegation instructions can reduce but cannot prove model compliance.
- No local package can guarantee model entitlement or prevent a user from launching Codex with higher-precedence overrides.
