# Jackdaw-on-Bevy integration architecture

This repository combines two Cargo workspaces without flattening them. The Bevy checkout is the authoritative engine source at revision `25368b78ce5e9b15dc770cdf2af4595602cc8a7b`; Jackdaw is an imported, nested workspace at `editor/jackdaw` from revision `b21229553bd42c89c7798ca05d5b8f5f4bf806b3`.

## Workspace boundary

The Bevy root workspace explicitly excludes `editor/jackdaw`. Jackdaw retains its own workspace members, lockfile, profiles, features, lints, toolchain pin, SDK/dylib boundary, examples, and fixtures. This keeps Jackdaw's product graph independently selectable while still unifying all source in one Git repository.

Root commands are exposed through `cargo studio ...`. The Rust xtask always starts nested Cargo processes with `editor/jackdaw` as their working directory, so Jackdaw's `.cargo/config.toml` remains authoritative. On Windows the wrapper imports an x64 Visual Studio developer environment, discovers Ninja, preserves paths containing spaces as structured process arguments, and selects the pinned toolchains without changing the user's global Rust default.

The Bevy root uses Rust `1.96.0`. Jackdaw uses `nightly-2026-07-31`, which satisfies the frozen Bevy snapshot and its SDK/compiler-wrapper work. The supplemental Bevy-Egui standalone gate uses stable 1.96 because it needs no nightly feature and the pinned nightly Cargo's experimental per-unit build layout reproducibly loses dependency metadata on that all-target graph. Command output and generated Cargo metadata are retained beneath the ignored `editor/jackdaw/target/studio-evidence` directory.

## Dependency ownership

Jackdaw's workspace dependency on `bevy` points to `../..` at version `0.20.0-dev`. Official `bevy_*` dependencies in the nested graph point to the corresponding local packages under the repository root. Narrow `[patch]` tables at the Jackdaw root are guardrails for accidentally retained compatible registry or Git declarations; direct manifest rewrites remain the primary ownership mechanism.

The generated inventory classifies an external package when its active manifest directly declares `bevy`, a `bevy_*` package, or an alias for one. The supported default, no-default, all-feature, workspace, test, fixture, SDK, runner, Play-in-Editor, multiplayer, physics, input, and navigation graph produced 46 classified packages grouped into 11 upstream source imports.

Those packages live below `editor/jackdaw/vendor/bevy-coupled` and are editable in place. Multiple packages from the same upstream revision share one source directory. Git imports retain complete upstream workspaces where migration or package grouping requires them; published package snapshots retain the exact crates.io source selected by the original lockfile. Vendored standalone workspaces also resolve their official Bevy dependencies to this checkout.

One additional checksum-verified source, Bevy-Egui 0.40.1, is recorded as supplemental rather than classified. It is not reachable from a supported Jackdaw matrix, but Aeronet's repository instructions mandate workspace/all-feature/all-target Clippy and its examples activate Bevy-Egui. Importing and migrating that exact snapshot keeps the upstream verification graph on local Bevy without inflating the authoritative 46-package Jackdaw closure.

General-purpose dependencies remain registry dependencies when they do not directly depend on official Bevy packages. Vendoring them would not improve Bevy type identity and would turn this integration into an unrelated full-ecosystem mirror. This boundary is the repository's definition of self-sufficiency: all engine-coupled source is local, while ordinary Rust ecosystem packages remain lockfile-pinned.

## Inventory and provenance

`docs/bevy-coupled-inventory.json` is the machine-readable classification record. It preserves the original resolved package IDs, sources, versions, checksums, activation matrices, dependency edges, topological migration order, local manifest mapping, and migration state. `docs/bevy-coupled-inventory.md` is generated from the same data.

`vendor/bevy-coupled/PROVENANCE.toml` is the source-import ledger. Its 12 records cover the 11 classified upstream groups and the one explicitly marked supplemental import. Every record contains the repository URL, immutable revision or registry checksums, package-to-manifest mappings, import method, local path, license files, Bevy before/after identity, local modifications, and source-verification evidence. The inventory refresh mode updates only dynamic lockfile, provenance, local-path, supplemental, and migration-status fields; it does not silently reclassify the frozen original graph.

Run the following after changing provenance or migration status:

```text
cargo studio inventory
cargo studio inventory --check
cargo studio provenance
```

## Source-identity enforcement

`cargo studio audit-bevy-sources` resolves the default, no-default, all-feature, and all-workspace/all-feature metadata matrices for `x86_64-pc-windows-msvc`. It fails if:

- an official Bevy package has a version other than `0.20.0-dev`, has a registry/Git source, or resolves outside this Bevy checkout;
- more than one `bevy` root package or more than one version of an official Bevy package resolves;
- a classified package has a non-local source, resolves outside `vendor/bevy-coupled`, or differs from its inventoried manifest;
- the complete workspace matrix and the 46-package inventory disagree;
- an active manifest retains an official Git Bevy dependency, an unsupported Bevy 0.19 requirement, or a machine-specific absolute Bevy path.

`cargo studio provenance` separately validates all upstream paths, immutable identities/checksums, package mappings, licenses, resolved migration states, and the Jackdaw lockfile identity. `cargo studio preflight` protects the frozen branch/base relationship, clean checkpoint state, nested-Git prohibition, toolchains, and native Windows prerequisites.

`cargo studio check-all` validates Bevy-Egui's standalone all-target/all-feature graph with stable 1.96 before the classified focused package loop, then runs Aeronet's mandated Clippy gate and all remaining focused checks. `--resume-focused-from <exact-package>` is an evidence-preserving recovery point for host command ceilings after the four Jackdaw matrices have already passed; the default command always performs the complete matrix.

## Formatting on Windows

Current `cargo-fmt --all` recursively follows local path dependencies. Once Jackdaw points at the Bevy checkout, that behavior collects the Bevy workspace and all local vendor targets into a single rustfmt invocation, exceeding Windows' command-line limit and crossing the intended nested-workspace boundary.

The root `fmt` alias therefore delegates to `cargo studio format`. The wrapper asks Cargo for the selected workspace's members only, groups Rust target roots by edition, de-duplicates them, and invokes the pinned rustfmt in bounded command-line chunks. The required command remains unchanged:

```text
cargo fmt --manifest-path editor/jackdaw/Cargo.toml --all -- --check
```

The Jackdaw workspace was formatted once after its nightly pin changed; vendored upstream workspaces remain excluded, so their imported source is not mass-reformatted.

## Updating the frozen Bevy snapshot

Treat a Bevy update as a new migration, not a routine pull:

1. Start from a clean checkpoint and freeze the intended new Bevy revision. Do not move it again during the migration.
2. Record the new version, MSRV/toolchain requirements, and root metadata before editing Jackdaw.
3. Update local path version assertions and patch guardrails without changing feature semantics.
4. Run `cargo studio inventory --check` to ensure dependency membership did not drift unexpectedly, then migrate the 11 classified vendored upstreams leaf-first using the generated order and revalidate any supplemental upstream gate.
5. Migrate Jackdaw subsystem-by-subsystem and update `migration-ledger.md` plus `PROVENANCE.toml` Bevy identities and modification records.
6. Refresh the inventory's dynamic fields, run the source audit, and complete every command in `verification-report.md`, including the editor launch.

If the Jackdaw feature graph itself changes, regenerate the original-graph metadata and full inventory before rewiring to the new Bevy snapshot; do not use refresh-only mode to hide a classification change.

## Updating a vendored upstream

1. Select an immutable Git revision or exact crates.io version/checksum from a frozen lockfile.
2. Regenerate the supported metadata inventory and confirm which packages and feature/target edges enter the closure.
3. Import the upstream once into its stable slug, remove nested Git metadata, and verify every copied source file against the selected source.
4. Retain every license and notice and update the matching provenance record and package mappings.
5. Rewire official Bevy and classified dependencies with relative paths while preserving aliases, optionality, features, target conditions, and dependency kinds.
6. Migrate leaf-first, record semantic changes and focused evidence, refresh inventory status, and run the complete verification matrix.

Moving branch heads, unverified source substitutions, and machine-specific paths are never valid update mechanisms.
