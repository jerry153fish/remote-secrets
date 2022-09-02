use actix_web::{get, web::Data, HttpRequest, HttpResponse, Responder};

use crate::manager::Manager;

use prometheus::{Encoder, TextEncoder};
use utils::metrics::REGISTRY;

#[get("/healthz")]
pub async fn health(_: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json("healthy")
}

#[get("/readyz")]
pub async fn ready(_: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json("ready")
}

#[get("/metrics")]
pub async fn metrics(_c: Data<Manager>, _req: HttpRequest) -> impl Responder {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder.encode(&REGISTRY.gather(), &mut buffer).unwrap();
    HttpResponse::Ok().body(buffer)
}
