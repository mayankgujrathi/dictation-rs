use eframe::egui;

use super::{VoiceApp, WINDOW_INNER_SIZE};

impl VoiceApp {
  pub(crate) fn ensure_positioned(
    &mut self,
    ctx: &egui::Context,
  ) {
    if self.positioned {
      return;
    }

    if let Some(monitor_res) =
      ctx.input(|i| i.viewport().monitor_size)
    {
      let window_size = egui::vec2(
        WINDOW_INNER_SIZE[0],
        WINDOW_INNER_SIZE[1],
      );
      let x = (monitor_res.x - window_size.x) / 2.0;
      let y = (monitor_res.y * 0.9) - window_size.y;
      ctx.send_viewport_cmd(
        egui::ViewportCommand::OuterPosition(egui::pos2(
          x, y,
        )),
      );
      self.positioned = true;
    }
  }
}
