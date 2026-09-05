use std::path::{Path, PathBuf};
use std::process::ExitCode;

const REQUIRED: &[&str] = &[
    "app.css",
    "app.js",
    "favicon.svg",
    "i18n.js",
    "index.html",
    "login.html",
];

fn main() -> ExitCode {
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist");
    println!("cargo:rerun-if-changed={}", dist.display());
    for name in REQUIRED {
        println!("cargo:rerun-if-changed={}", dist.join(name).display());
    }

    let missing: Vec<&str> = REQUIRED
        .iter()
        .copied()
        .filter(|name| !dist.join(name).is_file())
        .collect();
    if missing.is_empty() {
        return ExitCode::SUCCESS;
    }

    let dist_disp = display_path(&dist);
    eprintln!("error: web UI is not built; missing in {dist_disp}:");
    for name in missing {
        eprintln!("  - {name}");
    }
    eprintln!(
        "Static assets live in web/public/; Vite writes compile products to web/dist/.\n\
         Build the UI first:\n\
           cd web && npm ci && npm run build"
    );
    ExitCode::FAILURE
}

fn display_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}
