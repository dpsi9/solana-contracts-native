use litesvm::LiteSVM;

use solana_keypair::Keypair;
use solana_pubkey::{pubkey, Pubkey};
use solana_signer::Signer;

mod helpers;
use helpers::*;

const PROGRAM_ID: Pubkey = pubkey!("DinjxyZz2tjZTVi5FbKNoi2aayH71Q3EMzEh8yiGJnVY");

#[test]
fn test() {
    let mut svm = LiteSVM::new();

    svm.add_program_from_file(PROGRAM_ID, "../target/deploy/escrow.so").unwrap();

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 100_000_000_000).unwrap();

    let mint = create_mint(&mut svm, 6, &maker);

    println!("mint: {}", mint);
}
