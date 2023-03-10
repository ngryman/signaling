use crate::signaling::Signaling;

#[derive(Clone)]
pub(crate) struct ServerState {
  pub signaling: Signaling,
}

impl ServerState {
  pub fn new(signaling: Signaling) -> Self {
    Self { signaling }
  }
}
