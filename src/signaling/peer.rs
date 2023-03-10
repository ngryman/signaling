use std::collections::HashSet;
use std::fmt;

use axum::extract::ws::Message;
use axum::Error;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use ulid::Ulid;

use super::room::RoomId;

pub type PeerSender = UnboundedSender<Result<Message, Error>>;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct PeerId(Ulid);

impl PeerId {
  pub(super) fn new() -> Self {
    Self(Ulid::new())
  }
}

impl fmt::Display for PeerId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0.to_string().to_lowercase())
  }
}

#[derive(Debug, Serialize)]
pub struct Peer {
  pub id: PeerId,
  pub rooms: HashSet<RoomId>,
  #[serde(skip)]
  pub sender: PeerSender,
}

impl Peer {
  pub(super) fn new(id: PeerId, sender: PeerSender) -> Self {
    Self { id, rooms: Default::default(), sender }
  }
}
