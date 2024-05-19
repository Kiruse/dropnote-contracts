use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("Unauthorized")]
  Unauthorized {},

  #[error("Insufficient funds")]
  InsufficientFunds {},

  // Add any other custom errors you like here.
  // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

impl From<ContractError> for StdError {
  fn from(err: ContractError) -> StdError {
    StdError::GenericErr {
      msg: err.to_string(),
    }
  }
}
