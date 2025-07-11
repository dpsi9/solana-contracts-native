use solana_program::program_error::ProgramError;

#[derive(Debug)]
pub enum EscrowError {
    InvalidAmount,
    Unauthorized,
}

impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
