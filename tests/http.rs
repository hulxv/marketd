//! HTTP layer smoke test.
//!
//! Boots only marketd's axum router against a hand-built `SharedStore` (no
//! bitcoind, no sync) and asserts the two API endpoints return the expected
//! JSON shape. Catches regressions in routing, JSON serialization, and the
//! state types — the layer most likely to silently break a frontend.

mod common;

use marketd::state::{ApiFidelityBond, ApiOffer, ApiOutpoint, new_store};
use serde_json::Value;

#[test]
fn health_and_offers_empty_store() {
    let store = new_store();
    let guard = common::MarketdServerGuard::start(store);

    let resp = guard
        .agent
        .get(&guard.url("/api/health"))
        .call()
        .expect("GET /api/health");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.into_json().expect("health JSON");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["offer_count"], 0);
    assert!(body["last_sync"].is_null());

    let resp = guard
        .agent
        .get(&guard.url("/api/offers"))
        .call()
        .expect("GET /api/offers");
    assert_eq!(resp.status(), 200);
    let offers: Value = resp.into_json().expect("offers JSON");
    assert_eq!(offers.as_array().expect("array").len(), 0);
}

#[test]
fn offers_returns_seeded_data() {
    let store = new_store();
    {
        let mut s = store.write().unwrap();
        s.offers.push(ApiOffer {
            address: "test.onion:6102".into(),
            timestamp: 1_234_567_890,
            base_fee: 1000,
            amount_relative_fee_pct: 0.5,
            time_relative_fee_pct: 0.1,
            min_size: 10_000,
            max_size: 100_000_000,
            required_confirms: 1,
            minimum_locktime: 144,
            tweakable_point: "02abcd".into(),
            fidelity_bond: ApiFidelityBond {
                amount: 5_000_000,
                outpoint: ApiOutpoint {
                    txid: "deadbeef".into(),
                    vout: 0,
                },
                lock_time: 800_000,
                cert_hash: "feedface".into(),
                cert_sig: "303d".into(),
            },
        });
        s.last_sync = Some(1_700_000_000);
    }

    let guard = common::MarketdServerGuard::start(store);

    let resp = guard
        .agent
        .get(&guard.url("/api/offers"))
        .call()
        .expect("GET /api/offers");
    assert_eq!(resp.status(), 200);
    let offers: Value = resp.into_json().expect("offers JSON");
    let arr = offers.as_array().expect("array");
    assert_eq!(arr.len(), 1);

    let o = &arr[0];
    assert_eq!(o["address"], "test.onion:6102");
    assert_eq!(o["base_fee"], 1000);
    assert_eq!(o["fidelity_bond"]["amount"], 5_000_000);
    assert_eq!(o["fidelity_bond"]["outpoint"]["txid"], "deadbeef");
    assert_eq!(o["fidelity_bond"]["outpoint"]["vout"], 0);

    let resp = guard
        .agent
        .get(&guard.url("/api/health"))
        .call()
        .expect("GET /api/health");
    let body: Value = resp.into_json().expect("health JSON");
    assert_eq!(body["offer_count"], 1);
    assert_eq!(body["last_sync"], 1_700_000_000);
}
