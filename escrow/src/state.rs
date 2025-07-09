// State definitions for the escrow contract
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct EscrowAccount {
    // Define your account structure here
    // Example fields:
    // pub initializer_key: Pubkey,
    // pub initializer_deposit_token_account: Pubkey,
    // pub initializer_receive_token_account: Pubkey,
    // pub initializer_amount: u64,
    // pub taker_amount: u64,
}
