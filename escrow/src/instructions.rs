use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum EscrowInstructions {
    Make {
        amount_offered: u64,
        amount_required: u64,
    },
    Take {
        amount: u64,
    },
    Refund,
}
