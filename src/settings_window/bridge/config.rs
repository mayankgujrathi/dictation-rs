use crate::route_table;

use super::lib::RouteDef;

pub static ROUTES: &[RouteDef] = route_table!(
  (
    "POST",
    "/settings/ping",
    super::features::settings_ping::handle
  ),
  (
    "POST",
    "/settings/concat",
    super::features::settings_concat::handle
  ),
);
