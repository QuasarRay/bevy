# Jackdaw Bevy-main migration ledger

This ledger records compiler-driven changes from the pinned Bevy 0.19-compatible sources to local Bevy `0.20.0-dev` at `25368b78ce5e9b15dc770cdf2af4595602cc8a7b`. Dependency ownership was rewired at checkpoint `ca2ae139caa6` before API migration began.

## Dependency migration order

The authoritative package order is generated in `bevy-coupled-inventory.json`. Work proceeds leaf-first; a package marked **resolved** has passed a focused `cargo check` from Jackdaw's locked graph. Tests are recorded independently because upstream dev-only graphs can contain packages outside the supported Jackdaw closure.

| Upstream | Status | Current evidence |
|---|---|---|
| Aeronet | resolved | IO check in Jackdaw's graph, standalone integration tests, and the repository-mandated workspace/all-feature/all-target Clippy pass on a fully local Bevy graph |
| Bevy-Egui (supplemental) | resolved | exact 0.40.1 registry snapshot, standalone all-target/all-feature check, and direct lock/metadata audit against local Bevy |
| Bevy Enhanced Input | resolved | focused check, regression test, and Clippy pass |
| Bevy Monitors | resolved | focused check, 7 library tests, and all-feature Clippy pass without source changes |
| Bevy Replicon | resolved | focused check, 135 library tests, and Clippy pass |
| Bevy Transform Interpolation | resolved | focused check, standalone test-target compilation, and all-feature Clippy pass without source changes |
| Glam Matrix Extras | resolved | focused check, 21 library tests, and all-feature Clippy pass |
| Leafwing Input Manager | resolved | focused check, 140 library tests, and all-feature Clippy pass; 3 upstream tests remain ignored by upstream |
| Lightyear | resolved | all 29 classified packages check; supported feature matrices, 282/282 tests, all-feature Clippy, and local-source audit pass |
| Avian | resolved | both focused checks, local-Bevy source audits, explicit valid feature matrices, 63/63 avian2d tests, 67/67 runnable avian3d tests, and test-target Clippy passes; one deterministic upstream avian3d failure is documented and ignored |
| Bevy Heavy | resolved | default/all-feature checks, 14 library tests, and all-feature Clippy pass without source changes |
| Rerecast | resolved | four classified package checks, supported no-default/no-std matrices, all-feature Clippy, 12/12 algorithm tests, three zero-test integration harnesses, and an all-feature local-source audit |

## Failure and repair records

### Lightyear — widget interaction and full-width entity serialization

- Packages: all 29 classified Lightyear 0.28.0 packages, from `lightyear_metrics` and the transport/core leaves through the `lightyear` umbrella.
- Target and features: each default library; literal no-default libraries for 27 packages; explicit `2d,f32` and `3d,f32` no-default matrices for the two Avian adapters; grouped all-feature all-target compilation; all-feature no-deps library Clippy for all classified packages.
- First relevant errors: the debug tools package referenced Bevy's removed legacy `Button`/mutable `Interaction` UI API, and the delta test registered a component through Replicon's network protocol path without the now-required protocol hasher. Runtime-focused serialization tests then exposed loss of the top two entity-index bits.
- Root cause: Bevy main moved button behavior into `bevy_ui_widgets`, models a press with the transient `Pressed` marker, and reserves the top valid entity index for `Entity::PLACEHOLDER`. Lightyear shifted its `u32` index before widening it to a varint, truncating high indices. Current Replicon protocol registration also has stronger setup requirements that the delta unit test does not need.
- Old API: legacy `bevy_ui::Button` plus mutable `Interaction` reset; `u32` entity-index shifting followed by unchecked/truncating decode; network component registration in the isolated delta-manager test.
- New API: defensively install `ButtonPlugin` and consume `Added<Pressed>` once; widen to `u64` before adding flag bits and use checked generation/index conversion on decode; initialize only Lightyear's component registry for the delta test.
- Semantic risk: moderate. UI toggles retain one-action-per-press behavior, while entity serialization deliberately expands correctness to Bevy's complete valid index domain. Low-index wire bytes remain unchanged. A regression test covers index `1 << 30`, the highest ordinary index, and `Entity::PLACEHOLDER`.
- Feature-matrix scope: a literal no-default workspace selection is invalid because `lightyear_avian2d` and `lightyear_avian3d` require an explicit dimension and precision. The other 27 packages pass literal no-default checks, and both adapters pass their explicit minimum valid matrices. Literal all-feature `lightyear_replication` testing is also not a supported product matrix: its `delta` feature reaches upstream `unimplemented!` bodies and its bare Avian dependencies omit required dimension/precision features. The maximal supported replication matrix excludes only `delta`, adds the documented Avian `2d/f32` and `3d/f32` features, and passes all tests. Jackdaw and the top-level `lightyear` feature surface do not activate `delta`.
- Upstream target scope: preserved Lightyear demos and examples are not classified packages and are not reachable from Jackdaw's supported graph. Their optional GUI tooling can retain Bevy-0.19-era dependencies without entering the verified closure.
- Files changed: the Lightyear workspace and tools manifests, `crates/tools/tools/src/ui/debug.rs`, `crates/transport/serde/src/entity_map.rs`, and the delta test setup in `crates/replication/replication/src/delta.rs`.
- Verification: all 29 default library checks pass; grouped all-feature all-target compilation produces 31 test executables; the supported suites pass 282/282 tests; unified all-feature no-deps library Clippy passes with only the previously recorded Aeronet deprecations. The all-feature `lightyear` reachability audit covers 796 packages, reaches all 29 classified Lightyear packages beneath the vendored upstream, finds 65 official Bevy packages at the exact local checkout, and finds no Bevy 0.19 or alternate official Bevy source. The external `bevy_mikktspace` package is a standalone utility rather than an official versioned Bevy workspace package.

### Avian — Parry/glam identity, published-fixture, and test-graph migration

- Packages: `avian2d 0.7.0` and `avian3d 0.7.0`.
- Target and features: default libraries; four maximal valid no-default-feature matrices per package covering f32/f64 with SIMD or enhanced determinism; default test targets.
- First relevant error: roughly 110 mismatched vector, matrix, and isometry types while compiling the default libraries.
- Root cause: Parry 0.27 selects glamx 0.2 and glam 0.32, while local Bevy uses glam 0.33.2. Parry 0.28 is the smallest release whose glamx 0.3 edge shares glam 0.33 identity.
- Old API: Parry 0.27/glamx 0.2/glam 0.32 types crossing Bevy-main physics boundaries.
- New API: Parry 0.28/glamx 0.3/glam 0.33.2, with avian2d's direct test-only glam dependency aligned to the same version.
- Semantic risk: moderate. The Parry upgrade is intentionally minimal, and the valid precision/determinism feature combinations plus both library suites cover collider, solver, joint, transform, and mass-property behavior.
- Feature-matrix scope: literal `--all-features` is invalid upstream because Parry's SIMD and enhanced-determinism modes are mutually exclusive and Avian's f32/f64 modes are mutually exclusive. Each package instead passed default Clippy plus maximal f32+SIMD, f32+enhanced-determinism, f64+SIMD, and f64+enhanced-determinism no-deps library Clippy matrices, including the applicable debug, joint, parallel, scene, picking, serialization, diagnostics, validation, and mesh-collider features.
- Test-graph scope: `bevy_mod_debugdump 0.16` was used only by upstream debugdump examples and requires Bevy 0.19. The unused dev-dependency and explicit example targets were removed from both active and original manifests; their source files remain as documentation-only reference material.
- Fixture repair: the published avian3d crate retained a scene test but omitted `assets/ferris.glb`. The exact fixture was restored from Avian tag `v0.7.0` at `965e85bf590f53fbf8a29f7ebd0326b5dbc985d1` (SHA-256 `4bf5184325a1e5f441bf74256799eaa19ae8a9cf6c8a64362a901bcd898a33be`), and the scene test now runs on Windows.
- Numeric test repair: aggregate 2D mass/inertia and 3D compound-mesh vertex assertions now use tight tolerances for last-bit changes introduced by the Parry/glam upgrade; counts and topology remain exact.
- Upstream test limitation: `dynamics::joints::tests::revolute_motor_position_target` deterministically reports `0.120598495` against a `> 0.3` assertion. An untouched avian3d 0.7.0 control on its original Bevy 0.19, Parry 0.27, and glam 0.32 graph produced the identical failure, so the compiled test is explicitly ignored rather than weakening its assertion. Remaining risk is the inherited upstream position-motor coverage gap; related revolute/prismatic velocity, force, limit, spring-damper, and combined-target tests pass.
- Files changed: both packages' active/original manifests and lockfiles, two numeric test modules, the avian3d joint-test attribute, and the restored avian3d fixture.
- Verification: both default checks exit 0; both targeted dependency trees contain only local Bevy-main packages; all ten library Clippy matrix runs and both default test-target Clippy runs exit 0; avian2d passes 63/63 library tests; avian3d passes 67/67 runnable library tests with the one evidence-backed upstream ignore.

### Rerecast — glam type features and error-context disambiguation

- Packages: `rerecast 0.3.4`, `bevy_rerecast_core 0.4.1`, `bevy_rerecast_editor_integration 0.4.1`, and `bevy_rerecast 0.4.0` from Git revision `493074cd8c1e8c97bc8fc6b51da01e5295696860`.
- Target and features: default libraries in leaf-first order; documented no-std defaults for `rerecast` and core; literal no-default features for the two std-facing integration packages; all-feature no-deps library Clippy for all four.
- First relevant errors: 76 reflection trait errors in core because Rerecast's glam 0.32 types differed from Bevy's glam 0.33 types; after alignment, ambiguous `.context` calls in core and editor integration; the no-std gate then exposed missing glam integer-vector families.
- Root cause: glam 0.33 changed type identity and feature-gated type families that glam 0.32 always exposed, while Bevy main exports `bevy_ecs::error::ContextExt` through the ECS prelude alongside `anyhow::Context`.
- Old API: glam 0.32 with unconditional vector families and implicit method resolution for both context traits.
- New API: glam 0.33.2 with `all-types`, explicit `bevy_ecs::error::ContextExt::context` for `BevyError`, and explicit `anyhow::Context::context` for `anyhow::Result`.
- Semantic risk: low to moderate. Navmesh numeric types retain identity with Bevy and the old glam type surface; error variants and messages are unchanged; algorithm tests and feature-expanded checks guard generation behavior.
- No-std scope: bare `--no-default-features` is intentionally incomplete because glam requires a math backend. The upstream-documented `default_no_std` feature was used for `rerecast` and core; the editor-facing packages require core `std` directly and pass literal no-default checks.
- Upstream target scope: Jackdaw's classified closure does not use the standalone `bevy_rerecast_editor`, `avian_rerecast`, or the TrenchBroom example graph. Those full-tree sources remain preserved, while the classified packages and their reachable `test_utils` support code are the verified targets.
- Files changed: the Rerecast workspace manifest and lockfile, core navmesh generation, and editor-integration serialization.
- Verification: all four default and supported no-default checks exit 0; all four all-feature no-deps library Clippy runs exit 0; `rerecast` passes 12/12 library tests and the three integration crates' zero-test harnesses compile and run; the all-feature `bevy_rerecast` reachability audit covers 384 packages, finds 68 official Bevy packages local, all four classified packages local, and no Bevy 0.19 or external official Bevy source. The crates.io `bevy_mikktspace` utility is intentionally not an official versioned Bevy workspace package.

### Aeronet — error-context, example-graph, and Windows native-build migration

- Packages: `aeronet_io 0.21.0`, `aeronet_steam 0.21.0`, `aeronet_websocket 0.21.0`, and `aeronet_webtransport 0.21.0`.
- Target and features: each classified library with all features; IO library tests; WebSocket and WebTransport integration tests with all features; the upstream-required `cargo clippy --workspace --all-features --all-targets` gate.
- First relevant error: six `multiple applicable items in scope` diagnostics for `.context` and `.with_context` in the feature-gated Steam server.
- Root cause: Bevy main exports `bevy_ecs::error::ContextExt` through the ECS prelude, which overlaps `anyhow::Context` on `Option` and `Result`.
- Old API: method syntax selected `anyhow::Context` implicitly.
- New API: explicit `anyhow::Context::{context, with_context}` in functions returning `anyhow::Result`, and explicit `bevy_ecs::error::ContextExt::with_context` in the observer returning `BevyError`.
- Semantic risk: low. Error types and context strings are unchanged; only trait selection is explicit.
- Files changed: `crates/aeronet_steam/src/server.rs`, the Aeronet workspace manifest and lockfile, and the Steam/WebSocket/WebTransport package manifests.
- Test-graph scope: upstream interactive Egui examples are not shipped by Jackdaw, but Aeronet's repository instructions require them to compile under the all-target Clippy gate. The released `bevy_egui` edge targeted Bevy 0.19, so the exact 0.40.1 snapshot was imported, migrated, and rewired locally. Missing direct `bevy_egui` dev-dependencies were restored in the three example-owning packages rather than relying on an undeclared transitive edge.
- Host build note: on this Visual Studio 18 installation, AWS-LC debug builds require the Ninja generator, C11 selection, a short target path, and `AWS_LC_SYS_NO_ASM=1`; this avoids an unsupported Visual Studio generator and the space-sensitive prebuilt-NASM batch wrapper without changing release source.
- Verification: the IO check in Jackdaw's locked graph exits 0; all four standalone focused checks exit 0; all four targeted dependency trees contain only local Bevy-main packages; the required whole-workspace/all-feature/all-target Clippy command exits 0; IO passes 2/2 library tests; WebSocket passes 4/4 runnable integration tests with one pre-existing encrypted test ignored upstream; WebTransport passes 2/2 integration tests. Directly selecting the optional packages from Jackdaw's default graph hits a Cargo 1.96 activated-feature resolver panic, so their package-focused evidence comes from the pinned standalone workspace.

### Bevy-Egui — supplemental Aeronet all-target closure

- Package: `bevy_egui 0.40.1`, exact crates.io snapshot checksum `7466095822999850fc52074c0b65e9f8b42da100bf7bbd10e5dc87415c6b3401` with VCS revision `248da1144e3cfbae2ea578b1a71e80803ef2eeeb`.
- Classification: not reachable from a supported Jackdaw matrix, so it does not alter the generated 46-package closure. It is a supplemental verification-only import required because Aeronet mandates all-target Clippy and its examples activate Bevy-Egui.
- First relevant errors: the published graph resolved Bevy 0.19 and WGPU types 29; after rewiring, extraction APIs required an app marker, temporary render entities required explicit construction, projection and pipeline descriptors had changed, and the paint-callback example lacked WGPU 30 pipeline constants.
- Old API: registry Bevy 0.19, WGPU types 29, unmarked extracted resources, unit-style `TemporaryRenderEntity`, `Mat4::orthographic_rh`, and pipeline states without `constants`.
- New API: repository-relative Bevy `0.20.0-dev` paths, WGPU types 30, `RenderApp`-marked extraction, `TemporaryRenderEntity::default()`, `bevy_math::proj::orthographic`, and explicit empty pipeline-constant vectors in both library and example descriptors.
- Semantic risk: low to moderate. Rendering still uses the same Egui texture/resource ownership and empty specialization-constant behavior; the full library, tests, and examples compile together.
- Toolchain note: Jackdaw's pinned nightly Cargo uses a new per-unit artifact layout that reproducibly lost dependency metadata for this standalone all-target graph. Bevy-Egui declares Rust 1.95 and local Bevy requires 1.96, so its reproducible gate intentionally uses stable 1.96 with an isolated target directory. Stable crossed the failing Bevy UI crates and exposed only the two genuine example API errors above.
- Files changed: the complete Bevy-Egui manifest/lockfile, render extraction and pipeline source, `examples/paint_callback.rs`, Aeronet's local dependency edge, workspace exclusion, provenance, inventory support, and orchestration.
- Verification: stable 1.96 `cargo check --offline --locked --all-targets --all-features` exits 0; the standalone lock contains zero `0.19.0` entries; metadata contains 62 official local Bevy identities and zero Bevy 0.19 packages; Aeronet's mandated Clippy gate exits 0 with this package local.

### Bevy Enhanced Input — mouse scroll conversion

- Package: `bevy_enhanced_input 0.26.0`
- Target and features: library, features activated by Jackdaw's locked graph
- Command: `cargo check --offline --locked --manifest-path editor/jackdaw/Cargo.toml -p bevy_enhanced_input`
- First relevant error: `MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR` no longer exists.
- Root cause: Bevy replaced the fixed 100-pixel constant with the configurable `MouseScrollPixelsPerLine` resource and `AccumulatedMouseScroll::to_lines` conversion API.
- Old API: divide pixel deltas by `MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR`.
- New API: read optional `MouseScrollPixelsPerLine`, use Bevy's default when input resources were constructed manually, and call `AccumulatedMouseScroll::to_lines`.
- Semantic risk: platform or user-configured scroll ratios now affect enhanced-input values. This is the intended Bevy-main behavior and is more accurate than the old fixed ratio.
- Files changed: `vendor/bevy-coupled/bevy-enhanced-input/src/context/input_reader.rs`.
- Focused test: `mouse_wheel_pixels_use_configured_conversion` asserts 100 pixels becomes two lines with a configured 50-pixel ratio.
- Verification: focused `cargo check` exits 0; `cargo test --offline --locked --manifest-path editor/jackdaw/vendor/bevy-coupled/bevy-enhanced-input/Cargo.toml mouse_wheel_pixels_use_configured_conversion --lib` passes 1 test; all-feature library Clippy exits 0.

### Bevy Replicon — renamed collections and change-detection helper

- Package: `bevy_replicon 0.41.1`
- Target and features: library, features activated by Jackdaw's locked graph; standalone default-feature library tests; all-feature library Clippy.
- Command: `cargo check --offline --locked --manifest-path editor/jackdaw/Cargo.toml -p bevy_replicon`.
- First relevant diagnostics: deprecation warnings for `TypeIdMap` and `DetectChangesMut::set_if_neq`.
- Root cause: Bevy main renamed the deterministic type-ID map aliases to describe their hash/index implementation and renamed the conditional mutation helper.
- Old API: `TypeIdMap`, iteration-dependent storage using the old alias, and `set_if_neq`.
- New API: `TypeIdIndexMap` where iteration order is observable and `set_if_different` for conditional mutation.
- Semantic risk: low. The index-map alias preserves deterministic iteration; the mutation helper retains change-tick behavior.
- Files changed: `vendor/bevy-coupled/bevy-replicon/src/server/visibility/registry.rs`, `src/shared/replication/storage.rs`, and `src/test_app.rs`.
- Verification: focused check exits 0; standalone library suite passes 135/135 tests; all-feature library Clippy exits 0.

### Bevy Monitors — no source migration required

- Package: `bevy_monitors 0.3.0`.
- Old and new API: the package's Bevy 0.19-facing source remains source-compatible with local Bevy 0.20.0-dev after manifest rewiring.
- Semantic risk: low; no runtime behavior or public API was changed.
- Files changed: standalone lockfile only, to record the local Bevy graph.
- Verification: focused check exits 0; standalone library suite passes 7/7 tests; all-feature library Clippy exits 0.

### Bevy Heavy — no source migration required

- Package: `bevy_heavy 0.5.0`.
- Old and new API: the mass-property implementation remains source-compatible with local Bevy 0.20.0-dev and the migrated glam 0.33 matrix layer.
- Semantic risk: low; no runtime behavior or public API was changed.
- Files changed: standalone lockfile only, to record the local Bevy and vendored Glam Matrix Extras graph.
- Verification: standalone default and all-feature checks exit 0; all-feature library suite passes 14/14 tests; all-feature no-deps library Clippy exits 0.

### Glam Matrix Extras — glam 0.33 and reflection feature migration

- Package: `glam_matrix_extras 0.3.0`
- Target and features: Jackdaw's locked feature graph, standalone default features (`std`, `f32`, `f64`) with the upstream test-only `approx` feature, and all-feature library Clippy.
- Commands: `cargo check --offline --locked --manifest-path editor/jackdaw/Cargo.toml -p glam_matrix_extras`; `cargo test --offline --locked --manifest-path editor/jackdaw/vendor/bevy-coupled/glam-matrix-extras/Cargo.toml --lib`.
- First relevant errors: 88 reflection trait errors with glam 0.32; after aligning glam, standalone tests could not import glam's double-precision types and assignment implementations resolved operators through `&&mut` receivers.
- Root cause: local Bevy reflects glam 0.33 types, glam 0.33 gates double-precision types behind its new `f64` feature, current Rust method resolution requires an explicit copied receiver in these generic assignment implementations, and `ReflectDefault` is now exported through `bevy_reflect::prelude` rather than the crate root.
- Old API: glam 0.32, implicit availability of `DMat*`/`DVec*`, implicit assignment receiver coercion, and root-level `ReflectDefault`.
- New API: glam 0.33.2, `f64 = ["glam/f64"]`, explicit `(*self)` assignment receivers, and conditionally imported `bevy_reflect::prelude::ReflectDefault` plus serialization type data.
- Semantic risk: low to moderate. Numeric operations are unchanged, but the full f32/f64 suite is required to guard operator and conversion behavior.
- Files changed: both manifests and the rectangular/symmetric matrix implementations under `vendor/bevy-coupled/glam-matrix-extras/src`.
- Verification: focused check exits 0; standalone library suite passes 21/21 tests; all-feature library Clippy exits 0.

### Bevy Transform Interpolation — no source migration required

- Package: `bevy_transform_interpolation 0.5.0`.
- Old and new API: the package's Bevy 0.19-facing source remains source-compatible with local Bevy 0.20.0-dev after manifest rewiring.
- Semantic risk: low; no runtime behavior or public API was changed.
- Files changed: standalone lockfile only, to record the local Bevy graph.
- Verification: focused check exits 0; the standalone library test target compiles and runs with zero defined tests; all-feature library Clippy exits 0.

### Leafwing Input Manager — reflected trait-object registration

- Package: `leafwing-input-manager 0.21.0`
- Target and features: Jackdaw's locked feature graph, standalone default-feature library tests, and all-feature library Clippy.
- Command: `cargo check --offline --lib` from `vendor/bevy-coupled/leafwing-input-manager`.
- First relevant error: unresolved import `bevy::reflect::FromType` in the custom processor and user-input reflection modules.
- Root cause: Bevy main replaced the `FromType` type-data construction API with the generic `CreateTypeData<T, Input>` trait.
- Old API: `FromType::<Self>::from_type()` for `ReflectDeserialize`, `ReflectFromPtr`, and `ReflectSerialize` registrations.
- New API: explicit `<TypeData as CreateTypeData<Self>>::create_type_data(())` construction for each registration.
- Semantic risk: low. The same three type-data records are registered for every reflected trait-object family; explicit UFCS prevents inference from selecting the wrong record type.
- Files changed: `src/input_processing/dual_axis/custom.rs`, `src/input_processing/single_axis/custom.rs`, and `src/user_input/trait_reflection.rs`.
- Verification: focused check exits 0; standalone library suite passes 140 tests with zero failures and 3 pre-existing upstream ignores; all-feature library Clippy exits 0.

### Jackdaw core - fallible reflection conversion and BSN scene registration

- Subsystems: command snapshots and undo/redo, BSN patch application, reflected field reset, live scene registration, and Windows-portable emitted asset paths.
- First relevant errors: Bevy main changed `PartialReflect::to_dynamic` from an infallible clone into a fallible conversion, so existing snapshot, patch, and scene-registration call sites no longer type-checked. Windows BSN output also exposed backslash-separated relative asset paths.
- Root cause: opaque reflected values may not provide the cloning behavior required to construct a dynamic representation. Scene registration only needs to serialize the borrowed reflected component, while undo snapshots require a complete clone by definition.
- Old API: direct `to_dynamic()` calls returning `Box<dyn PartialReflect>` at every call site, including an unnecessary clone before `component_to_bsn_patch`; platform-native separators in emitted BSN asset paths.
- New API: live BSN patching treats conversion failure as a non-destructive skipped update with an actionable warning; snapshot rebuilding makes the complete-clone invariant explicit and panics with the reflected type path if it is violated; scene registration serializes borrowed reflected components directly and preserves the existing patch-upsert behavior; reset-to-default applies the borrowed default field directly; emitted paths normalize `\\` to `/` on Windows.
- Semantic risk: moderate. A non-cloneable live component is now left untouched rather than partially overwritten, while undo snapshots still fail loudly because an incomplete snapshot would corrupt history. Direct scene serialization removes a clone without changing the generated patch or node ownership.
- Files changed: `crates/jackdaw_bsn/src/apply.rs`, `crates/jackdaw_bsn/src/document/from_reflect.rs`, `src/commands.rs`, and `src/scene_io/registration.rs`.
- Verification: pinned-nightly `cargo check --offline --locked` for the default Jackdaw graph exits 0; the `jackdaw_bsn` test executable passes 53/53 tests; test targets for `jackdaw_bsn`, `jackdaw_feathers`, `jackdaw_panels`, `jackdaw_ui`, and `bevy_window_chrome` compile with `cargo check --tests`.
- Unresolved concerns: none in the reflection conversion itself. Full-workspace runtime behavior remains covered by the final editor launch gate.

### Jackdaw UI and Feathers - headless widgets, picking hover, and text input

- Subsystems: window chrome, Feathers buttons/checkboxes/dialogs/panel headers/text editing, dock and workspace tabs, editor header controls, scene tabs, menus, remote controls, hierarchy filtering, and material/UI support.
- First relevant errors: Bevy main removed the legacy mutable `Interaction`-driven button path, renamed `EditableTextInputPlugin`, moved text scrolling state into `EditableText::viewport`, and models pressed state with a transient marker.
- Root cause: Bevy main's UI widget layer is event-driven. `ButtonPlugin` translates pointer or focused keyboard input into `Activate`, `Hovered` tracks an entity and its descendants, and `Pressed` is present only for the active press.
- Old API: `Button`/`Interaction` bundles from the legacy UI API, `Changed<Interaction>` polling, `Interaction::Pressed` timing, `EditableTextInputPlugin`, and `TextScroll` offsets.
- New API: defensively install `ButtonPlugin`; add `bevy_ui_widgets::Button`; add `ActivateOnPress` wherever the old action fired on `Interaction::Pressed`; consume `Activate` observers; use `Hovered` and `Pressed` for visuals; use propagation-aware pointer observers for backdrop, sidebar, drag, and popup behavior; install `TextInputPlugin`; read `EditableText::viewport.offset`.
- Semantic risk: moderate. The migration preserves press-versus-release timing explicitly, and nested close buttons continue to stop pointer propagation before parent tabs activate. Bevy's UI picking backend hit-tests visible nodes without requiring a `Pickable` marker, so direct pointer observers on add buttons, backdrops, and drag grips remain reachable.
- Files changed: `crates/bevy_window_chrome`, `crates/jackdaw_feathers`, `crates/jackdaw_panels`, `crates/jackdaw_ui`, and the affected editor UI modules under `src/`.
- Verification: default Jackdaw check exits 0; all test-only code for the five directly changed support crates compiles; direct nightly `rustfmt --check --edition 2024` passes for every changed Rust file. Bevy main's local UI-picking and button implementations were reviewed to verify default node hit-testing, ancestor hover state, pointer propagation, and `ActivateOnPress` behavior.
- Unresolved concerns: graphical interaction and rendering still require the final Windows editor launch. The exact whole-workspace formatting command now passes through the root Windows-safe, workspace-bounded formatter; vendored upstream workspaces remain excluded.

### Jackdaw Windows toolchain and native build path

- Subsystems: workspace toolchain selection, CI/release toolchain identity, SDK tests, CSG native compilation, and Windows linking.
- Old setup: the imported workspace pinned `nightly-2026-03-05`, which is unavailable for the frozen Bevy main Rust requirements in this environment; native Manifold configuration selected an unavailable NMake generator; SDK tests assumed Unix-only executable naming.
- New setup: pin `nightly-2026-07-31`, use its Cargo/Rustc/Rustdoc explicitly, import the Visual Studio x64 developer environment, select the available Ninja generator, route linking directly through `rust-lld` without forwarding the GCC-only `-fuse-ld` option, serialize full Windows Cargo graphs to stay within the host commit limit, and make SDK assertions honor Windows executable suffixes.
- Semantic risk: low. These changes select build tools and portable fixture names without altering editor behavior. The local `.cargo/config.toml` remains authoritative for `rust-lld` linking.
- Files changed: `rust-toolchain.toml`, CI/release configuration, project-build and SDK tests, and related toolchain documentation from the migration checkpoints.
- Verification: pinned Cargo and rustc report `1.99.0-nightly`; the default workspace check completes after building the Manifold native kernel; the portable SDK test targets compile. Root orchestration imports Visual Studio's x64 developer environment, discovers Ninja, and compiles warning-free on the pinned Bevy root toolchain. A first parallel all-target check reproduced Windows error 1455 and rustc allocator failures; the serialized retry is the required replacement evidence.

### Root orchestration, resumable verification, and pinned workspace formatting

- Subsystems: root `cargo studio` interface, toolchain selection, Windows native environment discovery, evidence logging, inventory/provenance validation, Cargo metadata and dependency-source auditing, checks/tests/builds, controlled runtime smoke launch, and formatting.
- First relevant failures: Cargo's external `cargo-fmt --all` exceeded Windows' command-line limit; Aeronet's AWS-LC build selected incompatible Visual Studio/NASM behavior; and the exhaustive focused check exceeded the execution host's one-hour command ceiling after 28 successful packages.
- Root cause: Cargo fmt follows local path dependencies outside the selected workspace; AWS-LC's probe and batch wrapper do not account for Visual Studio 18 C11 defaults or spaced verbatim paths; and one monolithic cold verification command outlived the host even though Cargo remained healthy.
- Old behavior: an unbounded single rustfmt process crossed the nested workspace boundary; native Cargo commands depended on the caller already having a Visual Studio environment; final source identity was inspected through ad hoc commands.
- New behavior: root Rust orchestration selects the pinned toolchains, imports the Visual Studio x64 environment, selects Ninja, retains tee'd logs, scopes and chunks formatting, configures AWS-LC's C11/no-ASM debug path with a short relative target, supports an exact `--resume-focused-from` checkpoint after the four Jackdaw matrices pass, verifies supplemental Bevy-Egui with stable Cargo, and enforces all inventory/provenance/source-identity invariants.
- Semantic risk: low. The orchestration changes process selection and validation, not editor behavior. Changing the Jackdaw nightly required one whole-workspace rustfmt normalization (291 Rust files, 752 insertions and 747 deletions, primarily deterministic import ordering and line wrapping); vendored upstream sources were not formatted.
- Files changed: root `.cargo/config.toml`, root `rust-toolchain.toml`, `tools/bevy-studio-xtask`, the deterministic inventory tool/output, and the integration documentation. Jackdaw workspace Rust sources received formatting-only changes.
- Verification: root xtask check and strict Clippy pass; inventory refresh/check and 12-record provenance validation pass; all four metadata matrices resolve locked and offline; `cargo studio audit-bevy-sources` finds 66 local official Bevy packages and all 46 classified packages local in every matrix; all 46 focused packages pass across the initial and resumed evidence; supplemental Bevy-Egui and Aeronet's mandated Clippy pass; the exact mandated format command passes for 183 target roots without OS error 206.
- Unresolved concerns: the final check/test/build matrix and graphical runtime launch remain to be recorded in `verification-report.md`.

## Dependency ownership migration

- Old source model: official Bevy packages and the 46 classified packages resolved from crates.io/Git at the pinned Jackdaw lock.
- New source model: official Bevy packages resolve to repository-local Bevy `0.20.0-dev`; all classified packages and the one verification-only supplemental package resolve beneath `vendor/bevy-coupled`.
- Semantics preserved: aliases, features, optionality, default-feature settings, dependency kind, and target conditions.
- Verification: default, no-default-features, and all-features metadata resolve locked with zero external classified sources and zero official Bevy 0.19 packages.

## Temporary feature gates

None.
