use chrono::Local;
use eyre::{Context, Result, eyre};
use log::info;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

struct Entry {
    name: &'static str,
    content: &'static str,
}

const ENTRIES: &[Entry] = &[Entry {
    name: "scottidler",
    content: include_str!("../statusline.d/scottidler.sh"),
}];

const DEFAULT_NAME: &str = "scottidler";

fn claude_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| eyre!("Could not determine home directory"))?;
    Ok(home.join(".claude"))
}

fn find_entry(name: &str) -> Result<&'static Entry> {
    ENTRIES.iter().find(|e| e.name == name).ok_or_else(|| {
        eyre!(
            "Unknown statusline '{}'. Use 'ccu statusline --list' to see available options.",
            name
        )
    })
}

pub fn list() {
    println!("Available statuslines:");
    for entry in ENTRIES {
        let marker = if entry.name == DEFAULT_NAME {
            " (default)"
        } else {
            ""
        };
        println!("  {}{}", entry.name, marker);
    }
}

fn install_to(name: &str, dest_dir: &Path) -> Result<()> {
    let entry = find_entry(name)?;
    let target = dest_dir.join("statusline.sh");

    // Back up existing file
    if target.exists() {
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let backup = dest_dir.join(format!("statusline.sh.{}.bak", timestamp));
        fs::rename(&target, &backup).with_context(|| {
            format!(
                "Failed to back up {} to {}",
                target.display(),
                backup.display()
            )
        })?;
        info!("Backed up existing statusline to {}", backup.display());
        println!("Backed up existing statusline to {}", backup.display());
    }

    // Write new statusline
    fs::write(&target, entry.content)
        .with_context(|| format!("Failed to write {}", target.display()))?;

    // Make executable
    let mut perms = fs::metadata(&target)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&target, perms)?;

    info!("Installed '{}' statusline to {}", name, target.display());
    println!("Installed '{}' statusline to {}", name, target.display());

    Ok(())
}

pub fn install(name: Option<&str>) -> Result<()> {
    let name = name.unwrap_or(DEFAULT_NAME);
    let dest = claude_dir()?;
    install_to(name, &dest)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_entries_not_empty() {
        assert!(!ENTRIES.is_empty());
    }

    #[test]
    fn test_default_entry_exists() {
        assert!(find_entry(DEFAULT_NAME).is_ok());
    }

    #[test]
    fn test_unknown_entry_errors() {
        assert!(find_entry("nonexistent").is_err());
    }

    #[test]
    fn test_install_creates_file() {
        let dir = TempDir::new().unwrap();
        install_to("scottidler", dir.path()).unwrap();

        let target = dir.path().join("statusline.sh");
        assert!(target.exists());

        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("#!/usr/bin/env bash"));

        let perms = fs::metadata(&target).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o755);
    }

    #[test]
    fn test_install_default() {
        let dir = TempDir::new().unwrap();
        // Calling with DEFAULT_NAME should work the same as the public install with None
        install_to(DEFAULT_NAME, dir.path()).unwrap();
        assert!(dir.path().join("statusline.sh").exists());
    }

    #[test]
    fn test_install_backs_up_existing() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("statusline.sh");

        // Create an existing file
        fs::write(&target, "old content").unwrap();
        assert!(target.exists());

        install_to("scottidler", dir.path()).unwrap();

        // Original should be replaced
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("#!/usr/bin/env bash"));
        assert_ne!(content, "old content");

        // Backup should exist
        let backups: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("statusline.sh.")
                    && e.file_name().to_string_lossy().ends_with(".bak")
            })
            .collect();
        assert_eq!(backups.len(), 1);

        let backup_content = fs::read_to_string(backups[0].path()).unwrap();
        assert_eq!(backup_content, "old content");
    }

    #[test]
    fn test_install_unknown_errors() {
        let dir = TempDir::new().unwrap();
        assert!(install_to("nonexistent", dir.path()).is_err());
    }

    #[test]
    fn test_embedded_content_is_executable_script() {
        let entry = find_entry("scottidler").unwrap();
        assert!(entry.content.starts_with("#!/usr/bin/env bash"));
        assert!(entry.content.contains("ccu"));
    }
}
