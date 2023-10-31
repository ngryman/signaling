use anyhow::Result;
use clap::Parser;
use signaling::{Server, Signaling};
use tracing::Level;
use tracing_subscriber::fmt::format;
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
      .event_format(format().compact().with_target(false).with_source_location(false))
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
      .event_format(format().pretty().with_target(false).with_source_location(false).without_time())
      .with_env_filter(
        EnvFilter::builder()
          .with_default_directive(Level::DEBUG.into())
          .from_env_lossy()
          .add_directive("hyper=off".parse().unwrap())
          .add_directive("tungstenite=off".parse().unwrap()),
      )
      .init();
  }

  let args = Args::parse();
  let signaling = Signaling::default();
  let server = Server::new(args.port, signaling.clone());

  tokio::select! {
    _ = signaling.run() => {},
    _ = server.run() => {},
  }

  Ok(())
}
