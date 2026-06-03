//! HTTP layer smoke test.
//!
//! Boots only marketd's axum router against a hand-built `SharedStore` (no
//! bitcoind, no sync) and asserts the two API endpoints return the expected
//! JSON shape. Catches regressions in routing, JSON serialization, and the
//! state types — the layer most likely to silently break a frontend.

mod common;

use marketd::state::{ApiFidelityBond, ApiMaker, ApiMakerState, ApiOffer, ApiOutpoint, new_store};
use serde_json::Value;

#[test]
fn health_and_makers_empty_store() {
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
    assert_eq!(body["maker_count"], 0);
    assert_eq!(body["with_offer"], 0);
    assert!(body["last_sync"].is_null());

    let resp = guard
        .agent
        .get(&guard.url("/api/makers"))
        .call()
        .expect("GET /api/makers");
    assert_eq!(resp.status(), 200);
    let makers: Value = resp.into_json().expect("makers JSON");
    assert_eq!(makers.as_array().expect("array").len(), 0);
}

#[test]
fn makers_returns_seeded_data() {
    let store = new_store();
    {
        let mut s = store.write().unwrap();
        // A "Good" maker with a populated offer.
        s.makers.push(ApiMaker {
            address: "127.0.0.1:6102".into(),
            state: ApiMakerState::Good,
            protocol: Some("taproot"),
            timestamp: 1_234_567_890,
            last_offer_update_ts: Some(1_234_567_890),
            next_offer_check_ts: Some(1_234_567_950),
            offer: Some(ApiOffer {
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
            }),
        });
        // An unresponsive maker — no offer payload, but still appears.
        s.makers.push(ApiMaker {
            address: "127.0.0.1:7777".into(),
            state: ApiMakerState::Unresponsive { retries: 3 },
            protocol: None,
            timestamp: 1_234_567_890,
            last_offer_update_ts: None,
            next_offer_check_ts: Some(1_234_568_000),
            offer: None,
        });
        // A bad maker with no offer at all.
        s.makers.push(ApiMaker {
            address: "127.0.0.1:8888".into(),
            state: ApiMakerState::Bad,
            protocol: None,
            timestamp: 1_234_567_890,
            last_offer_update_ts: None,
            next_offer_check_ts: None,
            offer: None,
        });
        s.last_sync = Some(1_700_000_000);
    }

    let guard = common::MarketdServerGuard::start(store);

    let resp = guard
        .agent
        .get(&guard.url("/api/makers"))
        .call()
        .expect("GET /api/makers");
    assert_eq!(resp.status(), 200);
    let makers: Value = resp.into_json().expect("makers JSON");
    let arr = makers.as_array().expect("array");
    assert_eq!(arr.len(), 3, "bad and unresponsive makers must be returned");

    // Good maker
    let good = &arr[0];
    assert_eq!(good["address"], "127.0.0.1:6102");
    assert_eq!(good["state"]["kind"], "good");
    assert_eq!(good["protocol"], "taproot");
    assert_eq!(good["offer"]["base_fee"], 1000);
    assert_eq!(good["offer"]["fidelity_bond"]["amount"], 5_000_000);
    assert_eq!(
        good["offer"]["fidelity_bond"]["outpoint"]["txid"],
        "deadbeef"
    );

    // Unresponsive maker — kind=unresponsive with a retries count.
    let unresp = &arr[1];
    assert_eq!(unresp["state"]["kind"], "unresponsive");
    assert_eq!(unresp["state"]["retries"], 3);
    assert!(
        unresp["offer"].is_null(),
        "offer must be null for unresponsive maker"
    );
    assert!(unresp["protocol"].is_null());

    // Bad maker — same shape, kind=bad.
    let bad = &arr[2];
    assert_eq!(bad["state"]["kind"], "bad");
    assert!(bad["offer"].is_null());

    let resp = guard
        .agent
        .get(&guard.url("/api/health"))
        .call()
        .expect("GET /api/health");
    let body: Value = resp.into_json().expect("health JSON");
    assert_eq!(body["maker_count"], 3);
    assert_eq!(body["with_offer"], 1);
    assert_eq!(body["last_sync"], 1_700_000_000);
}
