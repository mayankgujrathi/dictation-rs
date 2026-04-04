use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use eframe::egui;

use super::{HISTORY_LEN, UIState, VoiceApp};

impl VoiceApp {
  fn update_model_downloading(&mut self, ctx: &egui::Context, my_frame: egui::Frame) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    self.spawn_model_download_worker_if_needed();

    let progress_raw = self.download_progress_atomic.load(Ordering::Relaxed);
    let progress = (progress_raw as f32 / 100.0).clamp(0.0, 1.0);

    egui::CentralPanel::default()
      .frame(my_frame)
      .show(ctx, |ui| {
        ui.vertical_centered(|ui| {
          let response = ui.add_sized(
            [ui.available_width(), ui.available_height().max(16.0)],
            egui::ProgressBar::new(progress).fill(egui::Color32::from_rgb(40, 120, 255)),
          );

          ui.painter().text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}%", (progress * 100.0).round() as u32),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
          );
        });
      });

    if progress >= 1.0 {
      self.ui_state = UIState::VisualizerRecording;
    }

    ctx.request_repaint_after(Duration::from_millis(50));
  }

  fn update_visualizer_recording(&mut self, ctx: &egui::Context, my_frame: egui::Frame) {
    let is_recording = self.is_recording.load(Ordering::SeqCst);

    if is_recording {
      ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
      self.saw_recording_active = true;
    }

    if !is_recording {
      if self.saw_recording_active {
        self.ui_state = UIState::Transcribing;
      } else {
        ctx.request_repaint_after(Duration::from_millis(100));
      }
      return;
    }

    let current_vol = self.volume_atomic.load(Ordering::Relaxed) as f32 / 1000.0;
    self.history.push_back(current_vol);
    if self.history.len() > HISTORY_LEN {
      self.history.pop_front();
    }

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

    ctx.request_repaint_after(Duration::from_millis(50));
  }

  fn update_transcribing(&mut self, ctx: &egui::Context, my_frame: egui::Frame) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(100.0, 40.0)));
    self.spawn_transcription_worker_if_needed();

    let status_opt = self.transcription_status.lock().ok().and_then(|slot| *slot);

    let text_to_show = if status_opt == Some(true) {
      "Error"
    } else {
      "Transcribing..."
    };

    egui::CentralPanel::default()
      .frame(my_frame)
      .show(ctx, |ui| {
        ui.with_layout(
          egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
          |ui| {
            ui.label(
              egui::RichText::new(text_to_show).color(if text_to_show == "Error" {
                egui::Color32::RED
              } else {
                egui::Color32::WHITE
              }),
            );
          },
        );
      });

    match status_opt {
      Some(false) => {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        return;
      }
      Some(true) => {
        if self.transcription_rendered_at.is_none() {
          self.transcription_rendered_at = Some(Instant::now());
        } else if self
          .transcription_rendered_at
          .map(|t| t.elapsed() >= Duration::from_secs(1))
          .unwrap_or(false)
        {
          ctx.send_viewport_cmd(egui::ViewportCommand::Close);
          return;
        }
      }
      None => {}
    }

    ctx.request_repaint_after(Duration::from_millis(50));
  }
}

impl eframe::App for VoiceApp {
  fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    [0.0, 0.0, 0.0, 0.0]
  }

  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    if self.should_exit.load(Ordering::SeqCst) {
      ctx.send_viewport_cmd(egui::ViewportCommand::Close);
      return;
    }

    self.ensure_positioned(ctx);

    let my_frame = egui::Frame::none()
      .fill(egui::Color32::BLACK)
      .rounding(50.0)
      .inner_margin(8.0);

    match self.ui_state {
      UIState::ModelDownloading => self.update_model_downloading(ctx, my_frame),
      UIState::VisualizerRecording => self.update_visualizer_recording(ctx, my_frame),
      UIState::Transcribing => self.update_transcribing(ctx, my_frame),
    }
  }
}
