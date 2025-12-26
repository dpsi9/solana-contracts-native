use litesvm::LiteSVM;

use solana_keypair::Keypair;
use solana_pubkey::{pubkey, Pubkey};
use solana_signer::Signer;

mod helpers;
use helpers::*;

const PROGRAM_ID: Pubkey = pubkey!("DinjxyZz2tjZTVi5FbKNoi2aayH71Q3EMzEh8yiGJnVY");

fn setup_escrow() -> (
    LiteSVM,
    Keypair,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    (Pubkey, u8),
    (Pubkey, u8),
) {
    let mut svm = LiteSVM::new();

    svm.add_program_from_file(PROGRAM_ID, "../target/deploy/escrow.so")
        .unwrap();

    assert!(svm.get_account(&PROGRAM_ID).unwrap().executable);

    
    let maker = Keypair::new();
    let taker = Keypair::new();
    
    svm.airdrop(&maker.pubkey(), 100_000_000_000).unwrap();
    svm.airdrop(&taker.pubkey(), 100_000_000_000).unwrap();
    
    let mint_a = create_mint(&mut svm, 6, &maker);
    let mint_b = create_mint(&mut svm, 6, &maker);
    
    let maker_token_a = create_token_account(&mut svm, &maker, &maker.pubkey(), &mint_a);
    let maker_token_b = create_token_account(&mut svm, &maker, &maker.pubkey(), &mint_b);
    
    let taker_token_a = create_token_account(&mut svm, &taker, &taker.pubkey(), &mint_a);
    let taker_token_b = create_token_account(&mut svm, &taker, &taker.pubkey(), &mint_b);
    
    mint_tokens(&mut svm, &maker, &mint_a, &maker, &maker_token_a, 500);
    mint_tokens(&mut svm, &taker, &mint_b, &maker, &taker_token_b, 300);
    println!("reached here");

    let (escrow_pda, escrow_bump) =
        derive_escrow_pda(&PROGRAM_ID, &maker.pubkey(), &mint_a, &mint_b);

    let (vault_pda, vault_bump) = derive_vault_pda(&escrow_pda, &PROGRAM_ID);

    (
        svm,
        maker,
        taker,
        mint_a,
        mint_b,
        maker_token_a,
        maker_token_b,
        taker_token_a,
        taker_token_b,
        (escrow_pda, escrow_bump),
        (vault_pda, vault_bump),
    )
}

#[test]
fn make() {
    let (
        mut svm,
        maker,
        taker,
        mint_a,
        mint_b,
        maker_token_a,
        maker_token_b,
        taker_token_a,
        taker_token_b,
        (escrow_pda, escrow_bump),
        (vault_pda, vault_bump),
    ) = setup_escrow();
}
