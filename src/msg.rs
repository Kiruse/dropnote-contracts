use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
  pub denom: String,
  pub store_keys_fee: Uint128,
  pub store_notes_fee: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
  UpdateFees {
    admin: Option<String>,
    store_keys: Uint128,
    store_notes: Uint128,
    denom: String,
    burn_fees: bool,
  },
  UpdateKey { key: String },
  StoreNote {
    recipient: String,
    note: String,
  },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
  #[returns(EncryptionKeyResponse)]
  EncryptionKey { address: String },
  #[returns(u64)]
  NoteCount {
    recipient: String,
    sender: String,
  },
  #[returns(Vec<String>)]
  Senders {
    recipient: String,
  },
  #[returns(Vec<crate::state::Note>)]
  Notes {
    recipient: String,
    sender: String,
    start_after: Option<u64>,
    limit: Option<u32>,
  },
  #[returns(crate::state::Fees)]
  Fees {},
}

#[cw_serde]
pub struct EncryptionKeyResponse {
  pub key: Option<Vec<u8>>,
}
