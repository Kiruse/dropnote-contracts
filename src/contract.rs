#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use cw2::set_contract_version;

use crate::ContractResult;
use crate::error::ContractError;
use crate::msg::{EncryptionKeyResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{find_senders, load_enc_key, load_fees, load_note_meta, load_notes, save_enc_key, save_fees, store_note, Fees, Note};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
  deps: DepsMut,
  _env: Env,
  info: MessageInfo,
  msg: InstantiateMsg,
) -> ContractResult<Response> {
  set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

  save_fees(deps.storage, &Fees {
    admin: Some(info.sender.clone()),
    store_keys: msg.store_keys_fee,
    store_notes: msg.store_notes_fee,
    denom: msg.denom,
    burn_fees: false,
  })?;

  Ok(Response::new()
    .add_attribute("method", "instantiate")
    .add_attribute("new_store_keys_fee", msg.store_keys_fee.to_string())
    .add_attribute("new_store_notes_fee", msg.store_notes_fee.to_string())
  )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
  deps: DepsMut,
  env: Env,
  info: MessageInfo,
  msg: ExecuteMsg,
) -> ContractResult<Response> {
  use ExecuteMsg::*;
  let ctx = ExecuteContext { deps, env, info };
  match msg {
    UpdateFees { admin, store_keys, store_notes, denom, burn_fees } =>
      exec_update_fees(ctx, admin, store_keys, store_notes, denom, burn_fees),
    UpdateKey { key } => exec_update_key(ctx, key),
    StoreNote { recipient, note } => exec_store_note(ctx, recipient, note),
  }
}

fn exec_update_fees(ctx: ExecuteContext, admin: Option<String>, store_keys: Uint128, store_notes: Uint128, denom: String, burn_fees: bool) -> ContractResult<Response> {
  let fees = load_fees(ctx.deps.storage)?;
  if fees.admin.is_none() || fees.admin.unwrap() != ctx.info.sender {
    return Err(ContractError::Unauthorized {});
  }

  let admin = admin.map(|a| ctx.deps.api.addr_validate(a.as_str())).transpose()?;
  save_fees(ctx.deps.storage, &Fees {
    admin,
    store_keys,
    store_notes,
    denom,
    burn_fees,
  })?;
  Ok(Response::new().add_attribute("method", "update_fees"))
}

fn exec_update_key(ctx: ExecuteContext, key: String) -> ContractResult<Response> {
  let fees = load_fees(ctx.deps.storage)?;
  let mut msgs: Vec<CosmosMsg> = vec![];

  if fees.store_keys > Uint128::zero() {
    let coin = find_coin(&fees.denom, &ctx.info.funds);
    match coin {
      Some(coin) => {
        if coin.amount < fees.store_keys {
          return Err(ContractError::InsufficientFunds {});
        }
        msgs.push(get_fee_msg(&fees, &coin));
      }
      None => return Err(ContractError::InsufficientFunds {}),
    }
  }

  save_enc_key(ctx.deps.storage, ctx.info.sender.clone(), &key.as_bytes().to_owned())?;

  Ok(Response::new()
    .add_attribute("method", "update_key")
    .add_messages(msgs)
  )
}

fn exec_store_note(ctx: ExecuteContext, recipient: String, note: String) -> ContractResult<Response> {
  let fees = load_fees(ctx.deps.storage)?;
  let mut msgs: Vec<CosmosMsg> = vec![];

  if fees.store_notes > Uint128::zero() {
    let coin = find_coin(&fees.denom, &ctx.info.funds);
    match coin {
      Some(coin) => {
        if coin.amount < fees.store_notes {
          return Err(ContractError::InsufficientFunds {});
        }
        msgs.push(get_fee_msg(&fees, &coin));
      }
      None => return Err(ContractError::InsufficientFunds {}),
    }
  }

  let recipient = ctx.deps.api.addr_validate(recipient.as_str())?;
  let note = Note {
    sender: ctx.info.sender.clone(),
    note: note.as_bytes().to_owned(),
    timestamp: ctx.env.block.time,
  };
  store_note(ctx.deps.storage, ctx.info.sender.clone(), recipient.clone(), note)?;

  Ok(Response::new()
    .add_attribute("method", "store_note")
    .add_attribute("recipient", recipient)
    .add_messages(msgs)
  )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
  let ctx = QueryContext { deps, env };
  let response = match msg {
    QueryMsg::Fees {} => to_json_binary(&query_fees(&ctx)?)?,
    QueryMsg::NoteCount { recipient, sender } => to_json_binary(&query_note_count(&ctx, recipient, sender)?)?,
    QueryMsg::Senders { recipient } => to_json_binary(&query_senders(&ctx, recipient)?)?,
    QueryMsg::EncryptionKey { address } => to_json_binary(&query_enc_key(&ctx, address)?)?,
    QueryMsg::Notes { recipient, sender, start_after, limit } =>
      to_json_binary(&query_notes(&ctx, recipient, sender, start_after, limit)?)?,
  };

  Ok(response)
}

fn query_fees(ctx: &QueryContext) -> ContractResult<Fees> {
  let fees = load_fees(ctx.deps.storage)?;
  Ok(fees)
}

fn query_note_count(ctx: &QueryContext, recipient: String, sender: String) -> ContractResult<u64> {
  let recipient = ctx.deps.api.addr_validate(recipient.as_str())?;
  let sender = ctx.deps.api.addr_validate(sender.as_str())?;
  let meta = load_note_meta(ctx.deps.storage, sender, recipient)?;
  Ok(meta.count)
}

fn query_senders(ctx: &QueryContext, recipient: String) -> ContractResult<Vec<String>> {
  let recipient = ctx.deps.api.addr_validate(recipient.as_str())?;
  let senders = find_senders(ctx.deps.storage, recipient)?;
  Ok(senders.iter().map(|a| a.to_string()).collect())
}

fn query_enc_key(ctx: &QueryContext, address: String) -> ContractResult<EncryptionKeyResponse> {
  let address = ctx.deps.api.addr_validate(address.as_str())?;
  let key = load_enc_key(ctx.deps.storage, address)?;
  Ok(EncryptionKeyResponse { key })
}

fn query_notes(ctx: &QueryContext, recipient: String, sender: String, start_after: Option<u64>, limit: Option<u32>) -> ContractResult<Vec<Note>> {
  let recipient = ctx.deps.api.addr_validate(recipient.as_str())?;
  let sender = ctx.deps.api.addr_validate(sender.as_str())?;
  let notes = load_notes(ctx.deps.storage, recipient, sender, start_after, limit)?;
  Ok(notes)
}

fn find_coin(denom: &str, coins: &Vec<Coin>) -> Option<Coin> {
  coins.iter().find(|coin| coin.denom == denom).cloned()
}

fn get_fee_msg(fees: &Fees, coin: &Coin) -> CosmosMsg {
  let sendmsg = BankMsg::Send {
    to_address: fees.admin.clone().map(|a| a.to_string()).unwrap_or("".to_string()),
    amount: vec![coin.clone()],
  };
  let burnmsg = BankMsg::Burn {
    amount: vec![coin.clone()],
  };
  match fees.admin {
    Some(_) => {
      if fees.burn_fees {
        CosmosMsg::Bank(burnmsg)
      } else {
        CosmosMsg::Bank(sendmsg)
      }
    }
    None => CosmosMsg::Bank(burnmsg),
  }
}

struct ExecuteContext<'a> {
  deps: DepsMut<'a>,
  #[allow(dead_code)]
  env: Env,
  info: MessageInfo,
}

struct QueryContext<'a> {
  deps: Deps<'a>,
  #[allow(dead_code)]
  env: Env,
}

#[cfg(test)]
mod tests {
  use super::*;
  use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
  use cosmwasm_std::{coins, from_json, SubMsg};

  #[test]
  fn init() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
      denom: "luna".to_string(),
      store_keys_fee: Uint128::new(1000000), // 1L
      store_notes_fee: Uint128::new(500000), // 0.5L
    };
    let info = mock_info("admin", &vec![]);

    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
  }

  #[test]
  fn store_stuff() {
    let mut owndeps = mock_dependencies();
    instantiate_no_fees(owndeps.as_mut());

    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &vec![]),
    };
    exec_update_key(ctx, "foobar".to_string()).unwrap();

    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &vec![]),
    };
    exec_store_note(ctx, "bob".to_string(), "barfoo".to_string()).unwrap();

    let msg = QueryMsg::EncryptionKey { address: "alice".to_string() };
    let bin = query(owndeps.as_ref(), mock_env(), msg).unwrap();
    let res = from_json::<EncryptionKeyResponse>(&bin).unwrap();
    assert_eq!(res.key.unwrap(), "foobar".as_bytes().to_owned());

    let msg = QueryMsg::NoteCount { recipient: "bob".to_string(), sender: "alice".to_string() };
    let bin = query(owndeps.as_ref(), mock_env(), msg).unwrap();
    let res = from_json::<u64>(&bin).unwrap();
    assert_eq!(res, 1);

    let msg = QueryMsg::Notes { recipient: "bob".to_string(), sender: "alice".to_string(), start_after: None, limit: None };
    let bin = query(owndeps.as_ref(), mock_env(), msg).unwrap();
    let res = from_json::<Vec<Note>>(&bin).unwrap();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].note, "barfoo".as_bytes().to_owned());
  }

  #[test]
  fn fees_update_key() {
    let mut owndeps = mock_dependencies();
    instantiate_default(owndeps.as_mut());

    // fails with insufficient fees
    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &coins(1, "luna")),
    };
    exec_update_key(ctx, "foobar".to_string()).expect_err("Unexpected success");

    // fails with no funds
    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &vec![]),
    };
    exec_update_key(ctx, "foobar".to_string()).expect_err("Unexpected success");

    // success with exact fees
    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &coins(1000000, "luna")),
    };
    let res = exec_update_key(ctx, "foobar".to_string()).unwrap();
    assert!(res.messages.iter().any(|submsg| is_fee_msg(submsg, "admin", "luna", 1000000)));

    // success with excess fees
    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &coins(1500000, "luna")),
    };
    let res = exec_update_key(ctx, "foobar".to_string()).unwrap();
    assert!(res.messages.iter().any(|submsg| is_fee_msg(submsg, "admin", "luna", 1500000)));
  }

  #[test]
  fn fees_store_note() {
    let mut owndeps = mock_dependencies();
    instantiate_default(owndeps.as_mut());

    // fails with insufficient fees
    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &coins(1, "luna")),
    };
    exec_store_note(ctx, "bob".to_string(), "barfoo".to_string()).expect_err("Unexpected success");

    // fails with no funds
    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &vec![]),
    };
    exec_store_note(ctx, "bob".to_string(), "barfoo".to_string()).expect_err("Unexpected success");

    // success with exact fees
    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &coins(500000, "luna")),
    };
    let res = exec_store_note(ctx, "bob".to_string(), "barfoo".to_string()).unwrap();
    assert!(res.messages.iter().any(|submsg| is_fee_msg(submsg, "admin", "luna", 500000)));

    // success with excess fees
    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &coins(750000, "luna")),
    };
    let res = exec_store_note(ctx, "bob".to_string(), "barfoo".to_string()).unwrap();
    assert!(res.messages.iter().any(|submsg| is_fee_msg(submsg, "admin", "luna", 750000)));
  }

  #[test]
  fn update_fees() {
    let mut owndeps = mock_dependencies();
    instantiate_default(owndeps.as_mut());

    // fails with unauthorized
    let msg = ExecuteMsg::UpdateFees {
      admin: None,
      store_keys: Uint128::new(1000000),
      store_notes: Uint128::new(500000),
      denom: "luna".to_string(),
      burn_fees: false,
    };
    execute(owndeps.as_mut(), mock_env(), mock_info("alice", &[]), msg).expect_err("Unexpected success");

    // success with authorized
    let msg = ExecuteMsg::UpdateFees {
      admin: Some("alice".to_string()),
      store_keys: Uint128::new(2000000),
      store_notes: Uint128::new(1000000),
      denom: "luna".to_string(),
      burn_fees: false,
    };
    execute(owndeps.as_mut(), mock_env(), mock_info("admin", &[]), msg).unwrap();

    // query updated fees
    let msg = QueryMsg::Fees {};
    let bin = query(owndeps.as_ref(), mock_env(), msg).unwrap();
    let res = from_json::<Fees>(&bin).unwrap();
    assert_eq!(res.admin.unwrap(), "alice");
    assert_eq!(res.store_keys, Uint128::new(2000000));
    assert_eq!(res.store_notes, Uint128::new(1000000));

    // success with new admin
    let msg = ExecuteMsg::UpdateFees {
      admin: Some("bob".to_string()),
      store_keys: Uint128::new(3000000),
      store_notes: Uint128::new(1500000),
      denom: "luna".to_string(),
      burn_fees: false,
    };
    execute(owndeps.as_mut(), mock_env(), mock_info("alice", &[]), msg).unwrap();

    // failure with old admin
    let msg = ExecuteMsg::UpdateFees {
      admin: Some("alice".to_string()),
      store_keys: Uint128::new(4000000),
      store_notes: Uint128::new(2000000),
      denom: "luna".to_string(),
      burn_fees: false,
    };
    execute(owndeps.as_mut(), mock_env(), mock_info("alice", &[]), msg).expect_err("Unexpected success");
  }

  #[test]
  fn query_senders() {
    let mut owndeps = mock_dependencies();
    instantiate_no_fees(owndeps.as_mut());

    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &vec![]),
    };
    exec_store_note(ctx, "bob".to_string(), "barfoo".to_string()).unwrap();

    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &vec![]),
    };
    exec_store_note(ctx, "bob".to_string(), "barfoo".to_string()).unwrap();

    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("charlie", &vec![]),
    };
    exec_store_note(ctx, "bob".to_string(), "barfoo".to_string()).unwrap();

    let msg = QueryMsg::Senders { recipient: "bob".to_string() };
    let bin = query(owndeps.as_ref(), mock_env(), msg).unwrap();
    let res = from_json::<Vec<String>>(&bin).unwrap();
    assert_eq!(res, vec!["alice", "charlie"]);
  }

  #[test]
  fn query_note_count() {
    let mut owndeps = mock_dependencies();
    instantiate_no_fees(owndeps.as_mut());

    let ctx = ExecuteContext {
      deps: owndeps.as_mut(),
      env: mock_env(),
      info: mock_info("alice", &vec![]),
    };
    exec_store_note(ctx, "bob".to_string(), "foobar".to_string()).unwrap();

    let msg = QueryMsg::NoteCount { recipient: "bob".to_string(), sender: "alice".to_string() };
    let bin = query(owndeps.as_ref(), mock_env(), msg).unwrap();
    let res = from_json::<u64>(&bin).unwrap();
    assert_eq!(res, 1);
  }

  fn instantiate_no_fees<'a>(deps: DepsMut<'a>) {
    instantiate(deps, mock_env(), mock_info("admin", &vec![]), InstantiateMsg {
      denom: "luna".to_string(),
      store_keys_fee: Uint128::zero(),
      store_notes_fee: Uint128::zero(),
    }).unwrap();
  }

  fn instantiate_default<'a>(deps: DepsMut<'a>) {
    instantiate(deps, mock_env(), mock_info("admin", &vec![]), InstantiateMsg {
      denom: "luna".to_string(),
      store_keys_fee: Uint128::new(1000000), // 1L
      store_notes_fee: Uint128::new(500000), // 0.5L
    }).unwrap();
  }

  fn is_fee_msg(submsg: &SubMsg, recipient: &str, denom: &str, amount: u128) -> bool {
    if let CosmosMsg::Bank(BankMsg::Send { to_address, amount: coins }) = &submsg.msg {
      if to_address != recipient {
        return false;
      }
      let coin = coins.iter().find(|coin| coin.denom == denom);
      if let None = coin {
        return false;
      }
      let coin = coin.unwrap();
      return coin.amount == Uint128::from(amount);
    }
    false
  }
}
