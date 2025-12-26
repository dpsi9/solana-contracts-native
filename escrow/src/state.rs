use borsh::{BorshDeserialize, BorshSerialize};
use solana_pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Escrow {
    pub owner: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub amount: u64,
    pub receive_amount: u64,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Escrow {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 1 + 1; // owner + mint_a + mint_b + amount + receive_amount + bump + vault_bump
}
