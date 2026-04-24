use std::path::PathBuf;

use crate::settings;

#[cfg(target_os = "windows")]
const AUTOSTART_VALUE_NAME: &str = "dictation-rs";

pub fn sync_from_settings() -> Result<(), String> {
  let enable = settings::current().start_on_login;
  if enable {
    enable_autostart()
  } else {
    disable_autostart()
  }
}

fn executable_path() -> Result<PathBuf, String> {
  std::env::current_exe()
    .map_err(|e| format!("resolve current exe failed: {e}"))
}

#[cfg(target_os = "windows")]
fn enable_autostart() -> Result<(), String> {
  use winreg::RegKey;
  use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};

  let exe = executable_path()?;
  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  let run_key = hkcu
    .open_subkey_with_flags(
      "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
      KEY_SET_VALUE,
    )
    .map_err(|e| format!("open Run key failed: {e}"))?;

  // Quote executable path to safely handle spaces.
  let value = format!("\"{}\"", exe.display());
  run_key
    .set_value(AUTOSTART_VALUE_NAME, &value)
    .map_err(|e| format!("set Run value failed: {e}"))
}

#[cfg(target_os = "windows")]
fn disable_autostart() -> Result<(), String> {
  use winreg::RegKey;
  use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};

  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  let run_key = hkcu
    .open_subkey_with_flags(
      "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
      KEY_SET_VALUE,
    )
    .map_err(|e| format!("open Run key failed: {e}"))?;

  // If value is absent, treat as success.
  match run_key.delete_value(AUTOSTART_VALUE_NAME) {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(format!("delete Run value failed: {e}")),
  }
}

#[cfg(target_os = "linux")]
fn enable_autostart() -> Result<(), String> {
  use std::fs;

  let exe = executable_path()?;
  let path = linux_autostart_file()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("create autostart dir failed: {e}"))?;
  }

  let content = format!(
    "[Desktop Entry]\nType=Application\nName=dictation-rs\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
    exe.display()
  );
  fs::write(path, content)
    .map_err(|e| format!("write autostart file failed: {e}"))
}

#[cfg(target_os = "linux")]
fn disable_autostart() -> Result<(), String> {
  use std::fs;

  let path = linux_autostart_file()?;
  if path.exists() {
    fs::remove_file(path)
      .map_err(|e| format!("remove autostart file failed: {e}"))
  } else {
    Ok(())
  }
}

#[cfg(target_os = "linux")]
fn linux_autostart_file() -> Result<PathBuf, String> {
  let home = directories::BaseDirs::new()
    .map(|b| b.home_dir().to_path_buf())
    .ok_or_else(|| "unable to resolve home directory".to_string())?;
  Ok(
    home
      .join(".config")
      .join("autostart")
      .join("dictation-rs.desktop"),
  )
}

#[cfg(target_os = "macos")]
fn enable_autostart() -> Result<(), String> {
  use std::fs;

  let exe = executable_path()?;
  let path = macos_launch_agent_file()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("create launch agents dir failed: {e}"))?;
  }

  let content = format!(
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.dictation.dictation</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>ProcessType</key>
  <string>Background</string>
  <key>LSBackgroundOnly</key>
  <true/>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#,
    exe.display()
  );
  fs::write(path, content)
    .map_err(|e| format!("write launch agent failed: {e}"))
}

#[cfg(target_os = "macos")]
fn disable_autostart() -> Result<(), String> {
  use std::fs;

  let path = macos_launch_agent_file()?;
  if path.exists() {
    fs::remove_file(path)
      .map_err(|e| format!("remove launch agent failed: {e}"))
  } else {
    Ok(())
  }
}

#[cfg(target_os = "macos")]
fn macos_launch_agent_file() -> Result<PathBuf, String> {
  let home = directories::BaseDirs::new()
    .map(|b| b.home_dir().to_path_buf())
    .ok_or_else(|| "unable to resolve home directory".to_string())?;
  Ok(
    home
      .join("Library")
      .join("LaunchAgents")
      .join("com.dictation.dictation.plist"),
  )
}

#[cfg(not(any(
  target_os = "windows",
  target_os = "linux",
  target_os = "macos"
)))]
fn enable_autostart() -> Result<(), String> {
  Ok(())
}

#[cfg(not(any(
  target_os = "windows",
  target_os = "linux",
  target_os = "macos"
)))]
fn disable_autostart() -> Result<(), String> {
  Ok(())
}
