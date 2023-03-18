#![deny(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![allow(clippy::unused_async)]

// use axum_macros::debug_handler;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router, extract::State,
};
use chrono::{DateTime, Utc};
use influxdb::{Client, InfluxDbWriteable, ReadQuery};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let client = Client::new("http://localhost:8086", "sensor").with_token(
        "j7t6tlAMH6i7UwnjF6tIPiX1xG1N-4l4zGeBkW6qOUFmv28EiB1rruaVxLI9L5E-CI0GPY66-XvvbUNpmwtU_g==",
    );
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
    Json(payload): Json<ReadingWithoutTime>     
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
