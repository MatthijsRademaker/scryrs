use std::io::Write;
use std::path::{Path, PathBuf};

use scryrs_types::{ROUTE_SCHEMA_VERSION, RouteManifestDocument};

pub(crate) struct LoadedRouteManifest {
    pub(crate) routes_path: PathBuf,
    pub(crate) routes_json: String,
    pub(crate) manifest: RouteManifestDocument,
}

pub(crate) fn resolve_repo_root(
    err: &mut impl Write,
    command: &str,
    path: &str,
) -> Result<PathBuf, i32> {
    std::path::absolute(path).map_err(|error| {
        let _ = writeln!(err, "{command}: cannot resolve path '{path}': {error}");
        2
    })
}

pub(crate) fn load_route_manifest(
    err: &mut impl Write,
    command: &str,
    repo_root: &Path,
) -> Result<LoadedRouteManifest, i32> {
    let routes_path = repo_root.join(".scryrs/routes.json");
    let routes_json = match std::fs::read_to_string(&routes_path) {
        Ok(contents) => contents,
        Err(_) => {
            let _ = writeln!(
                err,
                "{command}: route artifact not found at {}",
                routes_path.display()
            );
            let _ = writeln!(
                err,
                "Run `scryrs route <PATH>` first to generate the route manifest."
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return Err(2);
        }
    };

    let manifest: RouteManifestDocument = match serde_json::from_str(&routes_json) {
        Ok(document) => document,
        Err(error) => {
            let _ = writeln!(err, "{command}: malformed route artifact: {error}");
            let _ = writeln!(
                err,
                "Run `scryrs route <PATH>` to regenerate the route manifest."
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return Err(2);
        }
    };

    if manifest.schema_version != ROUTE_SCHEMA_VERSION {
        let _ = writeln!(
            err,
            "{command}: route schema version mismatch: got '{}', expected '{}'",
            manifest.schema_version, ROUTE_SCHEMA_VERSION
        );
        let _ = writeln!(
            err,
            "Run `scryrs route <PATH>` to regenerate the route manifest."
        );
        let _ = writeln!(err, "See `scryrs --help`");
        return Err(2);
    }

    Ok(LoadedRouteManifest {
        routes_path,
        routes_json,
        manifest,
    })
}

pub(crate) fn warn_if_route_artifact_changed(
    err: &mut impl Write,
    command: &str,
    routes_path: &Path,
    original: &str,
) {
    if let Err(error) = check_artifact_unchanged(routes_path, original, err, command) {
        let _ = writeln!(
            err,
            "{command}: warning: cannot verify route artifact unchanged: {error}"
        );
    }
}

fn check_artifact_unchanged(
    routes_path: &Path,
    original: &str,
    err: &mut impl Write,
    command: &str,
) -> std::io::Result<()> {
    std::fs::read_to_string(routes_path).map(|after| {
        if after != original {
            let _ = writeln!(
                err,
                "{command}: warning: route artifact was modified during execution"
            );
        }
    })
}
