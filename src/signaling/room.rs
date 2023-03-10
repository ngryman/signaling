use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::peer::PeerId;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct RoomId(Ulid);

impl fmt::Display for RoomId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0.to_string().to_lowercase())
  }
}

#[derive(Debug, Serialize)]
pub struct Room {
  pub id: RoomId,
  pub peers: HashSet<PeerId>,
}

impl Room {
  pub(super) fn new(id: RoomId) -> Self {
    Self { id, peers: Default::default() }
  }
}
