#![deny(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![allow(clippy::unused_async)]

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use influxdb::{Client, InfluxDbWriteable, ReadQuery};
use serde::{Deserialize, Serialize};
use std::env;
use std::{net::SocketAddr, sync::Arc};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv().expect(".env file not found");

    let token = env::var("TOKEN").expect("Coult not find TOKEN in .env file.");
    let db_url = env::var("DB_URL").expect("Coult not find DB_URL in .env file.");
    let db_name = env::var("DB_NAME").expect("Coult not find DB_NAME in .env file.");

    let client = Client::new(db_url, db_name).with_token(token);
    let q = Reading::default().into_query("sensor");
    client.query(&q).await.unwrap();
    tracing::debug!("Inserted {:?}", q);

    let rq = ReadQuery::new("SELECT * FROM SENSOR");
    client.query(rq).await.unwrap();

    let state = Arc::new(client);

    let app = Router::new()
        .route("/", get(root))
        .route("/reading", post(create_reading))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "This is a backend data-collector. Head to the InfluxDB login to see the data."
}

async fn create_reading(
    State(state): State<Arc<Client>>,
    Json(payload): Json<ReadingWithoutTime>,
) -> StatusCode {
    let r: Reading = payload.into();
    let t = r.time;
    let q = r.into_query("sensor");

    match state.query(q).await {
        Ok(_) => {
            tracing::debug!("Inserted data from {}", t);
            StatusCode::CREATED
        }
        Err(e) => {
            tracing::error!("{}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

// the output to our `create_reading` handler
#[derive(Deserialize, Serialize, Default, InfluxDbWriteable)]
struct Reading {
    time: DateTime<Utc>,
    pm1p0: f32,
    pm2p5: f32,
    pm4p0: f32,
    pm10p0: f32,
    temp: f32,
    humid: f32,
    voc: f32,
    nox: f32,
}

#[derive(Deserialize, Serialize, Default)]
struct ReadingWithoutTime {
    pm1p0: f32,
    pm2p5: f32,
    pm4p0: f32,
    pm10p0: f32,
    temp: f32,
    humid: f32,
    voc: f32,
    nox: f32,
}

impl From<ReadingWithoutTime> for Reading {
    fn from(value: ReadingWithoutTime) -> Self {
        Self {
            time: Utc::now(),
            pm1p0: value.pm1p0,
            pm2p5: value.pm2p5,
            pm4p0: value.pm4p0,
            pm10p0: value.pm10p0,
            temp: value.temp,
            humid: value.humid,
            voc: value.voc,
            nox: value.nox,
        }
    }
}
