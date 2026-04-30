use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButtonToken {
  Left,
  Right,
  Middle,
  Button4,
  Button5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyToken {
  Backquote,
  Space,
  Enter,
  Tab,
  Escape,
  Char(char),
  Function(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TriggerInput {
  Key(KeyToken),
  Mouse(MouseButtonToken),
}

#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize,
)]
pub struct Modifiers {
  pub ctrl: bool,
  pub shift: bool,
  pub alt: bool,
  pub meta: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotkeyStep {
  pub modifiers: Modifiers,
  pub trigger: TriggerInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedHotkey {
  pub normalized: String,
  pub sequence: Vec<HotkeyStep>,
}

impl Default for ParsedHotkey {
  fn default() -> Self {
    parse_hotkey_binding("Ctrl+`").unwrap_or(Self {
      normalized: "Ctrl+`".to_string(),
      sequence: Vec::new(),
    })
  }
}

fn parse_key_token(token: &str) -> Option<KeyToken> {
  let lower = token.trim().to_ascii_lowercase();
  match lower.as_str() {
    "`" | "backquote" | "grave" => Some(KeyToken::Backquote),
    "space" => Some(KeyToken::Space),
    "tab" => Some(KeyToken::Tab),
    "enter" | "return" => Some(KeyToken::Enter),
    "escape" | "esc" => Some(KeyToken::Escape),
    _ => {
      if let Some(stripped) = lower.strip_prefix('f') {
        let n = stripped.parse::<u8>().ok()?;
        if (1..=24).contains(&n) {
          return Some(KeyToken::Function(n));
        }
      }
      if lower.len() == 1 {
        return lower.chars().next().map(KeyToken::Char);
      }
      None
    }
  }
}

fn parse_mouse_token(token: &str) -> Option<MouseButtonToken> {
  match token.trim().to_ascii_lowercase().as_str() {
    "mouseleft" | "leftmouse" | "mouse1" => Some(MouseButtonToken::Left),
    "mouseright" | "rightmouse" | "mouse2" => Some(MouseButtonToken::Right),
    "mousemiddle" | "middlemouse" | "mouse3" => Some(MouseButtonToken::Middle),
    "mouse4" | "x1" => Some(MouseButtonToken::Button4),
    "mouse5" | "x2" => Some(MouseButtonToken::Button5),
    _ => None,
  }
}

fn normalize_step(step: &HotkeyStep) -> String {
  let mut parts = Vec::new();
  if step.modifiers.ctrl {
    parts.push("Ctrl".to_string());
  }
  if step.modifiers.shift {
    parts.push("Shift".to_string());
  }
  if step.modifiers.alt {
    parts.push("Alt".to_string());
  }
  if step.modifiers.meta {
    parts.push("Meta".to_string());
  }
  let trigger = match step.trigger {
    TriggerInput::Key(KeyToken::Backquote) => "`".to_string(),
    TriggerInput::Key(KeyToken::Space) => "Space".to_string(),
    TriggerInput::Key(KeyToken::Enter) => "Enter".to_string(),
    TriggerInput::Key(KeyToken::Tab) => "Tab".to_string(),
    TriggerInput::Key(KeyToken::Escape) => "Escape".to_string(),
    TriggerInput::Key(KeyToken::Char(c)) => c.to_ascii_uppercase().to_string(),
    TriggerInput::Key(KeyToken::Function(n)) => format!("F{n}"),
    TriggerInput::Mouse(MouseButtonToken::Left) => "MouseLeft".to_string(),
    TriggerInput::Mouse(MouseButtonToken::Right) => "MouseRight".to_string(),
    TriggerInput::Mouse(MouseButtonToken::Middle) => "MouseMiddle".to_string(),
    TriggerInput::Mouse(MouseButtonToken::Button4) => "Mouse4".to_string(),
    TriggerInput::Mouse(MouseButtonToken::Button5) => "Mouse5".to_string(),
  };
  parts.push(trigger);
  parts.join("+")
}

pub fn parse_hotkey_binding(input: &str) -> Result<ParsedHotkey, String> {
  let mut sequence = Vec::new();
  for raw_step in input.split(',') {
    let raw_step = raw_step.trim();
    if raw_step.is_empty() {
      return Err("hotkey contains an empty chord step".to_string());
    }
    let mut mods = Modifiers::default();
    let mut trigger: Option<TriggerInput> = None;
    for part in raw_step.split('+') {
      let t = part.trim();
      if t.is_empty() {
        return Err("hotkey has empty token".to_string());
      }
      match t.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => mods.ctrl = true,
        "shift" => mods.shift = true,
        "alt" | "option" => mods.alt = true,
        "meta" | "cmd" | "command" | "win" | "super" => mods.meta = true,
        _ => {
          let next_trigger = if let Some(k) = parse_key_token(t) {
            TriggerInput::Key(k)
          } else if let Some(m) = parse_mouse_token(t) {
            TriggerInput::Mouse(m)
          } else {
            return Err(format!("unsupported token '{t}' in hotkey"));
          };
          if trigger.is_some() {
            return Err(
              "each hotkey step can only have one trigger key/button"
                .to_string(),
            );
          }
          trigger = Some(next_trigger);
        }
      }
    }
    let trigger = trigger
      .ok_or_else(|| "hotkey step is missing trigger key/button".to_string())?;
    sequence.push(HotkeyStep {
      modifiers: mods,
      trigger,
    });
  }
  if sequence.is_empty() {
    return Err("hotkey cannot be empty".to_string());
  }
  let normalized = sequence
    .iter()
    .map(normalize_step)
    .collect::<Vec<_>>()
    .join(", ");
  Ok(ParsedHotkey {
    normalized,
    sequence,
  })
}

#[derive(Debug, Clone)]
pub struct HotkeyMatcher {
  sequence: Vec<HotkeyStep>,
  timeout: Duration,
  index: usize,
  last_match_at: Option<Instant>,
}

impl HotkeyMatcher {
  pub fn new(sequence: Vec<HotkeyStep>, timeout_ms: u64) -> Self {
    Self {
      sequence,
      timeout: Duration::from_millis(timeout_ms.max(100)),
      index: 0,
      last_match_at: None,
    }
  }

  pub fn register_trigger(
    &mut self,
    modifiers: Modifiers,
    trigger: TriggerInput,
    now: Instant,
  ) -> bool {
    if self.sequence.is_empty() {
      return false;
    }
    if let Some(last) = self.last_match_at
      && now.duration_since(last) > self.timeout
    {
      self.index = 0;
      self.last_match_at = None;
    }

    let expected = &self.sequence[self.index];
    let matched =
      expected.modifiers == modifiers && expected.trigger == trigger;

    if matched {
      self.index += 1;
      self.last_match_at = Some(now);
      if self.index >= self.sequence.len() {
        self.index = 0;
        self.last_match_at = None;
        return true;
      }
      return false;
    }

    // Restart if this input matches the first step.
    let first = &self.sequence[0];
    if first.modifiers == modifiers && first.trigger == trigger {
      self.index = 1;
      self.last_match_at = Some(now);
      return self.sequence.len() == 1;
    }

    self.index = 0;
    self.last_match_at = None;
    false
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_keyboard_and_mouse_chord() {
    let parsed = parse_hotkey_binding("Ctrl+K, Mouse4").expect("valid binding");
    assert_eq!(parsed.normalized, "Ctrl+K, Mouse4");
    assert_eq!(parsed.sequence.len(), 2);
  }

  #[test]
  fn matcher_completes_sequence() {
    let parsed = parse_hotkey_binding("Ctrl+K, C").expect("valid binding");
    let mut m = HotkeyMatcher::new(parsed.sequence, 1200);

    let now = Instant::now();
    assert!(!m.register_trigger(
      Modifiers {
        ctrl: true,
        ..Default::default()
      },
      TriggerInput::Key(KeyToken::Char('k')),
      now,
    ));

    assert!(m.register_trigger(
      Modifiers::default(),
      TriggerInput::Key(KeyToken::Char('c')),
      now + Duration::from_millis(100),
    ));
  }
}
