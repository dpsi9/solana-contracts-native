use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

pub fn create_mint(svm: &mut LiteSVM, decimals: u8, authority: &Keypair) -> Pubkey {
    CreateMint::new(svm, authority)
        .decimals(decimals)
        .authority(&authority.pubkey())
        .send()
        .unwrap()
}

pub fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    CreateAssociatedTokenAccount::new(svm, payer, mint)
        .owner(owner)
        .send()
        .unwrap()
}

pub fn mint_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    authority: &Keypair,
    destination: &Pubkey,
    amount: u64,
) {
    MintTo::new(svm, payer, mint, destination, amount)
        .owner(authority)
        .send()
        .unwrap();
}

pub fn derive_escrow_pda(
    program_id: &Pubkey,
    maker: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"escrow", maker.as_ref(), mint_a.as_ref(), mint_b.as_ref()],
        program_id,
    )
}

pub fn derive_vault_pda(escrow: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", escrow.as_ref()], program_id)
}
