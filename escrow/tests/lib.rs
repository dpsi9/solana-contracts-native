use litesvm::LiteSVM;

use borsh::BorshSerialize;
use escrow::instructions::EscrowInstructions;

use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::{pubkey, Pubkey};
use solana_signer::Signer;
use solana_transaction::Transaction;

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
        _taker,
        mint_a,
        mint_b,
        maker_token_a,
        _maker_token_b,
        _taker_token_a,
        _taker_token_b,
        (escrow_pda, _escrow_bump),
        (vault_pda, _vault_bump),
    ) = setup_escrow();

    let amount_offered: u64 = 100;
    let amount_required: u64 = 50;

    // Borsh 1.x uses 1-byte enum discriminant
    let mut instruction_data = Vec::new();
    instruction_data.push(0u8); // Make = variant 0
    instruction_data.extend_from_slice(&amount_offered.to_le_bytes());
    instruction_data.extend_from_slice(&amount_required.to_le_bytes());

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new_readonly(mint_a, false),
            AccountMeta::new_readonly(mint_b, false),
            AccountMeta::new(maker_token_a, false),
            AccountMeta::new(escrow_pda, false), // PDA - not a signer from client
            AccountMeta::new(vault_pda, false),  // PDA - not a signer from client
            AccountMeta::new_readonly(spl_token_interface::ID, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(solana_sysvar::rent::ID, false),
        ],
        data: instruction_data,
    };

    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&maker.pubkey()), &[&maker], blockhash);

    let result = svm.send_transaction(tx);

    match result {
        Ok(_meta) => {
            println!("Make ix succeeded!");
        }
        Err(e) => {
            panic!("Make ix failed: {:#?}", e);
        }
    }
}
