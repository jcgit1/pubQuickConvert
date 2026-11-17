//! KDE global shortcut registration via desktop actions and kglobalshortcutsrc.

use anyhow::Result;

pub const SHORTCUT_COMPONENT: &str = "com.quickconvert.QuickConvert";
pub const SHORTCUT_ACTION: &str = "show";

#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
use anyhow::Context;

pub fn is_kde_session() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CURRENT_DESKTOP")
            .map(|v| v.to_ascii_lowercase().contains("kde"))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Apply or clear the global shortcut in KDE's shortcut configuration.
pub fn apply_global_shortcut(enabled: bool, shortcut: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        apply_global_shortcut_linux(enabled, shortcut)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (enabled, shortcut);
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn apply_global_shortcut_linux(enabled: bool, shortcut: &str) -> Result<()> {
    if !is_kde_session() {
        return Ok(());
    }

    let value = if enabled && !shortcut.trim().is_empty() {
        shortcut.trim().to_string()
    } else {
        String::new()
    };

    let status = Command::new("kwriteconfig6")
        .args([
            "--file",
            "kglobalshortcutrc",
            "--group",
            SHORTCUT_COMPONENT,
            "--key",
            SHORTCUT_ACTION,
            &value,
        ])
        .status()
        .context("Failed to run kwriteconfig6 (install kde-cli-tools if missing)")?;

    if !status.success() {
        anyhow::bail!("kwriteconfig6 exited with status {status}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_action_is_non_empty() {
        assert!(!SHORTCUT_ACTION.is_empty());
    }
}
