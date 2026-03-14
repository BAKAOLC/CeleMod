use std::{
    env,
    fs::File,
    io::Read,
    path::Path,
    process::Command,
    time::SystemTime,
};

fn dir_has_newer(dir: &str, than: SystemTime, skip: &[&str]) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if skip.contains(&name_str.as_ref()) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            if dir_has_newer(path.to_str().unwrap(), than, skip) {
                return true;
            }
        } else if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified > than {
                    return true;
                }
            }
        }
    }
    false
}

fn build_ui() {
    // Tell Cargo to re-run this script when UI sources change
    println!("cargo:rerun-if-changed=src/celemod-ui");
    println!("cargo:rerun-if-changed=resources/dist.rc");

    let dist_rc = Path::new("resources/dist.rc");

    let needs_build = if !dist_rc.exists() {
        println!("cargo:warning=resources/dist.rc not found, building UI...");
        true
    } else {
        let dist_time = dist_rc.metadata().unwrap().modified().unwrap();
        let stale = dir_has_newer("src/celemod-ui", dist_time, &["node_modules", "dist"]);
        if stale {
            println!("cargo:warning=UI sources newer than dist.rc, rebuilding UI...");
        }
        stale
    };

    if needs_build {
        let status = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "yarn", "build"])
                .current_dir("src/celemod-ui")
                .status()
        } else {
            Command::new("yarn")
                .arg("build")
                .current_dir("src/celemod-ui")
                .status()
        };
        let status = status.expect("failed to spawn `yarn build`");
        assert!(status.success(), "`yarn build` exited with non-zero status");
    }
}

fn main() {
    build_ui();

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    let mut version = "".to_string();
    File::open("version.txt")
        .unwrap()
        .read_to_string(&mut version)
        .unwrap();
    println!("cargo:rustc-env=VERSION={}", version);
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    // possible architecture: win-x64 win-x86 linux osx
    let target = std::env::var("TARGET").unwrap();
    let target = target.split('-').collect::<Vec<_>>();
    let target = target[2];
    let target = match target {
        "windows" => {
            let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
            match arch.as_str() {
                "x86_64" => "win-x64",
                "x86" => "win-x86",
                _ => panic!("Unsupported target"),
            }
        }
        "linux" => "linux",
        "darwin" => "osx",
        _ => panic!("Unsupported target"),
    };
    println!("cargo:rustc-env=TARGET={}", target);

    use winres::WindowsResource;

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            .set_icon("icon.ico")
            .compile().unwrap();
    }
}
