use anyhow::Result;
use std::path::PathBuf;

fn xdg_config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

fn desktop_file_path(app_id: &str) -> Option<PathBuf> {
    xdg_config_dir().map(|dir| dir.join("autostart").join(format!("{}.desktop", app_id)))
}

pub fn set_auto_launch(app_id: &str, enabled: bool) -> Result<()> {
    let desktop_file = desktop_file_path(app_id)
        .ok_or_else(|| anyhow::anyhow!("Could not determine XDG config directory"))?;

    if enabled {
        if let Some(parent) = desktop_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let exe_path = std::env::current_exe()?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName={}\nExec={}\nX-GNOME-Autostart-enabled=true\n",
            app_id,
            exe_path.display()
        );
        std::fs::write(&desktop_file, content)?;
    } else {
        let _ = std::fs::remove_file(&desktop_file);
    }

    Ok(())
}

pub fn is_auto_launch_enabled(app_id: &str) -> bool {
    desktop_file_path(app_id).is_some_and(|path| path.exists())
}
