use axum::{
  async_trait,
  Router,
  http::StatusCode,
  handler::HandlerWithoutStateExt,
  routing::{get, post},
  extract::{self, State},
  response::IntoResponse
};
use tower_http::{
  trace::TraceLayer, 
  services::ServeDir,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use bb8::{Pool, PooledConnection};
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

use clap::Parser;
use std::net::{SocketAddr, IpAddr, Ipv6Addr};
use std::str::FromStr;
use std::time::Duration;


/***
 * static
 * + db(postgres)
 */

#[derive(Parser, Debug)]
#[command(name="axum-server")]
pub struct Cli {
  #[arg(long="addr", short='a', default_value="::1")]
  pub addr: String,

  #[arg(long="port", short='p', default_value="3000")]
  pub port: u16,

  #[arg(long="static-dir", short='d', default_value="./assets")]
  pub static_dir: String,

  #[arg(long="db", default_value="postgres://pg:ss@localhost/db")]
  db: String
}


#[tokio::main]
async fn main() {

  let cli = Cli::parse();
  
  // logging
  tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "crate_server=debug".into()))
    .with(tracing_subscriber::fmt::layer())
    .init();

  // set up connection pool
  let manager = PostgresConnectionManager::new_from_stringlike(cli.db.as_str(), NoTls).unwrap();
  let pool = Pool::builder()
    .max_size(5)
    .connection_timeout(Duration::from_secs(3))
    .build(manager).await.expect("can't connect to database");


  // serve dir
  async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
  }
  // you can convert handler function to service
  let service = handle_404.into_service();

  let serve_dir = ServeDir::new(&cli.static_dir).not_found_service(service);

  let app = Router::new()
    .route("/db/read", get(read_db))
    .route("/db/insert/:number", post(insert_db))
    .nest_service("/static", serve_dir.clone())
    .layer(TraceLayer::new_for_http())
    .with_state(pool);

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



// DB

type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;


async fn insert_db(
  State(pool): State<ConnectionPool>,
  extract::Path(number): extract::Path<String>
) -> impl IntoResponse {

  let number: i32 = number.parse().unwrap();

  let conn = pool.get().await.unwrap();

  let exe = conn
    .execute("INSERT INTO aa (a) VALUES ($1::INT)", &[&number])
    .await
    .map_err(|e| println!("{:?}", e));

  let res = if let Ok(exe) = exe {
    format!("{:?}", exe)
  } else {
    "Err".to_string()
  };
  res
}

async fn read_db(
  State(pool): State<ConnectionPool>,
) -> impl IntoResponse {
  let conn = pool.get().await;

  let conn = pool.get().await.unwrap();

  let rows = conn
    .query("SELECT * FROM aa", &[])
    .await
    .map_err(|e| println!("{:?}", e));

  let res: Vec<i32> = if let Ok(rows) = rows {

    rows.iter().filter_map(|x| x.get(0)).collect()
  } else {
    vec![]
  };
  format!("{:?}", res)
}