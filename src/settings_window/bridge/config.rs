use crate::route_table;

use super::lib::RouteDef;

pub static ROUTES: &[RouteDef] = route_table!(
  (
    "GET",
    "/settings",
    super::features::settings_get_all::handle
  ),
  (
    "GET",
    "/settings/start_on_login",
    super::features::settings_get_start_on_login::handle
  ),
  (
    "GET",
    "/settings/logging",
    super::features::settings_get_logging::handle
  ),
  (
    "GET",
    "/settings/transcription",
    super::features::settings_get_transcription::handle
  ),
  (
    "POST",
    "/settings/update/start_on_login",
    super::features::settings_update_start_on_login::handle
  ),
  (
    "POST",
    "/settings/update/logging",
    super::features::settings_update_logging::handle
  ),
  (
    "POST",
    "/settings/update/transcription",
    super::features::settings_update_transcription::handle
  ),
);
