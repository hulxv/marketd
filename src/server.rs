use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

use crate::state::{ApiMaker, ApiMakerState, SharedStore};

pub fn router(store: SharedStore, static_dir: String) -> Router {
    let index = format!("{static_dir}/index.html");
    let spa = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index));

    Router::new()
        .route("/api/makers", get(get_makers))
        .route("/api/health", get(get_health))
        .with_state(store)
        .layer(CorsLayer::permissive())
        .fallback_service(spa)
}

#[derive(Serialize, Deserialize)]
struct MakerQueryParams {
    state: Option<ApiMakerState>,
}

async fn get_makers(
    State(store): State<SharedStore>,
    Query(params): Query<MakerQueryParams>,
) -> Result<Json<Value>, StatusCode> {
    let makers = store
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .makers
        .clone();
    let makers = makers
        .iter()
        .filter(|m| params.state.as_ref().is_none_or(|state| state == &m.state))
        .collect::<Vec<&ApiMaker>>();

    Ok(Json(json!(makers)))
}

async fn get_health(State(store): State<SharedStore>) -> Json<Value> {
    let s = store.read().unwrap();
    let with_offer = s.makers.iter().filter(|m| m.offer.is_some()).count();
    Json(json!({
        "status": "ok",
        "maker_count": s.makers.len(),
        "with_offer": with_offer,
        "last_sync": s.last_sync,
    }))
}
