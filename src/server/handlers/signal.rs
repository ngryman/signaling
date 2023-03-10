use anyhow::{bail, Result};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{error, info, instrument};

use crate::server::event::Event;
use crate::server::state::ServerState;
use crate::signaling::PeerId;
use crate::Signaling;

pub(crate) async fn signal(
  ws: WebSocketUpgrade,
  State(state): State<ServerState>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: ServerState) {
  let (ws_sender, mut ws_receiver) = socket.split();
  let (sender, receiver) = mpsc::unbounded_channel();
  let peer_id = state.signaling.add_peer(sender);
  info!("{peer_id} connected");

  tokio::spawn(UnboundedReceiverStream::new(receiver).forward(ws_sender));

  while let Some(Ok(message)) = ws_receiver.next().await {
    if let Message::Close(_) = message {
      info!("{peer_id} left");
      break;
    }

    if let Err(e) = handle_message(message, peer_id, &state.signaling).await {
      error!("{e}")
    }
  }

  if let Err(e) = state.signaling.remove_peer(peer_id) {
    error!("{e}");
  }
}

#[instrument(name = "message", skip_all, fields(peer = peer_id.to_string()))]
async fn handle_message(message: Message, peer_id: PeerId, signaling: &Signaling) -> Result<()> {
  match message {
    Message::Text(payload) => handle_event(payload, peer_id, signaling).await,
    Message::Binary(_) => bail!("unsupported binary message"),
    Message::Ping(payload) => todo!(),
    Message::Pong(payload) => todo!(),
    _ => unreachable!(),
  }
}

async fn handle_event(payload: String, peer_id: PeerId, signaling: &Signaling) -> Result<()> {
  let event: Event = payload.parse()?;
  info!("{event}");

  match event {
    Event::Subscribe { room_ids } => {
      for room_id in room_ids {
        signaling.join_room(peer_id, room_id)?;
      }
    }
    Event::Unsubscribe { room_ids } => {
      for room_id in room_ids {
        signaling.leave_room(peer_id, room_id)?;
      }
    }
    Event::Publish { room_id } => signaling.broadcast(peer_id, room_id, payload)?,
    Event::Ping => signaling.send(peer_id, Event::Pong.to_string())?,
    _ => unreachable!(),
  }

  Ok(())
}
