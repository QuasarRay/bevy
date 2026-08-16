# Jackdaw-on-Bevy verification report

This report is updated from command evidence stored under the ignored `editor/jackdaw/target/studio-evidence` directory. It covers local Bevy `25368b78ce5e9b15dc770cdf2af4595602cc8a7b` and imported Jackdaw `b21229553bd42c89c7798ca05d5b8f5f4bf806b3` on `x86_64-pc-windows-msvc`.

## Environment

| Item | Observed value |
|---|---|
| Bevy package identity | `0.20.0-dev` |
| Bevy toolchain | Rust/Cargo `1.96.0` |
| Jackdaw toolchain | `nightly-2026-07-31`, Rust/Cargo `1.99.0-nightly` |
| Native build environment | Visual Studio 18 x64 developer tools with Ninja |
| Jackdaw lockfile SHA-256 | `7c9c770573cbaec40041b4a408d9eff67421fd3e986fa3d678bff92c82de2912` |
| Classified closure | 46 packages from 11 upstream source groups |
| Supplemental verification source | `bevy_egui 0.40.1` from one checksum-verified upstream record |

## Completed integration checks

| Command or evidence | Exit | Result |
|---|---:|---|
| `cargo check -p bevy-studio-xtask --offline --locked` | 0 | Root orchestration compiles on the pinned root toolchain. |
| `cargo clippy -p bevy-studio-xtask --all-targets --offline --locked -- --deny warnings` | 0 | Root orchestration is warning-free under the root lint policy. |
| `cargo studio inventory` | 0 | Dynamic provenance/status and current lockfile identity regenerated for 46 classified packages, 11 classified upstream groups, and one supplemental upstream. |
| `cargo studio inventory --check` | 0 | Checked-in JSON and Markdown inventory match deterministic output. |
| `cargo studio provenance` | 0 | All 12 imports, 46 classified plus one supplemental package mapping, immutable identities/checksums, local paths, and licenses validate. |
| `cargo studio metadata` | 0 | Default, no-default, all-feature, and workspace/all-feature metadata resolve offline and locked; each reports 731 packages and 47 Jackdaw workspace members. |
| `cargo studio audit-bevy-sources` | 0 | All four matrices resolve 66 official Bevy packages locally and all 46 classified packages below the vendor root; no Bevy 0.19 or external official Bevy source remains. |
| `cargo fmt --manifest-path editor/jackdaw/Cargo.toml --all -- --check` | 0 | 183 target roots pass pinned rustfmt through one Windows-safe workspace-bounded chunk. |
| Pinned-nightly `cargo check --offline --locked` from `editor/jackdaw` | 0 | The default editor graph compiles, including the native Manifold kernel. |
| Changed-support-crate `cargo check --tests` matrix | 0 | Jackdaw BSN, Feathers, Panels, UI, and window-chrome test targets compile. |
| Direct `jackdaw_bsn` test executable | 0 | 53 of 53 tests pass. |
| Initial `cargo studio check-all` through focused package 28 | host timeout | Four Jackdaw check matrices, Aeronet's mandated Clippy, and focused packages 1–28 passed; the execution host terminated the healthy process tree at its one-hour ceiling. |
| `cargo studio check-all --resume-focused-from lightyear_metrics` | 0 | Focused packages 29–46 all pass; together with the initial evidence every classified package is green. |
| Aeronet `cargo clippy --workspace --all-features --all-targets` | 0 | Full upstream target graph passes with local Bevy and local Bevy-Egui; Windows AWS-LC controls retain every target/feature. |
| Bevy-Egui stable 1.96 `cargo check --offline --locked --all-targets --all-features` | 0 | Supplemental library, tests, and examples compile after WGPU 30/Bevy-main migration. |
| Bevy-Egui standalone lock/metadata assertion | 0 | Zero Bevy 0.19 lock or metadata packages; all 62 resolved official Bevy identities are local. |

The dependency migration ledger contains the focused upstream checks, Clippy runs, and suites completed during leaf-first migration, including Lightyear 282/282, Replicon 135/135, Leafwing 140 passing with three upstream ignores, Avian 130 runnable passing across both dimensions with one evidence-backed upstream ignore, and the remaining upstream package suites.

## Final matrix status

The root orchestration and source-identity gates are complete. The following resource-intensive final commands are intentionally recorded as pending until the orchestration checkpoint is clean, because `cargo studio preflight` is designed to reject uncheckpointed tracked work:

| Required command | Current status |
|---|---|
| `cargo studio preflight` | Pending clean-checkpoint execution. |
| `cargo studio tree` | Pending final duplicate-tree capture. |
| `cargo studio check-all` | All component evidence passes; pending one canonical post-checkpoint rerun with the new integrated Bevy-Egui gate. |
| `cargo studio test` | Pending final all-workspace/all-target/all-feature run. |
| Established Jackdaw all-target Clippy command | Pending final rerun. |
| `cargo studio build` | Pending editor/CLI/runner/wrapper/SDK/PIE/release build matrix. |
| Root Bevy metadata and targeted xtask health | Pending final clean-checkpoint rerun. |
| `cargo studio editor --smoke-seconds 20` | Pending runtime launch and initialization observation. |

No pending command is claimed as passing. This table will be replaced with exact exit codes, skips, logs, and runtime observations after the final verification run.

## Known test limitations

- Avian 3D's `dynamics::joints::tests::revolute_motor_position_target` is ignored with retained evidence: it produces the same deterministic failure on untouched Avian 0.7.0 against its original Bevy 0.19/Parry 0.27 graph. Related joint tests pass; the remaining risk is an inherited upstream position-motor coverage gap.
- Three Leafwing tests remain ignored by their upstream source; 140 runnable library tests pass.
- Aeronet's pre-existing encrypted WebSocket integration test remains ignored upstream; its other IO, WebSocket, and WebTransport tests pass.
- Runtime rendering and graphical interaction are not yet claimed; they require the final editor launch gate below.

## Runtime observation

Pending. No process-start, editor-initialization, window-rendering, or controlled-termination claim is made yet.
