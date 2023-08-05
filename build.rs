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
            return Some(format!("{}/{}/lib", DIR, name).to_string());
        }
    }

    return None
}

fn main() {
    // libiconv is a struggle on macos
    if let Some(dir) = getdir("libiconv") {
        println!("cargo:rustc-link-search=native={}", dir)
    }
}