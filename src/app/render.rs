use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use eframe::egui;

use super::{HISTORY_LEN, UIState, VoiceApp, WINDOW_INNER_SIZE};

impl VoiceApp {
  fn enter_idle_mode(&mut self, ctx: &egui::Context) {
    // Keep viewport/event loop alive (for hotkey + tray exit responsiveness),
    // but make UI effectively non-intrusive and low-cost.
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
      1.0, 1.0,
    )));
    ctx.request_repaint_after(Duration::from_millis(150));
  }

  fn reset_transcription_cycle(&mut self) {
    self.transcription_spawned = false;
    self.transcription_rendered_at = None;
    self.saw_recording_active = false;
    if let Ok(mut slot) = self.transcription_status.lock() {
      *slot = None;
    }
  }

  fn update_model_downloading(
    &mut self,
    ctx: &egui::Context,
    my_frame: egui::Frame,
  ) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
      WINDOW_INNER_SIZE[0],
      WINDOW_INNER_SIZE[1],
    )));
    self.spawn_model_download_worker_if_needed();

    let progress_raw = self.download_progress_atomic.load(Ordering::Relaxed);
    let progress = (progress_raw as f32 / 100.0).clamp(0.0, 1.0);
    egui::CentralPanel::default()
      .frame(my_frame)
      .show(ctx, |ui| {
        let rect = ui.max_rect();
        let radius = (rect.height() * 0.5).max(1.0);

        ui.painter().rect_filled(
          rect,
          radius,
          egui::Color32::from_rgb(24, 24, 24),
        );

        let filled_width = (rect.width() * progress).clamp(0.0, rect.width());
        if filled_width > 0.0 {
          let filled_rect = egui::Rect::from_min_max(
            rect.left_top(),
            egui::pos2(rect.left() + filled_width, rect.bottom()),
          );
          ui.painter().rect_filled(
            filled_rect,
            radius,
            egui::Color32::from_rgb(40, 120, 255),
          );
        }

        ui.painter().text(
          rect.center(),
          egui::Align2::CENTER_CENTER,
          format!("{}%", (progress * 100.0).round() as u32),
          egui::FontId::proportional(12.0),
          egui::Color32::WHITE,
        );
      });

    if progress >= 1.0 {
      self.ui_state = UIState::VisualizerRecording;
    }

    ctx.request_repaint_after(Duration::from_millis(50));
  }

  fn update_visualizer_recording(
    &mut self,
    ctx: &egui::Context,
    my_frame: egui::Frame,
  ) {
    let is_recording = self.is_recording.load(Ordering::SeqCst);
    let mic_ready = self.mic_ready.load(Ordering::SeqCst);
    let recording_ready = self.recording_ready.load(Ordering::SeqCst);
    let actively_recording = is_recording && mic_ready;

    if actively_recording {
      ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
      ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
        WINDOW_INNER_SIZE[0],
        WINDOW_INNER_SIZE[1],
      )));
      ctx.request_repaint();
      self.saw_recording_active = true;
    }

    if !actively_recording {
      if self.saw_recording_active {
        if recording_ready {
          self.ui_state = UIState::Transcribing;
          ctx.request_repaint();
        } else {
          // Wait until current recording output is finalized before transcribing,
          // to avoid transcribing a stale previous file.
          self.enter_idle_mode(ctx);
        }
      } else {
        // Idle state: keep loop responsive while minimizing resource usage.
        self.enter_idle_mode(ctx);
      }
      return;
    }

    let current_vol =
      self.volume_atomic.load(Ordering::Relaxed) as f32 / 1000.0;
    self.history.push_back(current_vol);
    if self.history.len() > HISTORY_LEN {
      self.history.pop_front();
    }

    egui::CentralPanel::default()
      .frame(my_frame)
      .show(ctx, |ui| {
        let (rect, _) =
          ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        if rect.height() <= 0.0 || rect.width() <= 0.0 {
          return;
        }
        let painter = ui.painter();
        let spacing = rect.width() / self.history.len() as f32;

        for (i, &amp) in self.history.iter().enumerate() {
          let x = rect.left() + (i as f32 * spacing) + (spacing / 2.0);
          let max_h = (rect.height() * 0.9).max(2.0);
          let h = (amp * rect.height() * 4.0).clamp(2.0, max_h);

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

  fn update_transcribing(
    &mut self,
    ctx: &egui::Context,
    my_frame: egui::Frame,
  ) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
      100.0, 40.0,
    )));
    self.spawn_transcription_worker_if_needed();

    let status_opt =
      self.transcription_status.lock().ok().and_then(|slot| *slot);

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
            ui.label(egui::RichText::new(text_to_show).color(
              if text_to_show == "Error" {
                egui::Color32::RED
              } else {
                egui::Color32::WHITE
              },
            ));
          },
        );
      });

    match status_opt {
      Some(false) => {
        self.reset_transcription_cycle();
        self.ui_state = UIState::VisualizerRecording;
        self.enter_idle_mode(ctx);
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
          self.reset_transcription_cycle();
          self.ui_state = UIState::VisualizerRecording;
          self.enter_idle_mode(ctx);
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
      .rounding((WINDOW_INNER_SIZE[1] * 0.5).max(1.0))
      .inner_margin(8.0);

    match self.ui_state {
      UIState::ModelDownloading => self.update_model_downloading(ctx, my_frame),
      UIState::VisualizerRecording => {
        self.update_visualizer_recording(ctx, my_frame)
      }
      UIState::Transcribing => self.update_transcribing(ctx, my_frame),
    }
  }
}
