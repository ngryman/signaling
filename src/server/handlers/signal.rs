use std::time::Duration;

use anyhow::{bail, Result};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::Error;
use futures_util::stream::SplitSink;
use futures_util::StreamExt;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::{IntervalStream, UnboundedReceiverStream};
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
  let peer_id = state.signaling.add_peer(sender.clone());
  info!("{peer_id} connected");

  let channel_task = spawn_channel(receiver, ws_sender);
  let heartbeat_task = spawn_heartbeat(peer_id, sender.clone(), state.signaling.clone());

  while let Some(Ok(message)) = ws_receiver.next().await {
    if let Message::Close(_) = message {
      info!("{peer_id} left");
      break;
    }

    if let Err(e) = handle_message(message, peer_id, &state.signaling) {
      error!("{e}")
    }
  }

  if let Err(e) = state.signaling.remove_peer(peer_id) {
    error!("{e}");
  }

  channel_task.abort();
  heartbeat_task.abort();
}

fn spawn_channel(
  receiver: UnboundedReceiver<Result<Message, Error>>,
  ws_sender: SplitSink<WebSocket, Message>,
) -> JoinHandle<Result<(), Error>> {
  tokio::spawn(UnboundedReceiverStream::new(receiver).forward(ws_sender))
}

fn spawn_heartbeat(
  peer_id: PeerId,
  sender: UnboundedSender<Result<Message, Error>>,
  signaling: Signaling,
) -> JoinHandle<Result<()>> {
  tokio::spawn(async move {
    let mut stream = IntervalStream::new(tokio::time::interval(Duration::from_millis(30_000)));
    while stream.next().await.is_some() {
      if signaling.is_alive(peer_id) {
        signaling.heartbeat(peer_id, false)?;
        sender.send(Ok(Message::Ping("".into())))?;
      } else {
        sender.send(Ok(Message::Close(None)))?;
      }
    }
    Ok(())
  })
}

#[instrument(name = "message", skip_all, fields(peer = peer_id.to_string()))]
fn handle_message(message: Message, peer_id: PeerId, signaling: &Signaling) -> Result<()> {
  match message {
    Message::Text(payload) => handle_event(payload, peer_id, signaling),
    Message::Binary(_) => bail!("unsupported binary message"),
    Message::Pong(_) => signaling.heartbeat(peer_id, true),
    _ => unreachable!(),
  }
}

fn handle_event(payload: String, peer_id: PeerId, signaling: &Signaling) -> Result<()> {
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
