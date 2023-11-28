use axum::{
  Router,
  http::StatusCode,
  handler::HandlerWithoutStateExt,
};
use tower_http::{
  trace::TraceLayer, 
  services::ServeDir,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use clap::Parser;
use std::net::{SocketAddr, IpAddr, Ipv6Addr};
use std::str::FromStr;





#[derive(Parser, Debug)]
#[command(name="axum-server")]
pub struct Cli {
  #[arg(long="addr", short='a', default_value="::1")]
  pub addr: String,

  #[arg(long="port", short='s', default_value="3000")]
  pub port: u16,

  #[arg(long="static-dir", short='d', default_value="./assets")]
  pub static_dir: String,
}


#[tokio::main]
async fn main() {

  let cli = Cli::parse();
  
  // logging
  tracing_subscriber::registry()
  .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "crate_server=debug".into()))
  .with(tracing_subscriber::fmt::layer())
  .init();

  // serve dir
  async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
  }
  // you can convert handler function to service
  let service = handle_404.into_service();

  let serve_dir = ServeDir::new(&cli.static_dir).not_found_service(service);

  let app = Router::new()
    
    .nest_service("/static", serve_dir.clone())
    .layer(TraceLayer::new_for_http())
    ;

  // serve
  let addr = SocketAddr::from((
    IpAddr::from_str(cli.addr.as_str()).unwrap_or(IpAddr::V6(Ipv6Addr::LOCALHOST)), cli.port,
  ));

  axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}


async fn shutdown_signal() {
  tokio::signal::ctrl_c()
    .await
    .expect("expect tokio signal ctrl-c");
  println!("signal shutdown");
}