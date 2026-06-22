use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|e| {
        panic!("CARGO_MANIFEST_DIR missing: {e}");
    }));
    let frontend = manifest_dir.join("frontend");
    let dist = frontend.join("dist");

    println!("cargo:rerun-if-changed={}", frontend.join("src").display());
    println!(
        "cargo:rerun-if-changed={}",
        frontend.join("package.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        frontend.join("bun.lock").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        frontend.join("vite.config.ts").display()
    );

    if !needs_build(&frontend, &dist) {
        return;
    }

    require_bun();
    run_bun(&frontend, &["install", "--frozen-lockfile"]);
    run_bun(&frontend, &["run", "build"]);
}

fn needs_build(frontend: &Path, dist: &Path) -> bool {
    let index = dist.join("index.html");
    if !index.exists() {
        return true;
    }

    let dist_mtime = modified(&index);
    newest_source_mtime(frontend)
        .map(|source_mtime| source_mtime > dist_mtime)
        .unwrap_or(false)
}

fn newest_source_mtime(frontend: &Path) -> Option<std::time::SystemTime> {
    let roots = [
        frontend.join("src"),
        frontend.join("package.json"),
        frontend.join("bun.lock"),
        frontend.join("vite.config.ts"),
        frontend.join("tsconfig.json"),
        frontend.join("components.json"),
    ];
    roots
        .iter()
        .flat_map(|root| collect_paths(root))
        .filter(|path| !path.components().any(|c| c.as_os_str() == "dist"))
        .filter_map(|path| std::fs::metadata(path).ok())
        .filter_map(|meta| meta.modified().ok())
        .max()
}

fn collect_paths(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .flat_map(|entry| collect_paths(&entry.path()))
        .collect()
}

fn modified(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

#[allow(clippy::disallowed_methods)]
fn require_bun() {
    let status = Command::new("bun")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|mut child| child.wait());
    match status {
        Ok(s) if s.success() => {}
        _ => panic!(
            "Bun is required to build scryrs-dashboard frontend. Install Bun, then rerun cargo build."
        ),
    }
}

#[allow(clippy::disallowed_methods)]
fn run_bun(frontend: &Path, args: &[&str]) {
    let status = Command::new("bun")
        .args(args)
        .current_dir(frontend)
        .spawn()
        .and_then(|mut child| child.wait())
        .unwrap_or_else(|e| panic!("failed to run bun {}: {e}", args.join(" ")));
    if !status.success() {
        panic!("bun {} failed with status {status}", args.join(" "));
    }
}
