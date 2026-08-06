# Migrating from v2

1. Back up the repository's `.agents/`, `.codex/`, and `AGENTS.md` paths.
2. Replace the old reusable `.agents/` package with v3.1.
3. Audit exact legacy live configuration before installation:

```powershell
python .agents/scripts/cleanup_legacy_v2.py
```

4. Remove only byte-for-byte known v2 agent files and exact known v2 config tables when the report is correct:

```powershell
python .agents/scripts/cleanup_legacy_v2.py --remove-exact --dry-run
python .agents/scripts/cleanup_legacy_v2.py --remove-exact
```

Modified or ambiguous generic agents are reported but never deleted automatically. Review those manually because names such as `explorer` can shadow built-ins.

5. Install v3.1:

```powershell
python .agents/bootstrap/install.py --dry-run
python .agents/bootstrap/install.py
```

6. Run the package and static-config validators, then start a new trusted session with strict config checking and inspect `/status` plus `/debug-config`.
7. Do not reuse old tracked `.agents/state/CURRENT.md` as live state. Create a new per-task runtime ledger and copy only still-valid decisions.
