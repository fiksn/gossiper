use std::fs;

// getdir will search for library with given pattern in nix store
fn getdir(pattern: &str) -> Option<String> {
    const DIR: &str = "/nix/store";

    let entries = fs::read_dir(DIR).ok()?;

    for entry in entries {
        let entry = entry.ok()?;
        let entry_path = entry.path();

        if !entry_path.is_dir() || entry_path.file_name() == None {
            continue;
        }

        let name = entry_path.file_name()?.to_str()?;

        if name.contains(pattern) {
            return Some(format!("{}/{}", DIR, name).to_string());
        }
    }

    None
}

fn main() {
    if cfg!(target_os = "macos") {
        // When I use rust from nix this does not work else
        if let Some(dir) = getdir("libiconv") {
            println!("cargo:rustc-link-search=native={}/lib", dir)
        }
        if let Some(dir) = getdir("apple-framework-CoreFoundation") {
            println!(
                "cargo:rustc-link-search=framework={}/Library/Frameworks/",
                dir
            )
        }
    }
}
