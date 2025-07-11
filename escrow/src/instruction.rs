use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum EscrowInstruction {
    Make { amount: u64, receive_amount: u64 },
    Take { amount: u64 },
    Refund,
}
