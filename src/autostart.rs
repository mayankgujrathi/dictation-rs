#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::PathBuf;

use crate::settings;

#[cfg(target_os = "windows")]
const AUTOSTART_VALUE_NAME: &str = "vocoflow";

pub fn sync_from_settings() -> Result<(), String> {
  settings::refresh_from_disk_best_effort("autostart::sync_from_settings");
  let enable = settings::current().start_on_login;
  if enable {
    enable_autostart()
  } else {
    disable_autostart()
  }
}

pub fn sync_settings_from_system() -> Result<(), String> {
  settings::refresh_from_disk_best_effort(
    "autostart::sync_settings_from_system",
  );
  let enabled = system_autostart_enabled()?;
  let _ = settings::persist_start_on_login_from_system(enabled)?;
  Ok(())
}

fn system_autostart_enabled() -> Result<bool, String> {
  is_autostart_enabled()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn executable_path() -> Result<PathBuf, String> {
  std::env::current_exe()
    .map_err(|e| format!("resolve current exe failed: {e}"))
}

#[cfg(target_os = "windows")]
fn enable_autostart() -> Result<(), String> {
  ensure_windows_run_entry_present()?;

  // Windows Task Manager startup toggles are tracked via StartupApproved\Run.
  // Removing the value returns the entry to enabled/default state.
  clear_windows_startup_approved_state()
}

#[cfg(target_os = "windows")]
fn disable_autostart() -> Result<(), String> {
  ensure_windows_run_entry_present()?;

  // 0x03 is a known disabled flag in StartupApproved\Run.
  set_windows_startup_approved_state_disabled()
}

#[cfg(target_os = "windows")]
fn ensure_windows_run_entry_present() -> Result<(), String> {
  use winreg::RegKey;
  use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  let run_key = hkcu
    .open_subkey_with_flags(
      "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
      KEY_READ,
    )
    .map_err(|e| format!("open Run key failed: {e}"))?;

  let run_value: Result<String, _> = run_key.get_value(AUTOSTART_VALUE_NAME);
  match run_value {
    Ok(_) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(
      "Windows autostart registration is missing. Please reinstall Vocoflow to repair the startup entry."
        .to_string(),
    ),
    Err(e) => Err(format!("read Run value failed: {e}")),
  }
}

#[cfg(target_os = "windows")]
fn clear_windows_startup_approved_state() -> Result<(), String> {
  use winreg::RegKey;
  use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};

  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  let approved_key = hkcu
    .open_subkey_with_flags(
      "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run",
      KEY_SET_VALUE,
    )
    .map_err(|e| format!("open StartupApproved key failed: {e}"))?;

  match approved_key.delete_value(AUTOSTART_VALUE_NAME) {
    Ok(()) => Ok(()),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(format!("delete StartupApproved value failed: {e}")),
  }
}

#[cfg(target_os = "windows")]
fn set_windows_startup_approved_state_disabled() -> Result<(), String> {
  use winreg::RegKey;
  use winreg::RegValue;
  use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE, REG_BINARY};

  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  let approved_key = hkcu
    .open_subkey_with_flags(
      "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run",
      KEY_SET_VALUE,
    )
    .map_err(|e| format!("open StartupApproved key failed: {e}"))?;

  // 12 bytes aligns with common StartupApproved payload length used by Windows.
  let disabled = RegValue {
    vtype: REG_BINARY,
    bytes: vec![
      0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ],
  };
  approved_key
    .set_raw_value(AUTOSTART_VALUE_NAME, &disabled)
    .map_err(|e| format!("set StartupApproved disabled state failed: {e}"))
}

#[cfg(target_os = "windows")]
fn is_autostart_enabled() -> Result<bool, String> {
  use winreg::RegKey;
  use winreg::RegValue;
  use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  let run_key = hkcu
    .open_subkey_with_flags(
      "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
      KEY_READ,
    )
    .map_err(|e| format!("open Run key failed: {e}"))?;

  let run_value: Result<String, _> = run_key.get_value(AUTOSTART_VALUE_NAME);
  if run_value.is_err() {
    return Ok(false);
  }

  // Task Manager startup toggles are reflected via StartupApproved\Run.
  let approved = hkcu.open_subkey_with_flags(
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run",
    KEY_READ,
  );

  let Ok(approved_key) = approved else {
    return Ok(true);
  };

  let state: Result<RegValue, _> =
    approved_key.get_raw_value(AUTOSTART_VALUE_NAME);
  let Ok(state) = state else {
    return Ok(true);
  };

  let bytes = state.bytes;
  let flag = bytes.first().copied().unwrap_or_default();
  // Known disabled flags in StartupApproved: 0x03/0x07.
  if flag == 0x03 || flag == 0x07 {
    Ok(false)
  } else {
    Ok(true)
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
    "[Desktop Entry]\nType=Application\nName=vocoflow\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
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
      .join("vocoflow.desktop"),
  )
}

#[cfg(target_os = "linux")]
fn is_autostart_enabled() -> Result<bool, String> {
  Ok(linux_autostart_file()?.exists())
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
  <string>com.vocoflow.vocoflow</string>
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
      .join("com.vocoflow.vocoflow.plist"),
  )
}

#[cfg(target_os = "macos")]
fn is_autostart_enabled() -> Result<bool, String> {
  Ok(macos_launch_agent_file()?.exists())
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
fn is_autostart_enabled() -> Result<bool, String> {
  Ok(false)
}

#[cfg(not(any(
  target_os = "windows",
  target_os = "linux",
  target_os = "macos"
)))]
fn disable_autostart() -> Result<(), String> {
  Ok(())
}
