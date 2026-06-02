use axum::{Router, extract::State, http::StatusCode, response::Json, routing::get};
use serde_json::{Value, json};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

use crate::state::SharedStore;

pub fn router(store: SharedStore, static_dir: String) -> Router {
    let index = format!("{static_dir}/index.html");
    let spa = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index));

    Router::new()
        .route("/api/offers", get(get_offers))
        .route("/api/health", get(get_health))
        .with_state(store)
        .layer(CorsLayer::permissive())
        .fallback_service(spa)
}

async fn get_offers(State(store): State<SharedStore>) -> Result<Json<Value>, StatusCode> {
    let offers = store
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .offers
        .clone();
    Ok(Json(json!(offers)))
}

async fn get_health(State(store): State<SharedStore>) -> Json<Value> {
    let s = store.read().unwrap();
    Json(json!({
        "status": "ok",
        "offer_count": s.offers.len(),
        "last_sync": s.last_sync,
    }))
}
