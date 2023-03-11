use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::signaling::RoomId;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(super) enum Event {
  Subscribe {
    #[serde(rename = "topics")]
    room_ids: Vec<RoomId>,
  },
  Unsubscribe {
    #[serde(rename = "topics")]
    room_ids: Vec<RoomId>,
  },
  Publish {
    #[serde(rename = "topic")]
    room_id: RoomId,
  },
  Ping,
  Pong,
}

impl fmt::Display for Event {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
  }
}

impl FromStr for Event {
  type Err = serde_json::Error;

  fn from_str(s: &str) -> serde_json::Result<Self> {
    serde_json::from_str(s)
  }
}
