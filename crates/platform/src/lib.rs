use std::path::{Path, PathBuf};

const SENSITIVE_SUFFIXES: &[&str] = &[
    ".ssh",
    ".gnupg",
    "Library/Keychains",
    "Library/Application Support/1Password",
    "Library/Application Support/Google/Chrome",
    "Library/Application Support/Firefox",
    "AppData/Roaming/Microsoft/Credentials",
    "AppData/Roaming/1Password",
    "AppData/Local/Google/Chrome/User Data",
];

pub fn is_sensitive_path(path: impl AsRef<Path>) -> bool {
    let normalized = normalize(path.as_ref());
    SENSITIVE_SUFFIXES
        .iter()
        .any(|suffix| normalized.ends_with(&normalize(Path::new(suffix))))
}

pub fn default_sensitive_paths(home: impl AsRef<Path>) -> Vec<PathBuf> {
    let home = home.as_ref();
    SENSITIVE_SUFFIXES
        .iter()
        .map(|suffix| home.join(suffix))
        .collect()
}

fn normalize(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sensitive_suffixes() {
        assert!(is_sensitive_path("/Users/me/.ssh"));
        assert!(is_sensitive_path("/Users/me/Library/Keychains"));
        assert!(!is_sensitive_path("/Users/me/Desktop"));
    }
}
