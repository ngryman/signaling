mod config;
mod peer;
mod room;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::ws::Message;
use parking_lot::RwLock;
use tracing::error;

pub use self::config::Config;
pub(crate) use self::peer::{Peer, PeerId};
pub(crate) use self::room::{Room, RoomId};

use self::peer::PeerSender;

#[derive(Clone, Debug)]
pub struct Signaling {
  peers: Arc<RwLock<HashMap<PeerId, Arc<RwLock<Peer>>>>>,
  rooms: Arc<RwLock<HashMap<RoomId, Arc<RwLock<Room>>>>>,
}

impl Signaling {
  pub fn new(_config: Config) -> Self {
    Self { peers: Default::default(), rooms: Default::default() }
  }

  pub fn peers(&self) -> Vec<Arc<RwLock<Peer>>> {
    self.peers.read_arc().values().cloned().collect()
  }

  pub fn rooms(&self) -> Vec<Arc<RwLock<Room>>> {
    self.rooms.read_arc().values().cloned().collect()
  }

  pub fn add_peer(&self, sender: PeerSender) -> PeerId {
    let id = PeerId::new();
    let peer = Arc::new(RwLock::new(Peer::new(id, sender)));
    self.peers.write_arc().insert(id, peer);
    id
  }

  pub fn remove_peer(&self, peer_id: PeerId) -> Result<()> {
    // Remove peer from all rooms it joined
    self
      .peers
      .read_arc()
      .get(&peer_id)
      .with_context(|| format!("peer {peer_id} does not exist"))?
      .read_arc()
      .rooms
      .iter()
      .for_each(|room_id| {
        if let Some(room) = self.rooms.read_arc().get(room_id) {
          room.write_arc().peers.remove(&peer_id);
        }
      });

    // Remove the peer itself
    self.peers.write_arc().remove(&peer_id);

    Ok(())
  }

  pub fn set_alive(&self, peer_id: PeerId, is_alive: bool) -> Result<()> {
    // Set the last beat and payload of the peer to now
    self
      .peers
      .read_arc()
      .get(&peer_id)
      .with_context(|| format!("peer {peer_id} does not exist"))?
      .write_arc()
      .is_alive = is_alive;

    Ok(())
  }

  pub fn is_alive(&self, peer_id: PeerId) -> bool {
    self.peers.read_arc().get(&peer_id).map(|peer| peer.read_arc().is_alive).unwrap_or(false)
  }

  pub fn join_room(&self, peer_id: PeerId, room_id: RoomId) -> Result<()> {
    // Add the room to the list of joined rooms for this peer
    self
      .peers
      .read_arc()
      .get(&peer_id)
      .with_context(|| format!("peer {peer_id} does not exist"))?
      .write_arc()
      .rooms
      .insert(room_id.clone());

    // Add the peer to the room, creating it on the fly if it doesn't already exist
    if !self.rooms.read_arc().contains_key(&room_id) {
      let mut room = Room::new(room_id.clone());
      room.peers.insert(peer_id);
      self.rooms.write_arc().insert(room_id, Arc::new(RwLock::new(room)));
    } else {
      self
        .rooms
        .read_arc()
        .get(&room_id)
        .with_context(|| format!("room {room_id} does not exist"))?
        .write_arc()
        .peers
        .insert(peer_id);
    }

    Ok(())
  }

  pub fn leave_room(&self, peer_id: PeerId, room_id: RoomId) -> Result<()> {
    // Remove the room for the list of joined rooms for this peer
    self
      .peers
      .read_arc()
      .get(&peer_id)
      .with_context(|| format!("peer {peer_id} does not exist"))?
      .write_arc()
      .rooms
      .remove(&room_id);

    // Remove the peer from the room
    self
      .rooms
      .read_arc()
      .get(&room_id)
      .with_context(|| format!("room {room_id} does not exist"))?
      .write_arc()
      .peers
      .remove(&peer_id);

    Ok(())
  }

  pub fn broadcast(&self, peer_id: PeerId, room_id: RoomId, payload: String) -> Result<()> {
    self
      .rooms
      .read_arc()
      .get(&room_id)
      .with_context(|| format!("room {room_id} does not exist"))?
      .read_arc()
      .peers
      .iter()
      .filter(|other_id| **other_id != peer_id)
      .for_each(|peer_id| {
        if let Err(e) = self.send(*peer_id, payload.clone()) {
          error!("{e}")
        }
      });

    Ok(())
  }

  pub fn send(&self, peer_id: PeerId, payload: String) -> Result<()> {
    self
      .peers
      .read_arc()
      .get(&peer_id)
      .with_context(|| format!("peer {peer_id} does not exist"))?
      .read_arc()
      .sender
      .send(Ok(Message::Text(payload)))
      .map_err(Into::into)
  }
}
