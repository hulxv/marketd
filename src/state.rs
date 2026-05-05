use coinswap::{protocol::common_messages::Offer, taker::offers::MakerAddress};
use serde::Serialize;
use serde_json::Value;
use std::sync::{Arc, RwLock};

#[derive(Serialize, Clone, Debug)]
pub struct ApiOutpoint {
    pub txid: String,
    pub vout: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct ApiFidelityBond {
    pub amount: u64,
    pub outpoint: ApiOutpoint,
    pub lock_time: u32,
    pub cert_hash: String,
    pub cert_sig: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct ApiOffer {
    pub address: String,
    pub timestamp: u64,
    pub base_fee: u64,
    pub amount_relative_fee_pct: f64,
    pub time_relative_fee_pct: f64,
    pub min_size: u64,
    pub max_size: u64,
    pub required_confirms: u32,
    pub minimum_locktime: u16,
    pub tweakable_point: String,
    pub fidelity_bond: ApiFidelityBond,
}

impl ApiOffer {
    pub fn from_coinswap(offer: &Offer, address: &MakerAddress, timestamp: u64) -> Self {
        let bond = &offer.fidelity.bond;

        // TODO: FidelityBond::outpoint is pub(crate); extract via serde until it is made pub.
        let bond_json = serde_json::to_value(bond).unwrap_or(Value::Null);
        let txid = bond_json["outpoint"]["txid"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let vout = bond_json["outpoint"]["vout"].as_u64().unwrap_or(0) as u32;

        Self {
            address: address.to_string(),
            timestamp,
            base_fee: offer.base_fee,
            amount_relative_fee_pct: offer.amount_relative_fee_pct,
            time_relative_fee_pct: offer.time_relative_fee_pct,
            min_size: offer.min_size,
            max_size: offer.max_size,
            required_confirms: offer.required_confirms,
            minimum_locktime: offer.minimum_locktime,
            tweakable_point: offer.tweakable_point.to_string(),
            fidelity_bond: ApiFidelityBond {
                amount: bond.amount.to_sat(),
                outpoint: ApiOutpoint { txid, vout },
                lock_time: bond.lock_time.to_consensus_u32(),
                cert_hash: offer.fidelity.cert_hash.to_string(),
                cert_sig: hex::encode(offer.fidelity.cert_sig.serialize_der().as_ref()),
            },
        }
    }
}

#[derive(Default, Clone)]
pub struct OfferStore {
    pub offers: Vec<ApiOffer>,
    pub last_sync: Option<u64>,
}

pub type SharedStore = Arc<RwLock<OfferStore>>;

pub fn new_store() -> SharedStore {
    Arc::new(RwLock::new(OfferStore::default()))
}
