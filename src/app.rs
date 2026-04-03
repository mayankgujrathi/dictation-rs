use std::collections::VecDeque;
use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU32, Ordering},
};

use eframe::egui;

pub const WINDOW_INNER_SIZE: [f32; 2] = [100.0, 40.0];
pub const HISTORY_LEN: usize = 8;

pub struct VoiceApp {
  volume_atomic: Arc<AtomicU32>,
  is_recording: Arc<AtomicBool>,
  should_exit: Arc<AtomicBool>,
  history: VecDeque<f32>,
  positioned: bool,
}

impl VoiceApp {
  pub fn new(
    volume_atomic: Arc<AtomicU32>,
    is_recording: Arc<AtomicBool>,
    should_exit: Arc<AtomicBool>,
  ) -> Self {
    Self {
      volume_atomic,
      is_recording,
      should_exit,
      history: VecDeque::from(vec![0.0; HISTORY_LEN]),
      positioned: false,
    }
  }
}

impl eframe::App for VoiceApp {
  fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    [0.0, 0.0, 0.0, 0.0]
  }

  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // Check if exit was requested from tray
    if self.should_exit.load(Ordering::SeqCst) {
      ctx.send_viewport_cmd(egui::ViewportCommand::Close);
      return;
    }

    // Check if recording is active
    let is_recording = self.is_recording.load(Ordering::SeqCst);

    // If not recording, don't render but keep checking state
    if !is_recording {
      ctx.request_repaint_after(std::time::Duration::from_millis(100));
      return;
    }

    if !self.positioned {
      if let Some(monitor_res) = ctx.input(|i| i.viewport().monitor_size) {
        let window_size = egui::vec2(WINDOW_INNER_SIZE[0], WINDOW_INNER_SIZE[1]);
        let x = (monitor_res.x - window_size.x) / 2.0;
        let y = (monitor_res.y * 0.9) - window_size.y;
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
        self.positioned = true;
      }
    }

    let current_vol = self.volume_atomic.load(Ordering::Relaxed) as f32 / 1000.0;
    self.history.push_back(current_vol);
    if self.history.len() > HISTORY_LEN {
      self.history.pop_front();
    }

    let my_frame = egui::Frame::none()
      .fill(egui::Color32::BLACK)
      .rounding(50.0)
      .inner_margin(8.0);

    egui::CentralPanel::default()
      .frame(my_frame)
      .show(ctx, |ui| {
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let painter = ui.painter();
        let spacing = rect.width() / self.history.len() as f32;

        for (i, &amp) in self.history.iter().enumerate() {
          let x = rect.left() + (i as f32 * spacing) + (spacing / 2.0);
          let h = (amp * rect.height() * 4.0).clamp(2.0, rect.height() * 0.9);

          painter.line_segment(
            [
              egui::pos2(x, rect.center().y - h / 2.0),
              egui::pos2(x, rect.center().y + h / 2.0),
            ],
            egui::Stroke::new(4.0, egui::Color32::WHITE),
          );
        }
      });

    ctx.request_repaint_after(std::time::Duration::from_millis(50));
  }
}
