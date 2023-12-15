use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::Metadata;
use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub retry_delay: u64,
    pub job_id: String,
    pub owner: Addr,
    pub metadata: Metadata,
}

pub const ADD_COLLATERAL_TIMESTAMP: Map<String, Timestamp> = Map::new("add_collateral_timestamp");
pub const REPAY_TIMESTAMP: Map<String, Timestamp> = Map::new("repay_timestamp");
pub const STATE: Item<State> = Item::new("state");
