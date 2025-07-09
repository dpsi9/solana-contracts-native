// Instruction definitions for the escrow contract
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum EscrowInstruction {
    // Define your instructions here
    // Example:
    Initialize { amount: u64 },
    Exchange,
    Cancel,
}
