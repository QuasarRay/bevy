//! Reproducible orchestration for the vendored Jackdaw editor.

#![expect(
    clippy::print_stdout,
    reason = "this command-line tool reports command context and audit findings"
)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, ensure, Context, Result};
use clap::{Parser, Subcommand};
use pathdiff::diff_paths;
use serde::Deserialize;
use toml_edit::{value, Array, DocumentMut, InlineTable, Item, Table, Value};
use walkdir::{DirEntry, WalkDir};

mod orchestration;

const JACKDAW_RELATIVE: &str = "editor/jackdaw";
const INVENTORY_RELATIVE: &str = "editor/jackdaw/docs/bevy-coupled-inventory.json";
const LOCAL_BEVY_VERSION: &str = "0.20.0-dev";
const VENDOR_WORKSPACE_EXCLUDES: &[&str] = &[
    "vendor/bevy-coupled/aeronet",
    "vendor/bevy-coupled/avian/packages/avian2d",
    "vendor/bevy-coupled/avian/packages/avian3d",
    "vendor/bevy-coupled/bevy-enhanced-input",
    "vendor/bevy-coupled/bevy-heavy",
    "vendor/bevy-coupled/bevy-monitors",
    "vendor/bevy-coupled/bevy-replicon",
    "vendor/bevy-coupled/bevy-transform-interpolation",
    "vendor/bevy-coupled/glam-matrix-extras",
    "vendor/bevy-coupled/leafwing-input-manager",
    "vendor/bevy-coupled/lightyear",
    "vendor/bevy-coupled/rerecast",
];

#[derive(Debug, Parser)]
#[command(about, version)]
struct Cli {
    /// Rust toolchain used for the nested Jackdaw workspace.
    #[arg(long, global = true)]
    toolchain: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate the frozen repository, toolchains, and Windows native prerequisites.
    Preflight,
    /// Refresh or validate the checked-in Bevy-coupled inventory.
    Inventory {
        /// Fail instead of writing when generated inventory fields have drifted.
        #[arg(long)]
        check: bool,
    },
    /// Validate imported-source provenance, package mappings, and licenses.
    Provenance,
    /// Resolve and retain Cargo metadata for every supported Jackdaw matrix.
    Metadata,
    /// Print the supported graph's duplicate dependency tree.
    Tree,
    /// Enforce local Bevy and vendored Bevy-coupled package identity.
    AuditBevySources,
    /// Check the default editor graph.
    Check,
    /// Check all required Jackdaw feature/workspace matrices and vendored packages.
    CheckAll {
        /// Resume the focused vendored phase at this exact package after matrices passed.
        #[arg(long)]
        resume_focused_from: Option<String>,
    },
    /// Run the required all-feature Jackdaw workspace tests.
    Test,
    /// Build the editor, CLI, runner, compiler wrapper, SDK, and PIE fixtures.
    Build,
    /// Launch the Jackdaw editor against the repository-local Bevy snapshot.
    Editor {
        /// Stop a successful smoke launch after this many seconds.
        #[arg(long)]
        smoke_seconds: Option<u64>,
        /// Arguments forwarded to the editor binary.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run all non-interactive integration gates. Runtime launch remains explicit.
    Verify,
    /// Windows-safe, workspace-bounded rustfmt orchestration used by the `fmt` alias.
    Format {
        #[arg(long)]
        manifest_path: Option<PathBuf>,
        #[arg(long)]
        all: bool,
        #[arg(short = 'p', long = "package")]
        packages: Vec<String>,
        #[arg(short, long)]
        quiet: bool,
        #[arg(short, long)]
        verbose: bool,
        #[arg(last = true)]
        rustfmt_args: Vec<String>,
    },
    /// Rewrite supported dependency declarations to repository-local sources.
    #[command(hide = true)]
    Rewire {
        /// Persist the deterministic rewrite. Without this flag, only report drift.
        #[arg(long)]
        write: bool,
    },
}

#[derive(Debug, Deserialize)]
struct Inventory {
    official_bevy_packages: Vec<String>,
    packages: Vec<InventoryPackage>,
}

#[derive(Clone, Debug, Deserialize)]
struct InventoryPackage {
    name: String,
    version: String,
    source: Option<String>,
    repository: Option<String>,
    local_source_path: Option<String>,
}

#[derive(Clone, Debug)]
enum LocalKind {
    OfficialBevy,
    Vendored {
        source: Option<String>,
        repository: Option<String>,
    },
}

#[derive(Clone, Debug)]
struct LocalPackage {
    name: String,
    version: String,
    manifest: PathBuf,
    kind: LocalKind,
}

#[derive(Debug, Default)]
struct RewriteReport {
    changed_files: BTreeSet<PathBuf>,
    rewired: Vec<RewiredDependency>,
    internal_versions: usize,
    workspace_version: bool,
    workspace_exclude: bool,
    patch_entries: usize,
}

#[derive(Debug)]
struct RewiredDependency {
    manifest: PathBuf,
    table: String,
    alias: String,
    package: String,
    path: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = repository_root()?;

    match cli.command {
        Command::Preflight => orchestration::preflight(&root, cli.toolchain.as_deref()),
        Command::Inventory { check } => {
            orchestration::inventory(&root, cli.toolchain.as_deref(), check)
        }
        Command::Provenance => orchestration::provenance(&root),
        Command::Metadata => orchestration::metadata(&root, cli.toolchain.as_deref()),
        Command::Tree => orchestration::tree(&root, cli.toolchain.as_deref()),
        Command::AuditBevySources => {
            orchestration::audit_bevy_sources(&root, cli.toolchain.as_deref())
        }
        Command::Check => orchestration::check(&root, cli.toolchain.as_deref()),
        Command::CheckAll {
            resume_focused_from,
        } => orchestration::check_all(
            &root,
            cli.toolchain.as_deref(),
            resume_focused_from.as_deref(),
        ),
        Command::Test => orchestration::test(&root, cli.toolchain.as_deref()),
        Command::Build => orchestration::build(&root, cli.toolchain.as_deref()),
        Command::Editor {
            smoke_seconds,
            args,
        } => orchestration::editor(&root, cli.toolchain.as_deref(), smoke_seconds, &args),
        Command::Verify => orchestration::verify(&root, cli.toolchain.as_deref()),
        Command::Format {
            manifest_path,
            all,
            packages,
            quiet,
            verbose,
            rustfmt_args,
        } => orchestration::format(
            &root,
            cli.toolchain.as_deref(),
            manifest_path.as_deref(),
            all,
            &packages,
            quiet,
            verbose,
            &rustfmt_args,
        ),
        Command::Rewire { write } => rewire(&root, write),
    }
}

fn repository_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .ancestors()
        .nth(2)
        .context("bevy-studio-xtask must remain under tools/bevy-studio-xtask")?;
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve repository root {}", root.display()))?;
    ensure!(
        root.join("Cargo.toml").is_file() && root.join(JACKDAW_RELATIVE).is_dir(),
        "{} is not the integrated Bevy repository",
        root.display()
    );
    Ok(root)
}

fn rewire(root: &Path, write_changes: bool) -> Result<()> {
    let inventory_path = root.join(INVENTORY_RELATIVE);
    let inventory: Inventory = serde_json::from_slice(
        &fs::read(&inventory_path)
            .with_context(|| format!("failed to read {}", inventory_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", inventory_path.display()))?;

    let packages = local_packages(root, &inventory)?;
    let jackdaw_root = root.join(JACKDAW_RELATIVE);
    let manifests = cargo_manifests(&jackdaw_root)?;
    let mut report = RewriteReport::default();
    let mut rewritten_documents = BTreeMap::new();
    let mut used_official = BTreeSet::new();

    for manifest in manifests {
        let original = fs::read_to_string(&manifest)
            .with_context(|| format!("failed to read {}", manifest.display()))?;
        let mut document = original
            .parse::<DocumentMut>()
            .with_context(|| format!("failed to parse {}", manifest.display()))?;
        let is_jackdaw_root = manifest == jackdaw_root.join("Cargo.toml");
        let is_jackdaw_source = manifest.starts_with(jackdaw_root.join("crates"))
            || manifest.starts_with(jackdaw_root.join("examples"))
            || manifest.starts_with(jackdaw_root.join("tests"))
            || is_jackdaw_root;
        let before = report_position(&report);

        rewrite_document_dependencies(
            root,
            &manifest,
            &mut document,
            &packages,
            is_jackdaw_source,
            &mut used_official,
            &mut report,
        )?;

        if is_jackdaw_root {
            rewrite_jackdaw_workspace(&mut document, &mut report)?;
        }

        if report_position(&report) != before {
            let rendered = render_preserving_newlines(&document, &original);
            report.changed_files.insert(manifest.clone());
            rewritten_documents.insert(manifest, rendered);
        }
    }

    let jackdaw_manifest = jackdaw_root.join("Cargo.toml");
    let root_source = match rewritten_documents.get(&jackdaw_manifest) {
        Some(rewritten) => rewritten.clone(),
        None => fs::read_to_string(&jackdaw_manifest)
            .with_context(|| format!("failed to reread {}", jackdaw_manifest.display()))?,
    };
    let root_document = root_source
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to reparse {}", jackdaw_manifest.display()))?;
    let mut root_document = root_document;
    add_patch_guard(
        root,
        &jackdaw_manifest,
        &mut root_document,
        &packages,
        &used_official,
        &mut report,
    )?;
    let root_original = fs::read_to_string(&jackdaw_manifest)
        .with_context(|| format!("failed to reread {}", jackdaw_manifest.display()))?;
    let root_rendered = render_preserving_newlines(&root_document, &root_original);
    if root_rendered != root_original {
        report.changed_files.insert(jackdaw_manifest.clone());
        rewritten_documents.insert(jackdaw_manifest, root_rendered);
    }

    print_rewrite_report(root, &report, write_changes);

    if write_changes {
        for (path, contents) in rewritten_documents {
            fs::write(&path, contents)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        println!("rewire: wrote {} manifest(s)", report.changed_files.len());
        return Ok(());
    }

    if !report.changed_files.is_empty() {
        bail!(
            "manifest ownership drift detected in {} file(s); run `cargo studio rewire --write`",
            report.changed_files.len()
        );
    }

    println!("rewire: manifests already match the local-source policy");
    Ok(())
}

fn local_packages(root: &Path, inventory: &Inventory) -> Result<BTreeMap<String, LocalPackage>> {
    let official_names: BTreeSet<_> = inventory.official_bevy_packages.iter().cloned().collect();
    let mut packages = BTreeMap::new();

    for manifest in bevy_manifests(root)? {
        let text = fs::read_to_string(&manifest)
            .with_context(|| format!("failed to read {}", manifest.display()))?;
        let document = text
            .parse::<DocumentMut>()
            .with_context(|| format!("failed to parse {}", manifest.display()))?;
        let Some(package_table) = document.get("package").and_then(Item::as_table) else {
            continue;
        };
        let Some(name) = package_table.get("name").and_then(Item::as_str) else {
            continue;
        };
        if !official_names.contains(name) {
            continue;
        }
        let version = package_table
            .get("version")
            .and_then(Item::as_str)
            .unwrap_or(LOCAL_BEVY_VERSION);
        // The root metadata also contains example packages whose names happen to
        // begin with `bevy_` (for example `bevy_city`). They are not distributable
        // Bevy crates and intentionally retain their own versions.
        if version != LOCAL_BEVY_VERSION {
            continue;
        }
        let package = LocalPackage {
            name: name.to_owned(),
            version: version.to_owned(),
            manifest: manifest.clone(),
            kind: LocalKind::OfficialBevy,
        };
        ensure!(
            packages.insert(name.to_owned(), package).is_none(),
            "duplicate official Bevy package {name}"
        );
    }

    for candidate in &inventory.packages {
        let relative = candidate.local_source_path.as_deref().with_context(|| {
            format!(
                "inventory package {} {} has no local_source_path",
                candidate.name, candidate.version
            )
        })?;
        let manifest = root.join(relative);
        ensure!(
            manifest.is_file(),
            "inventory package {} points to missing manifest {}",
            candidate.name,
            manifest.display()
        );
        let package = LocalPackage {
            name: candidate.name.clone(),
            version: candidate.version.clone(),
            manifest,
            kind: LocalKind::Vendored {
                source: candidate.source.clone(),
                repository: candidate.repository.clone(),
            },
        };
        match packages.insert(candidate.name.clone(), package) {
            None => {}
            Some(previous) => {
                bail!(
                    "duplicate local package mapping for {}: {} and {}",
                    candidate.name,
                    previous.manifest.display(),
                    relative
                );
            }
        }
    }

    Ok(packages)
}

fn bevy_manifests(root: &Path) -> Result<Vec<PathBuf>> {
    let mut manifests = vec![root.join("Cargo.toml")];
    let crates = root.join("crates");
    for entry in
        fs::read_dir(&crates).with_context(|| format!("failed to read {}", crates.display()))?
    {
        let entry = entry?;
        let manifest = entry.path().join("Cargo.toml");
        if manifest.is_file() {
            manifests.push(manifest);
        }
    }

    for extra in [
        "examples/mobile",
        "examples/large_scenes/bevy_city",
        "crates/bevy_derive/compile_fail",
        "crates/bevy_ecs/compile_fail",
        "crates/bevy_reflect/compile_fail",
    ] {
        let manifest = root.join(extra).join("Cargo.toml");
        if manifest.is_file() {
            manifests.push(manifest);
        }
    }
    manifests.sort();
    manifests.dedup();
    Ok(manifests)
}

fn cargo_manifests(root: &Path) -> Result<Vec<PathBuf>> {
    let mut manifests = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(should_descend);
    for entry in walker {
        let entry = entry.with_context(|| format!("failed while walking {}", root.display()))?;
        if entry.file_type().is_file() && entry.file_name() == "Cargo.toml" {
            manifests.push(entry.into_path());
        }
    }
    manifests.sort();
    Ok(manifests)
}

fn should_descend(entry: &DirEntry) -> bool {
    !matches!(entry.file_name().to_str(), Some("target" | ".git" | ".jj"))
}

fn rewrite_document_dependencies(
    root: &Path,
    manifest: &Path,
    document: &mut DocumentMut,
    packages: &BTreeMap<String, LocalPackage>,
    update_internal_versions: bool,
    used_official: &mut BTreeSet<String>,
    report: &mut RewriteReport,
) -> Result<()> {
    for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(item) = document.get_mut(table_name) {
            rewrite_dependency_table(
                root,
                manifest,
                table_name,
                item,
                packages,
                update_internal_versions,
                used_official,
                report,
            )?;
        }
    }

    if let Some(workspace) = document.get_mut("workspace").and_then(Item::as_table_mut)
        && let Some(item) = workspace.get_mut("dependencies")
    {
        rewrite_dependency_table(
            root,
            manifest,
            "workspace.dependencies",
            item,
            packages,
            update_internal_versions,
            used_official,
            report,
        )?;
    }

    if let Some(targets) = document.get_mut("target").and_then(Item::as_table_mut) {
        for (target_name, target_item) in targets.iter_mut() {
            let Some(target_table) = target_item.as_table_mut() else {
                continue;
            };
            for dependency_kind in ["dependencies", "dev-dependencies", "build-dependencies"] {
                let Some(item) = target_table.get_mut(dependency_kind) else {
                    continue;
                };
                let label = format!("target.{}.{dependency_kind}", target_name.get());
                rewrite_dependency_table(
                    root,
                    manifest,
                    &label,
                    item,
                    packages,
                    update_internal_versions,
                    used_official,
                    report,
                )?;
            }
        }
    }

    if update_internal_versions {
        update_package_version(document, report);
    }

    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "dependency rewrites require explicit path, policy, and report state"
)]
fn rewrite_dependency_table(
    root: &Path,
    manifest: &Path,
    table_label: &str,
    item: &mut Item,
    packages: &BTreeMap<String, LocalPackage>,
    update_internal_versions: bool,
    used_official: &mut BTreeSet<String>,
    report: &mut RewriteReport,
) -> Result<()> {
    let Some(table) = item.as_table_like_mut() else {
        return Ok(());
    };
    let aliases: Vec<String> = table.iter().map(|(key, _)| key.to_owned()).collect();
    for alias in aliases {
        let dependency = table
            .get_mut(&alias)
            .with_context(|| format!("dependency {alias} disappeared while rewriting"))?;
        if dependency_inherits_workspace(dependency) {
            continue;
        }
        let package_name = dependency_package_name(&alias, dependency).to_owned();

        if update_internal_versions
            && dependency_has_version(dependency)
            && dependency_targets_jackdaw_source(root, manifest, dependency)?
            && set_dependency_version(dependency, LOCAL_BEVY_VERSION)
        {
            report.internal_versions += 1;
        }

        let Some(local) = packages.get(&package_name) else {
            continue;
        };
        if matches!(local.kind, LocalKind::OfficialBevy) {
            used_official.insert(package_name.clone());
        }
        let manifest_dir = manifest
            .parent()
            .with_context(|| format!("manifest {} has no parent", manifest.display()))?;
        let target_dir = local
            .manifest
            .parent()
            .with_context(|| format!("manifest {} has no parent", local.manifest.display()))?;
        let relative = relative_path(target_dir, manifest_dir)?;
        let changed = set_local_dependency(dependency, &relative, &local.version)?;
        if changed {
            report.rewired.push(RewiredDependency {
                manifest: manifest
                    .strip_prefix(root)
                    .unwrap_or(manifest)
                    .to_path_buf(),
                table: table_label.to_owned(),
                alias,
                package: local.name.clone(),
                path: relative,
            });
        }
    }
    Ok(())
}

fn dependency_inherits_workspace(item: &Item) -> bool {
    match item {
        Item::Value(Value::InlineTable(table)) => {
            table.get("workspace").and_then(Value::as_bool) == Some(true)
        }
        Item::Table(table) => table.get("workspace").and_then(Item::as_bool) == Some(true),
        _ => false,
    }
}

fn dependency_package_name<'a>(alias: &'a str, item: &'a Item) -> &'a str {
    match item {
        Item::Value(Value::InlineTable(table)) => table
            .get("package")
            .and_then(Value::as_str)
            .unwrap_or(alias),
        Item::Table(table) => table.get("package").and_then(Item::as_str).unwrap_or(alias),
        _ => alias,
    }
}

fn dependency_has_version(item: &Item) -> bool {
    match item {
        Item::Value(Value::InlineTable(table)) => table.contains_key("version"),
        Item::Table(table) => table.contains_key("version"),
        Item::Value(Value::String(_)) => true,
        _ => false,
    }
}

fn dependency_targets_jackdaw_source(
    root: &Path,
    manifest: &Path,
    dependency: &Item,
) -> Result<bool> {
    let relative = match dependency {
        Item::Value(Value::InlineTable(table)) => table.get("path").and_then(Value::as_str),
        Item::Table(table) => table.get("path").and_then(Item::as_str),
        _ => None,
    };
    let Some(relative) = relative else {
        return Ok(false);
    };
    let manifest_dir = manifest
        .parent()
        .with_context(|| format!("manifest {} has no parent", manifest.display()))?;
    let target = manifest_dir
        .join(relative)
        .canonicalize()
        .with_context(|| {
            format!(
                "failed to resolve internal dependency path {relative} from {}",
                manifest.display()
            )
        })?;
    let jackdaw = root.join(JACKDAW_RELATIVE).canonicalize()?;
    let vendor = jackdaw.join("vendor");
    Ok(target.starts_with(&jackdaw) && !target.starts_with(vendor))
}

fn set_dependency_version(item: &mut Item, version: &str) -> bool {
    match item {
        Item::Value(Value::InlineTable(table)) => {
            let before = table.to_string();
            set_inline_value(table, "version", version);
            table.fmt();
            table.to_string() != before
        }
        Item::Table(table) => set_table_value(table, "version", version),
        _ => false,
    }
}

fn set_local_dependency(item: &mut Item, path: &str, version: &str) -> Result<bool> {
    let changed = match item {
        Item::Value(Value::String(_)) => {
            let mut table = InlineTable::new();
            table.insert("version", Value::from(version));
            table.insert("path", Value::from(path));
            *item = Item::Value(Value::InlineTable(table));
            true
        }
        Item::Value(Value::InlineTable(table)) => {
            let before = table.to_string();
            let mut changed = remove_source_keys_inline(table);
            changed |= set_inline_value(table, "path", path);
            changed |= set_inline_value(table, "version", version);
            table.fmt();
            changed || table.to_string() != before
        }
        Item::Table(table) => {
            let mut changed = remove_source_keys_table(table);
            changed |= set_table_value(table, "path", path);
            changed |= set_table_value(table, "version", version);
            changed
        }
        Item::None => bail!("dependency declaration cannot be empty"),
        Item::ArrayOfTables(_) | Item::Value(_) => {
            bail!("unsupported dependency declaration: {item:?}")
        }
    };
    Ok(changed)
}

fn remove_source_keys_inline(table: &mut InlineTable) -> bool {
    ["git", "branch", "tag", "rev", "registry"]
        .into_iter()
        .filter_map(|key| table.remove(key))
        .count()
        != 0
}

fn remove_source_keys_table(table: &mut Table) -> bool {
    ["git", "branch", "tag", "rev", "registry"]
        .into_iter()
        .filter_map(|key| table.remove(key))
        .count()
        != 0
}

fn set_inline_value(table: &mut InlineTable, key: &str, expected: &str) -> bool {
    if table.get(key).and_then(Value::as_str) == Some(expected) {
        return false;
    }
    table.insert(key, Value::from(expected));
    true
}

fn set_table_value(table: &mut Table, key: &str, expected: &str) -> bool {
    if table.get(key).and_then(Item::as_str) == Some(expected) {
        return false;
    }
    table.insert(key, value(expected));
    true
}

fn relative_path(target: &Path, base: &Path) -> Result<String> {
    let path = diff_paths(target, base).with_context(|| {
        format!(
            "cannot compute a relative path from {} to {}",
            base.display(),
            target.display()
        )
    })?;
    Ok(path.to_string_lossy().replace('\\', "/"))
}

fn update_package_version(document: &mut DocumentMut, report: &mut RewriteReport) {
    let Some(package) = document.get_mut("package").and_then(Item::as_table_mut) else {
        return;
    };
    let Some(name) = package.get("name").and_then(Item::as_str) else {
        return;
    };
    if !name.starts_with("jackdaw") {
        return;
    }
    let Some(version) = package.get_mut("version") else {
        return;
    };
    if version.as_str().is_some() && version.as_str() != Some(LOCAL_BEVY_VERSION) {
        *version = value(LOCAL_BEVY_VERSION);
        report.internal_versions += 1;
    }
}

fn rewrite_jackdaw_workspace(document: &mut DocumentMut, report: &mut RewriteReport) -> Result<()> {
    let workspace = document
        .get_mut("workspace")
        .and_then(Item::as_table_mut)
        .context("Jackdaw manifest has no [workspace] table")?;
    let package = workspace
        .get_mut("package")
        .and_then(Item::as_table_mut)
        .context("Jackdaw manifest has no [workspace.package] table")?;
    if package.get("version").and_then(Item::as_str) != Some(LOCAL_BEVY_VERSION) {
        package.insert("version", value(LOCAL_BEVY_VERSION));
        report.workspace_version = true;
    }

    let exclude = workspace.entry("exclude").or_insert(value(Array::new()));
    let array = exclude
        .as_array_mut()
        .context("Jackdaw workspace.exclude is not an array")?;
    array.retain(|item| {
        !matches!(
            item.as_str(),
            Some("vendor/bevy-coupled/*" | "vendor/bevy-coupled/**")
        )
    });
    for vendor_exclude in VENDOR_WORKSPACE_EXCLUDES {
        if !array
            .iter()
            .any(|item| item.as_str() == Some(*vendor_exclude))
        {
            array.push(*vendor_exclude);
            report.workspace_exclude = true;
        }
    }
    Ok(())
}

fn add_patch_guard(
    root: &Path,
    jackdaw_manifest: &Path,
    document: &mut DocumentMut,
    packages: &BTreeMap<String, LocalPackage>,
    used_official: &BTreeSet<String>,
    report: &mut RewriteReport,
) -> Result<()> {
    let manifest_dir = jackdaw_manifest
        .parent()
        .context("Jackdaw manifest has no parent")?;
    let patch = document
        .entry("patch")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .context("Jackdaw [patch] is not a table")?;
    patch.set_implicit(true);

    for package in packages.values() {
        let source = match &package.kind {
            LocalKind::OfficialBevy if used_official.contains(&package.name) => "crates-io",
            LocalKind::OfficialBevy => continue,
            LocalKind::Vendored { source, repository } => {
                if source
                    .as_deref()
                    .is_some_and(|source| source.starts_with("git+"))
                {
                    repository.as_deref().with_context(|| {
                        format!("Git package {} has no repository URL", package.name)
                    })?
                } else {
                    "crates-io"
                }
            }
        };
        let target_dir = package
            .manifest
            .parent()
            .with_context(|| format!("manifest {} has no parent", package.manifest.display()))?;
        let relative = relative_path(target_dir, manifest_dir)?;
        let source_table = patch
            .entry(source)
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .with_context(|| format!("Jackdaw patch source {source} is not a table"))?;
        let existing_matches = source_table
            .get(&package.name)
            .and_then(Item::as_inline_table)
            .and_then(|dependency| dependency.get("path"))
            .and_then(Value::as_str)
            == Some(relative.as_str());
        let mut dependency = InlineTable::new();
        dependency.insert("path", Value::from(relative));
        let expected = Item::Value(Value::InlineTable(dependency));
        if !existing_matches {
            source_table.insert(&package.name, expected);
            report.patch_entries += 1;
        }
    }

    // Prove every generated patch target remains repository-relative.
    ensure!(
        !document
            .to_string()
            .contains(&root.to_string_lossy().to_string()),
        "generated Jackdaw manifest contains an absolute repository path"
    );
    Ok(())
}

fn print_rewrite_report(root: &Path, report: &RewriteReport, writing: bool) {
    let mode = if writing { "write" } else { "check" };
    println!(
        "rewire ({mode}): {} dependency declaration(s)",
        report.rewired.len()
    );
    for dependency in &report.rewired {
        println!(
            "  {} [{}] {} (package {}) -> {}",
            dependency.manifest.display(),
            dependency.table,
            dependency.alias,
            dependency.package,
            dependency.path
        );
    }
    println!(
        "rewire ({mode}): {} internal version assertion(s), workspace-version={}, workspace-exclude={}, {} patch entry update(s)",
        report.internal_versions,
        report.workspace_version,
        report.workspace_exclude,
        report.patch_entries
    );
    println!(
        "rewire ({mode}): {} changed manifest(s)",
        report.changed_files.len()
    );
    for manifest in &report.changed_files {
        println!(
            "  {}",
            manifest.strip_prefix(root).unwrap_or(manifest).display()
        );
    }
}

fn report_position(report: &RewriteReport) -> (usize, usize, bool, bool) {
    (
        report.rewired.len(),
        report.internal_versions,
        report.workspace_version,
        report.workspace_exclude,
    )
}

fn render_preserving_newlines(document: &DocumentMut, original: &str) -> String {
    let rendered = document.to_string();
    if original.contains("\r\n") {
        rendered.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        rendered
    }
}
