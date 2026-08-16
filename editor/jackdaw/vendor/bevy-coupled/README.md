# Vendored Bevy-coupled dependencies

This directory contains the editable source closure classified by
`../../docs/bevy-coupled-inventory.json` for Jackdaw's supported Windows build
matrices. General-purpose dependencies that do not directly declare a
`bevy`/`bevy_*` package remain Cargo dependencies.

Source policy:

- Aeronet, Lightyear, and Rerecast are complete upstream Git workspaces at the
  immutable revisions recorded in `PROVENANCE.toml`.
- The other upstreams are exact crates.io package snapshots from the pinned
  Jackdaw lockfile. Their normalized `Cargo.toml`, `Cargo.toml.orig` (when
  published), `.cargo_vcs_info.json` (when published), notices, and licenses
  are retained.
- Avian did not publish `.cargo_vcs_info.json` in either resolved package, so
  its two checksum-verified package snapshots are colocated under one `avian`
  source group instead of guessing an upstream Git revision.
- Bevy-Egui 0.40.1 is a checksum-verified supplemental package snapshot. It is
  outside Jackdaw's 46-package supported closure, but Aeronet's repository
  instructions require all-target Clippy and its examples depend on Bevy-Egui;
  keeping that gate on local Bevy required importing and migrating the exact
  published snapshot.
- Git metadata and Cargo cache-only `.cargo-ok` markers are intentionally not
  imported. Every other imported file was SHA-256 compared with its selected
  source after copying.

Do not update these sources from a branch head. To refresh an upstream, first
freeze a new Jackdaw lockfile, regenerate the metadata inventory, verify the
exact crates.io checksum or Git revision, replace only that upstream directory,
retain all license material, and update `PROVENANCE.toml` plus the migration and
verification ledgers.

Local modifications are recorded per upstream in `PROVENANCE.toml`. At the
initial import checkpoint every list is empty and every copied file is
byte-identical to its selected source.
