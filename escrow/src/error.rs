use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy)]
pub enum EscrowError {
    #[error("Invalid token amount")]
    InvalidAmount,
    #[error("Token account does not match the required mint")]
    InvalidMint,
    #[error("Token account is not owned by the user")]
    InvalidUser,
}

impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
