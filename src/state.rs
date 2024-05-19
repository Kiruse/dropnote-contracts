use cosmwasm_std::{Addr, Order, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const FEES: Item<Fees> = Item::new("fees");
const ENCRYPTION_KEYS: Map<Addr, Vec<u8>> = Map::new("state");

const NOTE_META: Map<(Addr, Addr), NoteMeta> = Map::new("note_counts");
const NOTES: Map<(Addr, Addr, u64), Note> = Map::new("notes");

#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Fees {
  // the address allowed to adjust fees. will automatically receive fees.
  // when unset, fees are burnt instead.
  pub admin: Option<Addr>,
  pub store_keys: Uint128,
  pub store_notes: Uint128,
  pub denom: String,
  // whether to burn fees rather than send to admin.
  pub burn_fees: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteMeta {
  pub count: u64,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Note {
  pub sender: Addr,
  pub note: Vec<u8>,
  pub timestamp: Timestamp,
}

pub fn load_fees(store: &dyn Storage) -> crate::ContractResult<Fees> {
  Ok(FEES.load(store)?)
}

pub fn save_fees(store: &mut dyn Storage, fees: &Fees) -> crate::ContractResult<()> {
  Ok(FEES.save(store, fees)?)
}

pub fn load_enc_key(store: &dyn Storage, addr: Addr) -> crate::ContractResult<Option<Vec<u8>>> {
  Ok(ENCRYPTION_KEYS.may_load(store, addr)?)
}

pub fn save_enc_key(store: &mut dyn Storage, addr: Addr, key: &Vec<u8>) -> crate::ContractResult<()> {
  Ok(ENCRYPTION_KEYS.save(store, addr, key)?)
}

pub fn load_notes(store: &dyn Storage, recipient: Addr, sender: Addr, start_after: Option<u64>, limit: Option<u32>) -> crate::ContractResult<Vec<Note>> {
  let meta = NOTE_META.load(store, (recipient.clone(), sender.clone()))?;
  let start = start_after.unwrap_or(0);
  let end = limit.map(|l| (start + l as u64).min(meta.count)).unwrap_or(meta.count);
  let res: Vec<Note> = (start..end)
    .map(|idx| NOTES.load(store, (recipient.clone(), sender.clone(), idx)))
    .collect::<Result<_, _>>()?;
  Ok(res)
}

pub fn load_note_meta(store: &dyn Storage, sender: Addr, recipient: Addr) -> crate::ContractResult<NoteMeta> {
  Ok(NOTE_META.load(store, (recipient, sender))?)
}

pub fn find_senders(store: &dyn Storage, recipient: Addr) -> crate::ContractResult<Vec<Addr>> {
  let prefix = NOTE_META.prefix(recipient.clone());
  let senders: Vec<Addr> = prefix.keys(store, None, None, Order::Ascending)
    .map(|item| item.unwrap())
    .collect();
  Ok(senders)
}

pub fn store_note(store: &mut dyn Storage, sender: Addr, recipient: Addr, note: Note) -> crate::ContractResult<()> {
  let meta = NOTE_META.may_load(store, (recipient.clone(), sender.clone()))?;
  let mut meta = match meta {
    Some(meta) => meta,
    None => NoteMeta { count: 0 },
  };
  let idx = meta.count;
  meta.count += 1;
  NOTE_META.save(store, (recipient.clone(), sender.clone()), &meta)?;
  NOTES.save(store, (recipient, sender, idx), &note)?;
  Ok(())
}
