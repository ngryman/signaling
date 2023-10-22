use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{bail, Result};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::Error;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::{IntervalStream, UnboundedReceiverStream};
use tracing::{debug, error, info, instrument};

use crate::server::event::Event;
use crate::server::state::ServerState;
use crate::signaling::PeerId;
use crate::Signaling;

pub(crate) async fn signal(
  ws: WebSocketUpgrade,
  State(state): State<ServerState>,
  ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| handle_socket(socket, state, addr))
}

#[instrument(name = "socket", skip_all, fields(addr = addr.to_string()))]
async fn handle_socket(socket: WebSocket, state: ServerState, addr: SocketAddr) {
  let (ws_sender, ws_receiver) = socket.split();
  let (sender, receiver) = mpsc::unbounded_channel();
  let peer_id = state.signaling.add_peer(sender.clone());
  info!("{peer_id} connected");

  tokio::select! {
    _ = handle_channel(receiver, ws_sender) => {},
    _ = handle_heartbeats(peer_id, sender, state.signaling.clone()) => {},
    _ = handle_messages(peer_id, ws_receiver, state.signaling.clone()) => {},
  }

  if let Err(e) = state.signaling.remove_peer(peer_id) {
    error!("{e}");
  }
}

async fn handle_channel(
  receiver: UnboundedReceiver<Result<Message, Error>>,
  ws_sender: SplitSink<WebSocket, Message>,
) -> Result<()> {
  UnboundedReceiverStream::new(receiver).forward(ws_sender).await.map_err(Into::into)
}

#[instrument(name = "heartbeat", skip_all, fields(peer = peer_id.to_string()))]
async fn handle_heartbeats(
  peer_id: PeerId,
  sender: UnboundedSender<Result<Message, Error>>,
  signaling: Signaling,
) -> Result<()> {
  let mut stream = IntervalStream::new(tokio::time::interval(Duration::from_millis(10_000)));
  while stream.next().await.is_some() {
    if signaling.is_alive(peer_id) {
      debug!("send ping");
      signaling.set_peer_alive(peer_id, false)?;
      sender.send(Ok(Message::Ping("".into())))?;
    } else {
      info!("connection timeout");
      break;
    }
  }
  Ok(())
}

#[instrument(name = "message", skip_all, fields(peer = peer_id.to_string()))]
async fn handle_messages(
  peer_id: PeerId,
  mut ws_receiver: SplitStream<WebSocket>,
  signaling: Signaling,
) {
  while let Some(Ok(message)) = ws_receiver.next().await {
    if let Message::Close(_) = message {
      info!("disconnected");
      break;
    }

    if let Err(e) = handle_message(message, peer_id, &signaling) {
      error!("{e}")
    }
  }
}

fn handle_message(message: Message, peer_id: PeerId, signaling: &Signaling) -> Result<()> {
  match message {
    Message::Text(payload) => handle_event(payload, peer_id, signaling),
    Message::Binary(_) => bail!("unsupported binary message"),
    Message::Pong(_) => {
      debug!("recv pong");
      signaling.set_peer_alive(peer_id, true)
    }
    _ => unreachable!(),
  }
}

fn handle_event(payload: String, peer_id: PeerId, signaling: &Signaling) -> Result<()> {
  let event: Event = payload.parse()?;
  info!("recv event event={event}");

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
