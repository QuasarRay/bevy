//! Deterministic build, audit, verification, and launch orchestration.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use std::{
    env,
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command as ProcessCommand, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Value};
use walkdir::WalkDir;

const JACKDAW_RELATIVE: &str = "editor/jackdaw";
const INVENTORY_RELATIVE: &str = "editor/jackdaw/docs/bevy-coupled-inventory.json";
const INVENTORY_MARKDOWN_RELATIVE: &str = "editor/jackdaw/docs/bevy-coupled-inventory.md";
const PROVENANCE_RELATIVE: &str = "editor/jackdaw/vendor/bevy-coupled/provenance.toml";
const INVENTORY_TOOL_RELATIVE: &str = "editor/jackdaw/tools/bevy_coupled_inventory.py";
const VENDOR_RELATIVE: &str = "editor/jackdaw/vendor/bevy-coupled";
const EVIDENCE_RELATIVE: &str = "editor/jackdaw/target/studio-evidence";
const LOCAL_BEVY_VERSION: &str = "0.20.0-dev";
const FROZEN_BEVY_REVISION: &str = "25368b78ce5e9b15dc770cdf2af4595602cc8a7b";
const JACKDAW_REVISION: &str = "b21229553bd42c89c7798ca05d5b8f5f4bf806b3";
const ROOT_TOOLCHAIN: &str = "1.96.0";
const TARGET_PLATFORM: &str = "x86_64-pc-windows-msvc";

static LOG_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
struct Inventory {
    lockfile_sha256: String,
    official_bevy_packages: Vec<String>,
    packages: Vec<InventoryPackage>,
    #[serde(default)]
    upstreams: Vec<InventoryUpstream>,
    #[serde(default)]
    cycles: Vec<serde_json::Value>,
    #[serde(default)]
    excluded_candidates: Vec<serde_json::Value>,
    #[serde(default)]
    supplemental_upstreams: Vec<InventorySupplementalUpstream>,
}

#[derive(Debug, Deserialize)]
struct InventorySupplementalUpstream {
    slug: String,
    local_path: String,
    #[serde(default)]
    packages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct InventoryPackage {
    name: String,
    version: String,
    source: Option<String>,
    repository: Option<String>,
    local_source_path: Option<String>,
    upstream_slug: String,
    migration_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InventoryUpstream {
    slug: String,
    local_path: Option<String>,
    #[serde(default)]
    packages: Vec<InventoryUpstreamPackage>,
}

#[derive(Debug, Deserialize)]
struct InventoryUpstreamPackage {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<MetadataPackage>,
    #[serde(default)]
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    id: String,
    name: String,
    version: String,
    source: Option<String>,
    manifest_path: PathBuf,
    #[serde(default)]
    targets: Vec<MetadataTarget>,
}

#[derive(Debug, Deserialize)]
struct MetadataTarget {
    src_path: PathBuf,
    edition: String,
}

#[derive(Clone, Copy)]
struct MetadataMatrix {
    label: &'static str,
    feature_args: &'static [&'static str],
}

const METADATA_MATRICES: &[MetadataMatrix] = &[
    MetadataMatrix {
        label: "default",
        feature_args: &[],
    },
    MetadataMatrix {
        label: "no-default-features",
        feature_args: &["--no-default-features"],
    },
    MetadataMatrix {
        label: "all-features",
        feature_args: &["--all-features"],
    },
    MetadataMatrix {
        label: "workspace-all-features",
        // `cargo metadata` always describes the complete workspace and has no
        // `--workspace` flag. The audit interprets all workspace roots in this
        // matrix, while `--all-features` activates their complete feature set.
        feature_args: &["--all-features"],
    },
];

struct Studio {
    root: PathBuf,
    jackdaw: PathBuf,
    toolchain: String,
    root_cargo: PathBuf,
    root_rustc: PathBuf,
    root_rustdoc: PathBuf,
    cargo: PathBuf,
    rustc: PathBuf,
    rustdoc: PathBuf,
    rustfmt: PathBuf,
    native_env: BTreeMap<OsString, OsString>,
    evidence: PathBuf,
}

impl Studio {
    fn new(root: &Path, requested_toolchain: Option<&str>) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", root.display()))?;
        let jackdaw = root.join(JACKDAW_RELATIVE);
        let toolchain = match requested_toolchain {
            Some(toolchain) => toolchain.to_owned(),
            None => read_toolchain_channel(&jackdaw.join("rust-toolchain.toml"))?,
        };
        let root_cargo = rustup_which(ROOT_TOOLCHAIN, "cargo")?;
        let root_rustc = rustup_which(ROOT_TOOLCHAIN, "rustc")?;
        let root_rustdoc = rustup_which(ROOT_TOOLCHAIN, "rustdoc")?;
        let cargo = rustup_which(&toolchain, "cargo")?;
        let rustc = rustup_which(&toolchain, "rustc")?;
        let rustdoc = rustup_which(&toolchain, "rustdoc")?;
        let rustfmt = rustup_which(&toolchain, "rustfmt")?;
        let mut native_env = windows_native_environment()?;
        prepend_to_environment_path(
            &mut native_env,
            cargo
                .parent()
                .context("selected Cargo has no parent directory")?,
        )?;
        let evidence = root.join(EVIDENCE_RELATIVE);
        fs::create_dir_all(&evidence)
            .with_context(|| format!("failed to create {}", evidence.display()))?;
        Ok(Self {
            root,
            jackdaw,
            toolchain,
            root_cargo,
            root_rustc,
            root_rustdoc,
            cargo,
            rustc,
            rustdoc,
            rustfmt,
            native_env,
            evidence,
        })
    }

    fn cargo_command(&self, cwd: &Path, args: &[&str], native: bool) -> ProcessCommand {
        let mut command = ProcessCommand::new(&self.cargo);
        command.current_dir(cwd).args(args);
        if native {
            command.envs(self.native_env.iter());
            command.env("CMAKE_GENERATOR", "Ninja");
        }
        // Apply Rust selection after importing the developer environment so
        // workspace-local `rust-toolchain.toml` files and inherited Cargo-run
        // variables cannot replace the caller-selected compiler.
        command
            .env("RUSTUP_TOOLCHAIN", &self.toolchain)
            .env("RUSTC", &self.rustc)
            .env("RUSTDOC", &self.rustdoc);
        if cfg!(windows) {
            // Full Bevy feature/test graphs can otherwise run several multi-GB
            // rustc processes concurrently and exhaust the Windows commit
            // limit. Child Cargo invocations inherit this bound as well.
            command.env("CARGO_BUILD_JOBS", "1");
        }
        command
    }

    fn run_cargo(&self, label: &str, args: &[&str], native: bool) -> Result<()> {
        let command = self.cargo_command(&self.jackdaw, args, native);
        self.run_logged(label, command)
    }

    fn run_root_cargo(&self, label: &str, args: &[&str], native: bool) -> Result<()> {
        let mut command = ProcessCommand::new(&self.root_cargo);
        command.current_dir(&self.root).args(args);
        if native {
            command.envs(self.native_env.iter());
            command.env("CMAKE_GENERATOR", "Ninja");
        }
        command
            .env("RUSTUP_TOOLCHAIN", ROOT_TOOLCHAIN)
            .env("RUSTC", &self.root_rustc)
            .env("RUSTDOC", &self.root_rustdoc);
        if cfg!(windows) {
            command.env("CARGO_BUILD_JOBS", "1");
        }
        self.run_logged(label, command)
    }

    fn run_logged(&self, label: &str, command: ProcessCommand) -> Result<()> {
        let log_path = self.log_path(label);
        let TeeStatus::Exited(status) = run_tee(command, &log_path, None)? else {
            unreachable!("a command without a timeout cannot time out");
        };
        ensure!(
            status.success(),
            "{label} failed with {status}; full output: {}",
            log_path.display()
        );
        println!("{label}: exit 0; log {}", log_path.display());
        Ok(())
    }

    fn run_logged_timeout(
        &self,
        label: &str,
        command: ProcessCommand,
        timeout: Duration,
    ) -> Result<RunOutcome> {
        let log_path = self.log_path(label);
        let status = run_tee(command, &log_path, Some(timeout))?;
        match status {
            TeeStatus::Exited(status) => {
                ensure!(
                    status.success(),
                    "{label} exited before the smoke interval with {status}; full output: {}",
                    log_path.display()
                );
                println!("{label}: exited cleanly; log {}", log_path.display());
                Ok(RunOutcome::Exited(status))
            }
            TeeStatus::TimedOut(status) => {
                println!(
                    "{label}: initialized for {}s and was intentionally terminated ({status}); log {}",
                    timeout.as_secs(),
                    log_path.display()
                );
                Ok(RunOutcome::ControlledTermination(status))
            }
        }
    }

    fn capture_cargo_json(
        &self,
        label: &str,
        cwd: &Path,
        args: &[&str],
    ) -> Result<(CargoMetadata, PathBuf)> {
        let command = self.cargo_command(cwd, args, false);
        self.capture_json_command(label, command)
    }

    fn capture_root_cargo_json(
        &self,
        label: &str,
        args: &[&str],
    ) -> Result<(CargoMetadata, PathBuf)> {
        let mut command = ProcessCommand::new(&self.root_cargo);
        command
            .current_dir(&self.root)
            .args(args)
            .env("RUSTUP_TOOLCHAIN", ROOT_TOOLCHAIN)
            .env("RUSTC", &self.root_rustc)
            .env("RUSTDOC", &self.root_rustdoc);
        self.capture_json_command(label, command)
    }

    fn capture_json_command(
        &self,
        label: &str,
        mut command: ProcessCommand,
    ) -> Result<(CargoMetadata, PathBuf)> {
        let context = command_context(&command);
        println!("+ {context}");
        let output = command
            .output()
            .with_context(|| format!("failed to start {context}"))?;
        if !output.stderr.is_empty() {
            io::stderr().write_all(&output.stderr)?;
        }
        if !output.status.success() {
            if !output.stdout.is_empty() {
                io::stdout().write_all(&output.stdout)?;
            }
            bail!("{label} failed with {}", output.status);
        }
        let json_path = self.log_path(label).with_extension("json");
        fs::write(&json_path, &output.stdout)
            .with_context(|| format!("failed to write {}", json_path.display()))?;
        let metadata = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("failed to parse metadata from {label}"))?;
        println!("{label}: exit 0; metadata {}", json_path.display());
        Ok((metadata, json_path))
    }

    fn metadata_matrix(&self, matrix: MetadataMatrix) -> Result<(CargoMetadata, PathBuf)> {
        let mut args = vec![
            "metadata",
            "--offline",
            "--locked",
            "--filter-platform",
            TARGET_PLATFORM,
            "--format-version",
            "1",
        ];
        args.extend_from_slice(matrix.feature_args);
        self.capture_cargo_json(&format!("metadata-{}", matrix.label), &self.jackdaw, &args)
    }

    fn log_path(&self, label: &str) -> PathBuf {
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let sequence = LOG_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        self.evidence.join(format!(
            "{epoch}-{}-{sequence:03}-{}.log",
            std::process::id(),
            sanitize_label(label)
        ))
    }
}

#[derive(Debug)]
enum RunOutcome {
    Exited(ExitStatus),
    ControlledTermination(ExitStatus),
}

enum TeeStatus {
    Exited(ExitStatus),
    TimedOut(ExitStatus),
}

fn read_toolchain_channel(path: &Path) -> Result<String> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {}", path.display()))?;
    document
        .get("toolchain")
        .and_then(Item::as_table)
        .and_then(|table| table.get("channel"))
        .and_then(Item::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{} has no toolchain.channel", path.display()))
}

fn rustup_which(toolchain: &str, binary: &str) -> Result<PathBuf> {
    let output = ProcessCommand::new("rustup")
        .args(["which", "--toolchain", toolchain, binary])
        .output()
        .with_context(|| format!("failed to invoke rustup for {toolchain} {binary}"))?;
    ensure!(
        output.status.success(),
        "rustup cannot resolve {binary} for {toolchain}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let path = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    ensure!(
        path.is_file(),
        "rustup returned missing path {}",
        path.display()
    );
    Ok(path)
}

fn windows_native_environment() -> Result<BTreeMap<OsString, OsString>> {
    let mut result = BTreeMap::new();
    if !cfg!(windows) {
        return Ok(result);
    }

    if env::var_os("VSCMD_VER").is_some() && env::var_os("VCINSTALLDIR").is_some() {
        for (key, value) in env::vars_os() {
            result.insert(key, value);
        }
    } else {
        let vsdevcmd = find_vsdevcmd()?;
        let output = ProcessCommand::new("cmd.exe")
            // Pass shell tokens separately. Rust's normal Windows quoting then
            // quotes only the Program Files path; passing one script argument
            // would escape its inner quotes for the MSVC argv convention,
            // which `cmd.exe` deliberately does not implement.
            .args(["/d", "/c", "call"])
            .arg(&vsdevcmd)
            .args([
                "-no_logo",
                "-arch=x64",
                "-host_arch=x64",
                ">nul",
                "&&",
                "set",
            ])
            .output()
            .with_context(|| format!("failed to import {}", vsdevcmd.display()))?;
        ensure!(
            output.status.success(),
            "Visual Studio developer environment failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if !key.is_empty() && !key.starts_with('=') {
                result.insert(OsString::from(key), OsString::from(value));
            }
        }
    }

    let ninja = find_program_in_environment("ninja.exe", &result).or_else(|| {
        env::var_os("USERPROFILE").and_then(|home| {
            let candidate = PathBuf::from(home).join(".cmake-deps/ninja/win/x64/ninja.exe");
            candidate.is_file().then_some(candidate)
        })
    });
    let ninja = ninja.context(
        "Ninja is required for the Windows Manifold/AWS-LC native builds but was not found",
    )?;
    prepend_to_environment_path(&mut result, ninja.parent().context("Ninja has no parent")?)?;
    result.insert(OsString::from("CMAKE_GENERATOR"), OsString::from("Ninja"));
    Ok(result)
}

fn find_vsdevcmd() -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
        let vswhere =
            PathBuf::from(program_files_x86).join("Microsoft Visual Studio/Installer/vswhere.exe");
        if vswhere.is_file() {
            let output = ProcessCommand::new(&vswhere)
                .args([
                    "-latest",
                    "-products",
                    "*",
                    "-requires",
                    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                    "-property",
                    "installationPath",
                ])
                .output()?;
            if output.status.success() {
                let installation = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if !installation.is_empty() {
                    candidates.push(PathBuf::from(installation).join("Common7/Tools/VsDevCmd.bat"));
                }
            }
        }
    }
    if let Some(program_files) = env::var_os("ProgramFiles") {
        let visual_studio = PathBuf::from(program_files).join("Microsoft Visual Studio");
        if let Ok(versions) = fs::read_dir(&visual_studio) {
            for version in versions.flatten() {
                if let Ok(editions) = fs::read_dir(version.path()) {
                    for edition in editions.flatten() {
                        candidates.push(edition.path().join("Common7/Tools/VsDevCmd.bat"));
                    }
                }
            }
        }
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .context("Visual Studio C++ x64 developer tools were not found")
}

fn find_program_in_environment(
    program: &str,
    environment: &BTreeMap<OsString, OsString>,
) -> Option<PathBuf> {
    let path = environment
        .iter()
        .find(|(key, _)| key.to_string_lossy().eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| value.clone())
        .or_else(|| env::var_os("PATH"))?;
    env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
}

fn prepend_to_environment_path(
    environment: &mut BTreeMap<OsString, OsString>,
    directory: &Path,
) -> Result<()> {
    let path_key = environment
        .keys()
        .find(|key| key.to_string_lossy().eq_ignore_ascii_case("PATH"))
        .cloned()
        .unwrap_or_else(|| OsString::from("PATH"));
    let previous = environment
        .get(&path_key)
        .cloned()
        .or_else(|| env::var_os("PATH"))
        .unwrap_or_default();
    let mut paths = vec![directory.to_path_buf()];
    paths.extend(env::split_paths(&previous));
    environment.insert(path_key, env::join_paths(paths)?);
    Ok(())
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn command_context(command: &ProcessCommand) -> String {
    let cwd = command.get_current_dir().map_or_else(
        || Path::new(".").display().to_string(),
        |path| path.display().to_string(),
    );
    let program = command.get_program().to_string_lossy();
    let args = command
        .get_args()
        .map(|argument| format!("{:?}", argument.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ");
    format!("(cwd={cwd}) {program} {args}")
}

fn run_tee(
    mut command: ProcessCommand,
    log_path: &Path,
    timeout: Option<Duration>,
) -> Result<TeeStatus> {
    let context = command_context(&command);
    println!("+ {context}");
    let mut log = File::create(log_path)
        .with_context(|| format!("failed to create {}", log_path.display()))?;
    writeln!(log, "+ {context}")?;
    log.flush()?;
    let log = Arc::new(Mutex::new(log));

    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {context}"))?;
    let stdout = child.stdout.take().context("child stdout unavailable")?;
    let stderr = child.stderr.take().context("child stderr unavailable")?;
    let stdout_log = Arc::clone(&log);
    let stderr_log = Arc::clone(&log);
    let stdout_thread = thread::spawn(move || pump_stream(stdout, false, stdout_log));
    let stderr_thread = thread::spawn(move || pump_stream(stderr, true, stderr_log));

    let (status, timed_out) = wait_for_child(&mut child, timeout)?;
    stdout_thread
        .join()
        .map_err(|_| anyhow::anyhow!("stdout forwarding thread panicked"))??;
    stderr_thread
        .join()
        .map_err(|_| anyhow::anyhow!("stderr forwarding thread panicked"))??;
    if timed_out {
        Ok(TeeStatus::TimedOut(status))
    } else {
        Ok(TeeStatus::Exited(status))
    }
}

fn wait_for_child(child: &mut Child, timeout: Option<Duration>) -> Result<(ExitStatus, bool)> {
    let Some(timeout) = timeout else {
        return Ok((child.wait()?, false));
    };
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok((status, false));
        }
        if start.elapsed() >= timeout {
            child.kill().context("failed to terminate smoke process")?;
            return Ok((child.wait()?, true));
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn pump_stream<R: Read>(mut reader: R, stderr: bool, log: Arc<Mutex<File>>) -> io::Result<()> {
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(());
        }
        if stderr {
            let mut output = io::stderr().lock();
            output.write_all(&buffer[..count])?;
            output.flush()?;
        } else {
            let mut output = io::stdout().lock();
            output.write_all(&buffer[..count])?;
            output.flush()?;
        }
        let mut file = log
            .lock()
            .map_err(|_| io::Error::other("evidence log lock poisoned"))?;
        file.write_all(&buffer[..count])?;
        file.flush()?;
    }
}

fn capture_checked(program: impl AsRef<OsStr>, args: &[&str], cwd: &Path) -> Result<String> {
    let output = ProcessCommand::new(program)
        .current_dir(cwd)
        .args(args)
        .output()?;
    ensure!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub(crate) fn preflight(root: &Path, toolchain: Option<&str>) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    let branch = capture_checked("git", &["branch", "--show-current"], &studio.root)?;
    ensure!(
        branch == "integration/jackdaw-bevy-main",
        "expected integration/jackdaw-bevy-main, found {branch}"
    );
    for reference in ["main", "refs/remotes/origin/main"] {
        let revision = capture_checked("git", &["rev-parse", reference], &studio.root)?;
        ensure!(
            revision == FROZEN_BEVY_REVISION,
            "{reference} moved from the frozen Bevy baseline: {revision}"
        );
    }
    let ancestor_status = ProcessCommand::new("git")
        .current_dir(&studio.root)
        .args(["merge-base", "--is-ancestor", FROZEN_BEVY_REVISION, "HEAD"])
        .status()?;
    ensure!(
        ancestor_status.success(),
        "HEAD is not descended from frozen Bevy {FROZEN_BEVY_REVISION}"
    );

    let status = capture_checked("git", &["status", "--porcelain=v1"], &studio.root)?;
    let unexpected = status
        .lines()
        .filter(|line| *line != "?? .agents/" && !line.is_empty())
        .collect::<Vec<_>>();
    ensure!(
        unexpected.is_empty(),
        "preflight requires a checkpointed worktree; unexpected entries: {unexpected:?}"
    );
    let submodules = capture_checked("git", &["submodule", "status"], &studio.root)?;
    ensure!(submodules.is_empty(), "unexpected submodules: {submodules}");

    for entry in WalkDir::new(&studio.jackdaw)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !matches!(entry.file_name().to_str(), Some("target" | ".jj")))
    {
        let entry = entry?;
        ensure!(
            entry.file_name() != OsStr::new(".git"),
            "nested Git metadata found at {}",
            entry.path().display()
        );
    }

    let root_cargo_version = capture_checked(&studio.root_cargo, &["-V"], &studio.root)?;
    let root_rustc_version = capture_checked(&studio.root_rustc, &["-Vv"], &studio.root)?;
    let cargo_version = capture_checked(&studio.cargo, &["-V"], &studio.jackdaw)?;
    let rustc_version = capture_checked(&studio.rustc, &["-Vv"], &studio.jackdaw)?;
    ensure!(
        rustc_version.contains(TARGET_PLATFORM),
        "Jackdaw rustc host is not {TARGET_PLATFORM}: {rustc_version}"
    );

    let ninja = find_program_in_environment("ninja.exe", &studio.native_env)
        .context("Ninja disappeared from the prepared native environment")?;
    let vs_version = studio
        .native_env
        .iter()
        .find(|(key, _)| key.to_string_lossy().eq_ignore_ascii_case("VSCMD_VER"))
        .map(|(_, value)| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "not applicable".to_owned());
    println!("preflight: branch {branch}");
    println!("preflight: frozen Bevy {FROZEN_BEVY_REVISION}");
    println!("preflight: imported Jackdaw {JACKDAW_REVISION}");
    println!("preflight: root {root_cargo_version}");
    println!("preflight: root {}", first_line(&root_rustc_version));
    println!("preflight: nested {cargo_version}");
    println!("preflight: nested {}", first_line(&rustc_version));
    println!("preflight: Visual Studio developer environment {vs_version}");
    println!("preflight: Ninja {}", ninja.display());
    println!(
        "preflight: evidence directory {}",
        studio.evidence.display()
    );

    let (_, metadata_path) = studio.capture_root_cargo_json(
        "root-metadata-no-deps",
        &[
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ],
    )?;
    println!(
        "preflight: root metadata resolves at {}",
        metadata_path.display()
    );
    Ok(())
}

pub(crate) fn inventory(root: &Path, toolchain: Option<&str>, check: bool) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    let input_json = studio.root.join(INVENTORY_RELATIVE);
    let output_json = studio
        .log_path("inventory-generated")
        .with_extension("json");
    let output_markdown = studio.log_path("inventory-generated").with_extension("md");
    let mut command = ProcessCommand::new("python");
    command
        .current_dir(&studio.root)
        .arg(studio.root.join(INVENTORY_TOOL_RELATIVE))
        .arg("--refresh-existing")
        .arg(&input_json)
        .arg("--provenance")
        .arg(studio.root.join(PROVENANCE_RELATIVE))
        .arg("--lockfile")
        .arg(studio.jackdaw.join("Cargo.lock"))
        .arg("--output-json")
        .arg(&output_json)
        .arg("--output-markdown")
        .arg(&output_markdown);
    studio.run_logged("inventory-refresh", command)?;

    let generated = load_inventory(&output_json)?;
    validate_inventory(&studio.root, &generated)?;
    let checked_json = studio.root.join(INVENTORY_RELATIVE);
    let checked_markdown = studio.root.join(INVENTORY_MARKDOWN_RELATIVE);
    if check {
        ensure!(
            fs::read(&checked_json)? == fs::read(&output_json)?,
            "inventory JSON drifted; run `cargo studio inventory`"
        );
        ensure!(
            fs::read(&checked_markdown)? == fs::read(&output_markdown)?,
            "inventory Markdown drifted; run `cargo studio inventory`"
        );
        println!(
            "inventory: checked {} packages across {} upstreams; no drift",
            generated.packages.len(),
            generated.upstreams.len()
        );
    } else {
        fs::copy(&output_json, &checked_json)
            .with_context(|| format!("failed to update {}", checked_json.display()))?;
        fs::copy(&output_markdown, &checked_markdown)
            .with_context(|| format!("failed to update {}", checked_markdown.display()))?;
        println!(
            "inventory: wrote {} packages across {} upstreams",
            generated.packages.len(),
            generated.upstreams.len()
        );
    }
    Ok(())
}

pub(crate) fn provenance(root: &Path) -> Result<()> {
    let inventory = load_inventory(&root.join(INVENTORY_RELATIVE))?;
    let inventory_names = inventory
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect::<BTreeSet<_>>();
    let provenance_path = root.join(PROVENANCE_RELATIVE);
    let source = fs::read_to_string(&provenance_path)
        .with_context(|| format!("failed to read {}", provenance_path.display()))?;
    let document = source
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {}", provenance_path.display()))?;
    ensure!(
        document.get("schema_version").and_then(Item::as_integer) == Some(1),
        "unsupported provenance schema"
    );
    ensure!(
        document.get("bevy_revision").and_then(Item::as_str) == Some(FROZEN_BEVY_REVISION),
        "provenance Bevy revision drift"
    );
    ensure!(
        document.get("jackdaw_revision").and_then(Item::as_str) == Some(JACKDAW_REVISION),
        "provenance Jackdaw revision drift"
    );
    ensure!(
        document
            .get("jackdaw_lockfile_sha256")
            .and_then(Item::as_str)
            == Some(inventory.lockfile_sha256.as_str()),
        "provenance and inventory disagree on the Jackdaw lockfile identity"
    );
    let upstreams = document
        .get("upstream")
        .and_then(Item::as_array_of_tables)
        .context("provenance has no [[upstream]] records")?;
    ensure!(
        upstreams.len() == 12,
        "expected 12 upstreams, found {}",
        upstreams.len()
    );
    let mut slugs = BTreeSet::new();
    let mut mapped_packages = BTreeSet::new();
    let mut supplemental_packages = BTreeSet::new();
    let vendor = root.join(VENDOR_RELATIVE).canonicalize()?;

    for upstream in upstreams {
        let slug = upstream
            .get("slug")
            .and_then(Item::as_str)
            .context("provenance upstream has no slug")?;
        ensure!(slugs.insert(slug), "duplicate provenance upstream {slug}");
        let supplemental = upstream
            .get("supplemental")
            .and_then(Item::as_bool)
            .unwrap_or(false);
        let local_path = upstream
            .get("local_path")
            .and_then(Item::as_str)
            .with_context(|| format!("upstream {slug} has no local_path"))?;
        ensure_relative(local_path, "provenance local_path")?;
        let local_root = root.join(local_path).canonicalize()?;
        ensure!(
            local_root.starts_with(&vendor),
            "upstream {slug} escapes the vendor root"
        );
        let status = upstream
            .get("migration_status")
            .and_then(Item::as_str)
            .unwrap_or_default();
        ensure!(
            !status.to_ascii_lowercase().contains("pending"),
            "upstream {slug} still reports a pending migration"
        );
        let original_rev = upstream
            .get("original_rev")
            .and_then(Item::as_str)
            .unwrap_or_default();
        let registry_checksums = upstream
            .get("registry_checksums")
            .and_then(Item::as_array)
            .is_some_and(|items| !items.is_empty());
        ensure!(
            !original_rev.is_empty() || registry_checksums,
            "upstream {slug} has neither an immutable revision nor registry checksums"
        );

        let package_paths = upstream
            .get("package_paths")
            .and_then(Item::as_array)
            .with_context(|| format!("upstream {slug} has no package_paths"))?;
        for mapping in package_paths.iter().filter_map(Value::as_str) {
            let (name, relative) = mapping
                .split_once('=')
                .with_context(|| format!("invalid package mapping {mapping}"))?;
            if supplemental {
                ensure!(
                    !inventory_names.contains(name),
                    "supplemental provenance package {name} is already classified"
                );
                ensure!(
                    supplemental_packages.insert(name),
                    "provenance maps supplemental package {name} more than once"
                );
            } else {
                ensure!(
                    inventory_names.contains(name),
                    "provenance package {name} is not classified"
                );
                ensure!(
                    mapped_packages.insert(name),
                    "provenance maps package {name} more than once"
                );
            }
            ensure_relative(relative, "provenance package path")?;
            ensure!(
                local_root.join(relative).is_file(),
                "missing mapped manifest {}",
                local_root.join(relative).display()
            );
        }
        let license_files = upstream
            .get("license_files")
            .and_then(Item::as_array)
            .with_context(|| format!("upstream {slug} has no license_files"))?;
        ensure!(!license_files.is_empty(), "upstream {slug} has no licenses");
        for license in license_files.iter().filter_map(Value::as_str) {
            ensure_relative(license, "provenance license path")?;
            ensure!(
                local_root.join(license).is_file(),
                "missing license {}",
                local_root.join(license).display()
            );
        }
    }
    ensure!(
        mapped_packages == inventory_names,
        "provenance package mappings disagree with the inventory"
    );
    ensure!(
        supplemental_packages.len() == 1 && supplemental_packages.contains("bevy_egui"),
        "expected the supplemental bevy_egui package, found {supplemental_packages:?}"
    );
    println!(
        "provenance: {} upstreams, {} classified and {} supplemental package mappings, immutable source identity and licenses valid",
        slugs.len(),
        mapped_packages.len(),
        supplemental_packages.len()
    );
    Ok(())
}

pub(crate) fn metadata(root: &Path, toolchain: Option<&str>) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    for matrix in METADATA_MATRICES {
        let (metadata, path) = studio.metadata_matrix(*matrix)?;
        println!(
            "metadata {}: {} packages, {} workspace members, {}",
            matrix.label,
            metadata.packages.len(),
            metadata.workspace_members.len(),
            path.display()
        );
    }
    Ok(())
}

pub(crate) fn tree(root: &Path, toolchain: Option<&str>) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    studio.run_cargo(
        "tree-duplicates",
        &["tree", "--offline", "--locked", "--duplicates"],
        false,
    )
}

fn load_inventory(path: &Path) -> Result<Inventory> {
    serde_json::from_slice(
        &fs::read(path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))
}

fn validate_inventory(root: &Path, inventory: &Inventory) -> Result<()> {
    ensure!(
        inventory.packages.len() == 46,
        "expected 46 classified packages, found {}",
        inventory.packages.len()
    );
    ensure!(
        inventory.upstreams.len() == 11,
        "expected 11 upstream groups, found {}",
        inventory.upstreams.len()
    );
    ensure!(
        inventory.cycles.is_empty(),
        "inventory contains dependency cycles"
    );
    ensure!(
        inventory.excluded_candidates.is_empty(),
        "inventory has unexplained excluded candidates"
    );
    ensure!(
        inventory.supplemental_upstreams.len() == 1,
        "expected one supplemental upstream, found {}",
        inventory.supplemental_upstreams.len()
    );
    let vendor = root.join(VENDOR_RELATIVE).canonicalize()?;
    let supplemental = &inventory.supplemental_upstreams[0];
    ensure!(
        supplemental.slug == "bevy-egui" && supplemental.packages == ["bevy_egui"],
        "unexpected supplemental upstream {} packages {:?}",
        supplemental.slug,
        supplemental.packages
    );
    ensure_relative(&supplemental.local_path, "supplemental upstream path")?;
    let supplemental_root = root.join(&supplemental.local_path).canonicalize()?;
    ensure!(
        supplemental_root.starts_with(&vendor) && supplemental_root.is_dir(),
        "supplemental upstream {} escapes or is missing from the vendor root",
        supplemental.slug
    );
    let mut keys = BTreeSet::new();
    let upstream_slugs = inventory
        .upstreams
        .iter()
        .map(|upstream| upstream.slug.as_str())
        .collect::<BTreeSet<_>>();
    let mut packages_by_upstream = BTreeMap::<&str, BTreeSet<(&str, &str)>>::new();
    for package in &inventory.packages {
        ensure!(
            keys.insert((package.name.as_str(), package.version.as_str())),
            "duplicate inventory package {} {}",
            package.name,
            package.version
        );
        ensure!(
            upstream_slugs.contains(package.upstream_slug.as_str()),
            "package {} has unknown upstream {}",
            package.name,
            package.upstream_slug
        );
        ensure!(
            package
                .source
                .as_deref()
                .is_some_and(|source| !source.is_empty()),
            "package {} lost its original source identity",
            package.name
        );
        ensure!(
            package
                .repository
                .as_deref()
                .is_some_and(|repository| !repository.is_empty()),
            "package {} lost its upstream repository identity",
            package.name
        );
        packages_by_upstream
            .entry(package.upstream_slug.as_str())
            .or_default()
            .insert((package.name.as_str(), package.version.as_str()));
        let relative = package
            .local_source_path
            .as_deref()
            .with_context(|| format!("package {} has no local_source_path", package.name))?;
        ensure_relative(relative, "inventory local source")?;
        let manifest = root.join(relative).canonicalize()?;
        ensure!(
            manifest.starts_with(&vendor) && manifest.is_file(),
            "package {} escapes or is missing from the vendor root",
            package.name
        );
        let status = package.migration_status.as_deref().unwrap_or_default();
        ensure!(
            !status.to_ascii_lowercase().contains("pending"),
            "package {} still reports a pending migration",
            package.name
        );
    }
    for upstream in &inventory.upstreams {
        let local_path = upstream
            .local_path
            .as_deref()
            .with_context(|| format!("upstream {} has no local_path", upstream.slug))?;
        ensure_relative(local_path, "inventory upstream path")?;
        let local_root = root.join(local_path).canonicalize()?;
        ensure!(
            local_root.starts_with(&vendor) && local_root.is_dir(),
            "upstream {} escapes or is missing from the vendor root",
            upstream.slug
        );
        let recorded = upstream
            .packages
            .iter()
            .map(|package| (package.name.as_str(), package.version.as_str()))
            .collect::<BTreeSet<_>>();
        let classified = packages_by_upstream
            .get(upstream.slug.as_str())
            .with_context(|| format!("upstream {} has no classified packages", upstream.slug))?;
        ensure!(
            &recorded == classified,
            "upstream {} package records disagree with classified packages",
            upstream.slug
        );
    }
    Ok(())
}

fn ensure_relative(path: &str, label: &str) -> Result<()> {
    let path = Path::new(path);
    ensure!(
        !path.is_absolute(),
        "{label} is absolute: {}",
        path.display()
    );
    ensure!(
        !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir)),
        "{label} escapes with `..`: {}",
        path.display()
    );
    Ok(())
}

fn first_line(value: &str) -> &str {
    value.lines().next().unwrap_or(value)
}

pub(crate) fn audit_bevy_sources(root: &Path, toolchain: Option<&str>) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    let inventory = load_inventory(&studio.root.join(INVENTORY_RELATIVE))?;
    validate_inventory(&studio.root, &inventory)?;
    let official = inventory
        .official_bevy_packages
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let classified = inventory
        .packages
        .iter()
        .map(|package| ((package.name.as_str(), package.version.as_str()), package))
        .collect::<BTreeMap<_, _>>();
    let vendor = studio.root.join(VENDOR_RELATIVE).canonicalize()?;

    for matrix in METADATA_MATRICES {
        let (metadata, _) = studio.metadata_matrix(*matrix)?;
        let mut official_versions = BTreeMap::<&str, BTreeSet<&str>>::new();
        let mut resolved_classified = BTreeSet::new();
        let mut bevy_count = 0;
        for package in &metadata.packages {
            if official.contains(package.name.as_str()) {
                if package.name == "bevy" {
                    bevy_count += 1;
                }
                official_versions
                    .entry(&package.name)
                    .or_default()
                    .insert(&package.version);
                ensure!(
                    package.version == LOCAL_BEVY_VERSION,
                    "{} resolves official {} at unsupported version {}",
                    matrix.label,
                    package.name,
                    package.version
                );
                ensure!(
                    package.source.is_none(),
                    "{} resolves official {} from {:?}",
                    matrix.label,
                    package.name,
                    package.source
                );
                let manifest = package.manifest_path.canonicalize()?;
                ensure!(
                    manifest.starts_with(&studio.root) && !manifest.starts_with(&vendor),
                    "{} resolves official {} outside the frozen Bevy checkout: {}",
                    matrix.label,
                    package.name,
                    manifest.display()
                );
            }
            if let Some(expected) =
                classified.get(&(package.name.as_str(), package.version.as_str()))
            {
                ensure!(
                    package.source.is_none(),
                    "{} resolves classified {} from {:?}",
                    matrix.label,
                    package.name,
                    package.source
                );
                let manifest = package.manifest_path.canonicalize()?;
                let expected_manifest = studio
                    .root
                    .join(
                        expected
                            .local_source_path
                            .as_deref()
                            .context("classified package has no local path")?,
                    )
                    .canonicalize()?;
                ensure!(
                    manifest == expected_manifest && manifest.starts_with(&vendor),
                    "{} resolves classified {} from unexpected manifest {}",
                    matrix.label,
                    package.name,
                    manifest.display()
                );
                resolved_classified.insert((package.name.as_str(), package.version.as_str()));
            }
        }
        ensure!(
            bevy_count == 1,
            "{} resolves {bevy_count} `bevy` packages instead of one",
            matrix.label
        );
        for (name, versions) in official_versions {
            ensure!(
                versions.len() == 1,
                "{} resolves duplicate official Bevy identities for {name}: {versions:?}",
                matrix.label
            );
        }
        if matrix.label == "workspace-all-features" {
            let expected = classified.keys().copied().collect::<BTreeSet<_>>();
            ensure!(
                resolved_classified == expected,
                "workspace all-features metadata and the classified inventory disagree"
            );
        }
        println!(
            "audit {}: {} official package identities and {} classified packages are local",
            matrix.label,
            metadata
                .packages
                .iter()
                .filter(|package| official.contains(package.name.as_str()))
                .count(),
            resolved_classified.len()
        );
    }
    audit_manifests(&studio.root, &official)?;
    println!(
        "audit-bevy-sources: no external official Bevy source, Bevy 0.19 requirement, or non-vendored classified package"
    );
    Ok(())
}

fn audit_manifests(root: &Path, official: &BTreeSet<&str>) -> Result<()> {
    let jackdaw = root.join(JACKDAW_RELATIVE);
    for entry in WalkDir::new(&jackdaw)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !matches!(entry.file_name().to_str(), Some("target" | ".git" | ".jj"))
        })
    {
        let entry = entry?;
        if !entry.file_type().is_file() || entry.file_name() != OsStr::new("Cargo.toml") {
            continue;
        }
        let source = fs::read_to_string(entry.path())?;
        let document = source
            .parse::<DocumentMut>()
            .with_context(|| format!("failed to parse {}", entry.path().display()))?;
        audit_dependency_document(entry.path(), &document, official)?;
    }
    Ok(())
}

fn audit_dependency_document(
    manifest: &Path,
    document: &DocumentMut,
    official: &BTreeSet<&str>,
) -> Result<()> {
    for dependency_table in dependency_tables(document) {
        for (alias, dependency) in dependency_table.iter() {
            let package = dependency_package(alias, dependency);
            if !official.contains(package) {
                continue;
            }
            if let Some(version) = dependency_string(dependency, "version") {
                ensure!(
                    !version.contains("0.19"),
                    "unsupported Bevy 0.19 requirement for {alias} in {}",
                    manifest.display()
                );
            }
            ensure!(
                dependency_string(dependency, "git").is_none(),
                "official Bevy dependency {alias} retains a Git source in {}",
                manifest.display()
            );
            if let Some(path) = dependency_string(dependency, "path") {
                ensure!(
                    !Path::new(path).is_absolute(),
                    "official Bevy dependency {alias} uses an absolute path in {}",
                    manifest.display()
                );
            }
        }
    }
    Ok(())
}

fn dependency_tables(document: &DocumentMut) -> Vec<&dyn toml_edit::TableLike> {
    let mut result = Vec::new();
    for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = document.get(name).and_then(Item::as_table_like) {
            result.push(table);
        }
    }
    if let Some(workspace) = document.get("workspace").and_then(Item::as_table)
        && let Some(table) = workspace.get("dependencies").and_then(Item::as_table_like)
    {
        result.push(table);
    }
    if let Some(targets) = document.get("target").and_then(Item::as_table) {
        for (_, target) in targets {
            let Some(target) = target.as_table() else {
                continue;
            };
            for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(table) = target.get(name).and_then(Item::as_table_like) {
                    result.push(table);
                }
            }
        }
    }
    result
}

fn dependency_package<'a>(alias: &'a str, dependency: &'a Item) -> &'a str {
    dependency_string(dependency, "package").unwrap_or(alias)
}

fn dependency_string<'a>(dependency: &'a Item, key: &str) -> Option<&'a str> {
    match dependency {
        Item::Value(Value::InlineTable(table)) => table.get(key).and_then(Value::as_str),
        Item::Table(table) => table.get(key).and_then(Item::as_str),
        Item::Value(Value::String(value)) if key == "version" => Some(value.value()),
        _ => None,
    }
}

pub(crate) fn check(root: &Path, toolchain: Option<&str>) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    studio.run_cargo("check-default", &["check", "--offline", "--locked"], true)
}

pub(crate) fn check_all(
    root: &Path,
    toolchain: Option<&str>,
    resume_focused_from: Option<&str>,
) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    let matrices: &[(&str, &[&str])] = &[
        ("check-default", &["check", "--offline", "--locked"]),
        (
            "check-no-default-features",
            &["check", "--offline", "--locked", "--no-default-features"],
        ),
        (
            "check-all-features",
            &["check", "--offline", "--locked", "--all-features"],
        ),
        (
            "check-workspace-all-targets-all-features",
            &[
                "check",
                "--offline",
                "--locked",
                "--workspace",
                "--all-targets",
                "--all-features",
            ],
        ),
    ];
    if resume_focused_from.is_none() {
        for (label, args) in matrices {
            studio.run_cargo(label, args, true)?;
        }
    }
    focused_vendored_checks(&studio, resume_focused_from)
}

fn focused_vendored_checks(studio: &Studio, resume_from: Option<&str>) -> Result<()> {
    let inventory = load_inventory(&studio.root.join(INVENTORY_RELATIVE))?;
    let mut packages = inventory.packages.iter().collect::<Vec<_>>();
    packages.sort_by(|left, right| {
        left.upstream_slug
            .cmp(&right.upstream_slug)
            .then_with(|| left.name.cmp(&right.name))
    });
    let start = if let Some(resume_from) = resume_from {
        packages
            .iter()
            .position(|package| package.name == resume_from)
            .with_context(|| {
                format!(
                    "cannot resume focused checks: package `{resume_from}` is not in the inventory"
                )
            })?
    } else {
        0
    };
    if resume_from.is_none() {
        focused_bevy_egui_check(studio)?;
    }
    let mut aeronet_checked = false;
    for package in packages.into_iter().skip(start) {
        if package.upstream_slug == "aeronet" {
            if !aeronet_checked {
                let manifest = studio.root.join(VENDOR_RELATIVE).join("aeronet/Cargo.toml");
                let manifest = manifest.to_string_lossy();
                let mut command = studio.cargo_command(
                    &studio.jackdaw,
                    &[
                        "clippy",
                        "--offline",
                        "--locked",
                        "--manifest-path",
                        &manifest,
                        "--workspace",
                        "--all-features",
                        "--all-targets",
                    ],
                    true,
                );
                if cfg!(windows) {
                    // aws-lc-sys 0.34's compiler probe does not account for the
                    // C11 mode required by Visual Studio 18, and its prebuilt
                    // NASM batch file cannot quote this checkout's spaced path.
                    // These are supported debug-build controls; no Aeronet
                    // feature or target is removed from the mandated Clippy run.
                    command
                        .env("AWS_LC_SYS_C_STD", "11")
                        .env("AWS_LC_SYS_NO_ASM", "1")
                        // Keep this relative: canonical Windows paths carry a
                        // `\\?\` prefix that cmd.exe rejects as a UNC cwd when
                        // AWS-LC invokes the MSVC librarian through Ninja.
                        .env("CARGO_TARGET_DIR", "../../target/jackdaw-aeronet");
                }
                studio.run_logged("vendored-aeronet-clippy", command)?;
                aeronet_checked = true;
            }
            continue;
        }
        let manifest = studio.root.join(
            package
                .local_source_path
                .as_deref()
                .context("vendored package has no manifest")?,
        );
        let manifest = manifest.to_string_lossy();
        studio.run_cargo(
            &format!("vendored-check-{}", package.name),
            &[
                "check",
                "--offline",
                "--locked",
                "--manifest-path",
                &manifest,
                "--package",
                &package.name,
            ],
            true,
        )?;
    }
    Ok(())
}

fn focused_bevy_egui_check(studio: &Studio) -> Result<()> {
    let cwd = studio.root.join(VENDOR_RELATIVE).join("bevy-egui");
    let manifest = cwd.join("Cargo.toml");
    let manifest = manifest.to_string_lossy();
    let mut native_env = studio.native_env.clone();
    prepend_to_environment_path(
        &mut native_env,
        studio
            .root_cargo
            .parent()
            .context("root Cargo has no parent directory")?,
    )?;
    let mut command = ProcessCommand::new(&studio.root_cargo);
    command
        .current_dir(&cwd)
        .envs(native_env.iter())
        .env("CMAKE_GENERATOR", "Ninja")
        .env("RUSTUP_TOOLCHAIN", ROOT_TOOLCHAIN)
        .env("RUSTC", &studio.root_rustc)
        .env("RUSTDOC", &studio.root_rustdoc)
        .env("CARGO_BUILD_JOBS", "1")
        .env("CARGO_INCREMENTAL", "0")
        // This snapshot is a standalone workspace. Keep its stable-Cargo
        // artifacts isolated from Jackdaw's required nightly build layout.
        .env(
            "CARGO_TARGET_DIR",
            "../../../target/jackdaw-bevy-egui-stable",
        )
        .args([
            "check",
            "--offline",
            "--locked",
            "--manifest-path",
            &manifest,
            "--all-targets",
            "--all-features",
        ]);
    studio.run_logged("vendored-bevy-egui-check-all", command)
}

pub(crate) fn test(root: &Path, toolchain: Option<&str>) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    let mut command = studio.cargo_command(
        &studio.jackdaw,
        &[
            "test",
            "--offline",
            "--locked",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--jobs",
            "1",
        ],
        true,
    );
    if cfg!(windows) {
        // Avoid multi-gigabyte PDBs for every test harness. This changes only
        // debug-symbol emission, not code, features, or which tests execute.
        command.env("CARGO_PROFILE_TEST_DEBUG", "0");
    }
    studio.run_logged("test-workspace-all-targets-all-features", command)
}

pub(crate) fn build(root: &Path, toolchain: Option<&str>) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    let variants: &[(&str, &[&str])] = &[
        (
            "build-editor-default",
            &[
                "build",
                "--offline",
                "--locked",
                "-p",
                "jackdaw",
                "--bin",
                "jackdaw",
            ],
        ),
        (
            "build-editor-all-features",
            &[
                "build",
                "--offline",
                "--locked",
                "-p",
                "jackdaw",
                "--bin",
                "jackdaw",
                "--all-features",
            ],
        ),
        (
            "build-cli",
            &[
                "build",
                "--offline",
                "--locked",
                "-p",
                "jackdaw",
                "--bin",
                "jd",
            ],
        ),
        (
            "build-runner",
            &["build", "--offline", "--locked", "-p", "jackdaw_runner"],
        ),
        (
            "build-rustc-wrapper",
            &[
                "build",
                "--offline",
                "--locked",
                "-p",
                "jackdaw_rustc_wrapper",
            ],
        ),
        (
            "build-sdk",
            &["build", "--offline", "--locked", "-p", "jackdaw_sdk"],
        ),
        (
            "build-pie-fixtures",
            &[
                "build",
                "--offline",
                "--locked",
                "-p",
                "dynamic_extension",
                "-p",
                "test_fixture_extension",
            ],
        ),
        (
            "build-release-support",
            &[
                "build",
                "--offline",
                "--locked",
                "--release",
                "-p",
                "jackdaw_runner",
                "-p",
                "jackdaw_rustc_wrapper",
            ],
        ),
    ];
    for (label, args) in variants {
        studio.run_cargo(label, args, true)?;
    }
    Ok(())
}

pub(crate) fn editor(
    root: &Path,
    toolchain: Option<&str>,
    smoke_seconds: Option<u64>,
    args: &[String],
) -> Result<()> {
    let studio = Studio::new(root, toolchain)?;
    studio.run_cargo(
        "editor-build",
        &[
            "build",
            "--offline",
            "--locked",
            "-p",
            "jackdaw",
            "--bin",
            "jackdaw",
        ],
        true,
    )?;
    let executable = studio.jackdaw.join("target/debug").join(if cfg!(windows) {
        "jackdaw.exe"
    } else {
        "jackdaw"
    });
    ensure!(
        executable.is_file(),
        "editor build did not produce {}",
        executable.display()
    );
    println!("editor: executable {}", executable.display());
    let mut command = ProcessCommand::new(&executable);
    command
        .current_dir(&studio.jackdaw)
        .args(args)
        .envs(studio.native_env.iter())
        .env("RUSTUP_TOOLCHAIN", &studio.toolchain);
    if let Some(seconds) = smoke_seconds {
        let outcome =
            studio.run_logged_timeout("editor-runtime", command, Duration::from_secs(seconds))?;
        match outcome {
            RunOutcome::Exited(status) => {
                println!("editor: exited cleanly with {status}");
            }
            RunOutcome::ControlledTermination(status) => {
                println!("editor: controlled smoke termination recorded as {status}");
            }
        }
        Ok(())
    } else {
        studio.run_logged("editor-runtime", command)
    }
}

pub(crate) fn verify(root: &Path, toolchain: Option<&str>) -> Result<()> {
    preflight(root, toolchain)?;
    inventory(root, toolchain, true)?;
    provenance(root)?;
    metadata(root, toolchain)?;
    tree(root, toolchain)?;
    audit_bevy_sources(root, toolchain)?;
    format(
        root,
        toolchain,
        Some(&root.join(JACKDAW_RELATIVE).join("Cargo.toml")),
        true,
        &[],
        false,
        false,
        &["--check".to_owned()],
    )?;
    check_all(root, toolchain, None)?;
    test(root, toolchain)?;
    let studio = Studio::new(root, toolchain)?;
    studio.run_cargo(
        "clippy-established",
        &[
            "clippy",
            "--offline",
            "--locked",
            "--workspace",
            "--all-targets",
            "--features",
            "dylib",
            "--",
            "--deny",
            "warnings",
        ],
        true,
    )?;
    build(root, toolchain)?;
    studio.run_root_cargo(
        "root-bevy-xtask-check",
        &["check", "--offline", "--locked", "-p", "bevy-studio-xtask"],
        false,
    )?;
    println!("verify: all non-interactive gates passed; run `cargo studio editor` for Gate 6");
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "the formatter mirrors cargo-fmt's stable command-line surface"
)]
pub(crate) fn format(
    root: &Path,
    toolchain: Option<&str>,
    manifest_path: Option<&Path>,
    all: bool,
    requested_packages: &[String],
    quiet: bool,
    verbose: bool,
    rustfmt_args: &[String],
) -> Result<()> {
    ensure!(
        !(quiet && verbose),
        "quiet and verbose modes are incompatible"
    );
    let studio = Studio::new(root, toolchain)?;
    let manifest = match manifest_path {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => env::current_dir()?.join(path),
        None => env::current_dir()?.join("Cargo.toml"),
    }
    .canonicalize()
    .with_context(|| "format manifest does not exist")?;
    ensure!(
        manifest.file_name() == Some(OsStr::new("Cargo.toml")),
        "format manifest must be a Cargo.toml"
    );
    let manifest_text = manifest.to_string_lossy();
    let (metadata, _) = studio.capture_cargo_json(
        "format-metadata",
        manifest.parent().context("format manifest has no parent")?,
        &[
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            &manifest_text,
        ],
    )?;
    let workspace_members = metadata.workspace_members.iter().collect::<BTreeSet<_>>();
    let requested = requested_packages
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut seen_requested = BTreeSet::new();
    let mut targets = BTreeMap::<String, BTreeSet<PathBuf>>::new();
    for package in &metadata.packages {
        let selected = if all {
            workspace_members.contains(&package.id)
        } else if !requested.is_empty() {
            if requested.contains(package.name.as_str()) {
                seen_requested.insert(package.name.as_str());
                true
            } else {
                false
            }
        } else {
            package.manifest_path.canonicalize()? == manifest
        };
        if !selected {
            continue;
        }
        for target in &package.targets {
            let path = target.src_path.canonicalize()?;
            targets
                .entry(target.edition.clone())
                .or_default()
                .insert(path);
        }
    }
    ensure!(
        seen_requested == requested,
        "unknown format packages: {:?}",
        requested.difference(&seen_requested).collect::<Vec<_>>()
    );
    ensure!(
        !targets.is_empty(),
        "no Rust targets selected for formatting"
    );
    let workspace_root = metadata.workspace_root.canonicalize()?;
    let mut chunk_index = 0_u64;
    let mut file_count = 0_usize;
    for (edition, files) in targets {
        let mut chunk = Vec::<PathBuf>::new();
        let mut command_length = 0_usize;
        for file in files {
            if verbose {
                println!("[{}] {}", edition, file.display());
            }
            let length = file.as_os_str().to_string_lossy().len() + 3;
            if !chunk.is_empty() && command_length + length > 20_000 {
                run_rustfmt_chunk(
                    &studio,
                    &workspace_root,
                    &edition,
                    &chunk,
                    rustfmt_args,
                    quiet,
                    chunk_index,
                )?;
                chunk_index += 1;
                chunk.clear();
                command_length = 0;
            }
            file_count += 1;
            command_length += length;
            chunk.push(file);
        }
        if !chunk.is_empty() {
            run_rustfmt_chunk(
                &studio,
                &workspace_root,
                &edition,
                &chunk,
                rustfmt_args,
                quiet,
                chunk_index,
            )?;
            chunk_index += 1;
        }
    }
    println!(
        "format: {file_count} workspace target roots passed rustfmt in {chunk_index} Windows-safe chunk(s)"
    );
    Ok(())
}

fn run_rustfmt_chunk(
    studio: &Studio,
    workspace_root: &Path,
    edition: &str,
    files: &[PathBuf],
    rustfmt_args: &[String],
    quiet: bool,
    index: u64,
) -> Result<()> {
    let mut command = ProcessCommand::new(&studio.rustfmt);
    command
        .current_dir(workspace_root)
        .args(files)
        .args(["--edition", edition])
        .args(rustfmt_args)
        .env("RUSTUP_TOOLCHAIN", &studio.toolchain);
    if quiet {
        command.stdout(Stdio::null());
    }
    studio.run_logged(&format!("rustfmt-{edition}-{index:03}"), command)
}
