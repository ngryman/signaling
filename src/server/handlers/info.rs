use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use crate::server::state::ServerState;

pub(crate) async fn info(State(state): State<ServerState>) -> impl IntoResponse {
  let peers = serde_json::to_value(state.signaling.peers().to_vec()).unwrap();
  let rooms = serde_json::to_value(state.signaling.rooms().to_vec()).unwrap();
  let json = json!({ "peers": peers, "rooms": rooms });
  Json(json)
}
