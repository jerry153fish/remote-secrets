use actix_web::{
    get, middleware, web::Data, App, HttpRequest, HttpResponse, HttpServer, Responder,
};

use crate::manager::Manager;

use prometheus::{Encoder, TextEncoder};

#[get("/healthz")]
pub async fn health(_: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json("healthy")
}

#[get("/readyz")]
pub async fn ready(_: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json("ready")
}

#[get("/metrics")]
pub async fn metrics(c: Data<Manager>, _req: HttpRequest) -> impl Responder {
    let metrics = c.metrics();
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder.encode(&metrics, &mut buffer).unwrap();
    HttpResponse::Ok().body(buffer)
}
