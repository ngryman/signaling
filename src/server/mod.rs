mod event;
mod handlers;
mod state;

use std::net::SocketAddr;

use anyhow::Result;
use axum::http::Method;
use axum::routing::get;
use axum::Router;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;
use tracing::{info, Level};

use crate::signaling::Signaling;

use self::state::ServerState;

pub struct Server {
  port: u16,
  signaling: Signaling,
}

impl Server {
  pub fn new(port: u16, signaling: Signaling) -> Self {
    Self { port, signaling }
  }

  pub async fn listen(self) -> Result<()> {
    let state = ServerState::new(self.signaling);
    let app = Router::new()
      .route("/", get(handlers::signal))
      .route("/info", get(handlers::info))
      .layer(cors())
      .layer(trace())
      .with_state(state);

    info!("starting server: {}", self.port);
    let addr = SocketAddr::new([0, 0, 0, 0].into(), self.port);
    axum::Server::bind(&addr)
      .serve(app.into_make_service_with_connect_info::<SocketAddr>())
      .await?;

    Ok(())
  }
}

fn cors() -> CorsLayer {
  if cfg!(not(debug_assertions)) {
    CorsLayer::new()
      .allow_methods([Method::GET])
      .allow_origin(["https://wildbits.app".parse().unwrap()])
  } else {
    CorsLayer::new().allow_methods([Method::GET]).allow_origin(Any)
  }
}

fn trace() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
  TraceLayer::new_for_http()
    .on_response(DefaultOnResponse::new().level(Level::INFO).latency_unit(LatencyUnit::Micros))
}
