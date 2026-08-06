# Jackdaw integration preflight snapshot

This file is the Stage 0 evidence snapshot required by the migration brief. Canonical mutable orchestration state remains under `.agents/runtime/tasks/` per the v3 state-management protocol.

- Recorded: 2026-08-01 (Australia/Sydney)
- Repository: `E:/Game Engines/bevy`
- Starting branch: `main`
- Frozen Bevy commit: `25368b78ce5e9b15dc770cdf2af4595602cc8a7b`
- `origin/main` at preflight: `25368b78ce5e9b15dc770cdf2af4595602cc8a7b`
- Description: `v0.16.0-rc.4-3202-g25368b78c`
- Root package: `bevy 0.20.0-dev`
- Declared `rust-version`: `1.96.0`
- Required invocation: `rustup run 1.96.0 <tool> ...` because system `cargo`/`rustc` resolve to a separate Rust 1.95 installation.
- Verified Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Verified Cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`
- Verified host: `x86_64-pc-windows-msvc`
- CMake: `4.1.2`
- Root metadata: 89 workspace members; workspace root and all reported paths resolve under this repository.
- Initial worktree status: only user-owned untracked `.agents/`.
- Submodules: none.
- Worktrees: one (`E:/Game Engines/bevy`).
- Existing integration branch: none.
- Existing `editor/jackdaw`: absent.
- Root `AGENTS.md`: absent.
- Project `.codex/`: absent.
- Root `.cargo/config.toml`: absent.
- Effective `/status` and `/debug-config`: unavailable in this app session; no child delegation will be used unless effective runtime configuration becomes verifiable.
- Current Jackdaw `main` observed by `git ls-remote`: `b21229553bd42c89c7798ca05d5b8f5f4bf806b3` (selection provenance still to be captured from the exact commit).

The reported Bevy SHA `742979efb0f3d88a48ba7e2b2a175669a7850c61` was not the actual local baseline. Per the brief, the intentional current local `main` snapshot above is authoritative and must remain fixed.

## Continuation snapshot - 2026-08-02

- Active branch: `integration/jackdaw-bevy-main`.
- Frozen Bevy base remains `25368b78ce5e9b15dc770cdf2af4595602cc8a7b`; local `main` and `origin/main` remain at that commit.
- Imported Jackdaw source remains pinned to `b21229553bd42c89c7798ca05d5b8f5f4bf806b3`.
- Latest completed checkpoint before the current Jackdaw source migration: `78ab7931f8d37824ebc87ef585fc4cc46dba5c68`.
- Classified closure: 46 packages across 11 vendored upstream groups; all dependency migrations are resolved in `editor/jackdaw/docs/migration-ledger.md`.
- Active Jackdaw toolchain: `nightly-2026-07-31` (`rustc 1.99.0-nightly`, `cargo 1.99.0-nightly`). Cargo, rustc, and rustdoc must be selected explicitly because the system toolchain is Rust 1.95.
- Windows native prerequisite: import the Visual Studio x64 developer environment and set `CMAKE_GENERATOR=Ninja`; this successfully builds the Manifold CSG kernel.
- Latest source evidence: default locked/offline Jackdaw check exits 0; `jackdaw_bsn` passes 53/53 tests; directly changed support-crate test targets compile; all migrated Rust files pass direct nightly rustfmt check.
- User-owned `.agents/` remains untracked and must not be included in integration commits.
