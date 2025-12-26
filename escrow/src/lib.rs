pub mod error;
pub mod instructions;
pub mod processor;
pub mod state;

use solana_account_info::AccountInfo;
use solana_program_entrypoint::{entrypoint, ProgramResult};
use solana_pubkey::Pubkey;

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process(program_id, accounts, instruction_data)
}
