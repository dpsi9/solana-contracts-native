use borsh::BorshDeserialize;
use litesvm::LiteSVM;

use escrow::state::Escrow;

use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_program_pack::Pack;
use solana_pubkey::{pubkey, Pubkey};
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_token_interface::state::Account as TokenAccount;

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

    mint_tokens(&mut svm, &maker, &mint_a, &maker, &maker_token_a, 100);
    mint_tokens(&mut svm, &taker, &mint_b, &maker, &taker_token_b, 50);

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

    // // Borsh 1.x uses 1-byte enum discriminant
    let mut instruction_data = vec![0u8];
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

    let vault = svm.get_account(&vault_pda).unwrap();
    let vault_token = TokenAccount::unpack(&vault.data).unwrap();
    assert_eq!(vault_token.amount, 100);

    let maker_token_account = svm.get_account(&maker_token_a).unwrap();
    let maker_token_amount = TokenAccount::unpack(&maker_token_account.data).unwrap();
    assert_eq!(maker_token_amount.amount, 0);

    let escrow_account = svm.get_account(&escrow_pda).unwrap();
    let escrow = Escrow::try_from_slice(&escrow_account.data).unwrap();
    assert_eq!(escrow.amount, 100);
    assert_eq!(escrow.receive_amount, 50);
    assert_eq!(escrow.owner, maker.pubkey());
    assert_eq!(escrow.mint_a, mint_a);
    assert_eq!(escrow.mint_b, mint_b);
}

#[test]
fn take() {
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
        (escrow_pda, _escrow_bump),
        (vault_pda, _vault_bump),
    ) = setup_escrow();

    let amount_offered: u64 = 100;
    let amount_required: u64 = 50;

    // First execute make instruction to create the escrow
    let mut make_data = vec![0u8]; // discriminator for make fn
    make_data.extend_from_slice(&amount_offered.to_le_bytes());
    make_data.extend_from_slice(&amount_required.to_le_bytes());

    let make_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new_readonly(mint_a, false),
            AccountMeta::new_readonly(mint_b, false),
            AccountMeta::new(maker_token_a, false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(solana_sysvar::rent::ID, false),
        ],
        data: make_data,
    };

    let blockhash = svm.latest_blockhash();
    let tx =
        Transaction::new_signed_with_payer(&[make_ix], Some(&maker.pubkey()), &[&maker], blockhash);
    svm.send_transaction(tx).expect("Make instruction failed");

    // Now execute take instruction
    let mut instruction_data = vec![1u8]; // discriminator for take fn
    instruction_data.extend_from_slice(&amount_required.to_le_bytes());

    let take_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(taker.pubkey(), true),
            AccountMeta::new(maker.pubkey(), false),
            AccountMeta::new(mint_a, false),
            AccountMeta::new(mint_b, false),
            AccountMeta::new(taker_token_a, false),
            AccountMeta::new(taker_token_b, false),
            AccountMeta::new(maker_token_b, false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(solana_system_interface::program::ID, false),
            AccountMeta::new_readonly(spl_token_interface::ID, false),
        ],
        data: instruction_data,
    };

    let recent_blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(
        &[take_ix],
        Some(&taker.pubkey()),
        &[&taker],
        recent_blockhash,
    );

    let result = svm.send_transaction(tx);

    match result {
        Ok(_meta) => {
            println!("take ix successful");
        }
        Err(e) => {
            panic!("Take ix failed: {:#?}", e);
        }
    }

    let taker_token_b_account = svm.get_account(&taker_token_b).unwrap();
    let taker_token_b_data = TokenAccount::unpack(&taker_token_b_account.data).unwrap();
    assert_eq!(taker_token_b_data.amount, 0);

    let maker_token_b_account = svm.get_account(&maker_token_b).unwrap();
    let maker_token_b_data = TokenAccount::unpack(&maker_token_b_account.data).unwrap();
    assert_eq!(maker_token_b_data.amount, amount_required);

    // Assert escrow account is closed (account no longer exists or has 0 lamports)
    let escrow_balance = svm.get_balance(&escrow_pda).unwrap_or(0);
    assert_eq!(escrow_balance, 0);
}
