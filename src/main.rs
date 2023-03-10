use anyhow::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use clap::Parser;
use signaling::{Config, Server, Signaling};
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Server port
  #[arg(short, long, env, default_value_t = 3000, value_parser = clap::value_parser!(u16).range(1025..))]
  port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
  if cfg!(not(debug_assertions)) {
    tracing_subscriber::fmt()
      .with_env_filter(
        EnvFilter::builder()
          .with_default_directive(Level::INFO.into())
          .from_env_lossy()
          .add_directive("hyper=off".parse().unwrap())
          .add_directive("tungstenite=off".parse().unwrap()),
      )
      .init();
  } else {
    tracing_subscriber::fmt()
      .with_env_filter(
        EnvFilter::builder()
          .with_default_directive(Level::DEBUG.into())
          .from_env_lossy()
          .add_directive("hyper=off".parse().unwrap())
          .add_directive("tungstenite=off".parse().unwrap()),
      )
      .without_time()
      .init();
  }

  let args = Args::parse();
  let signaling = Signaling::new(Config::default());
  let server = Server::new(args.port, signaling);
  server.listen().await
}

pub async fn generate_handler() -> impl IntoResponse {
  StatusCode::OK
}
