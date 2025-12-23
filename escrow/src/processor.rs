use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::{self, ProgramResult},
    instruction::Instruction,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

use spl_token::{
    instruction::{initialize_account3, transfer_checked},
    solana_program::program_pack::Pack,
    state::{Account as TokenAccount, Mint},
};

use crate::instructions::EscrowInstructions;
use crate::state::Escrow;

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = EscrowInstructions::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        EscrowInstructions::Make {
            amount_offered,
            amount_required,
        } => make(program_id, accounts, amount_offered, amount_required)?,
        EscrowInstructions::Take { amount } => take(program_id, accounts, amount)?,
        EscrowInstructions::Refund => refund(program_id, accounts)?,
    }

    Ok(())
}

pub fn make(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount_offered: u64,
    amount_required: u64,
) -> ProgramResult {
    let accs = &mut accounts.iter();

    let maker = next_account_info(accs)?;
    let mint_a = next_account_info(accs)?;
    let mint_b = next_account_info(accs)?;
    let maker_token_a = next_account_info(accs)?;
    let escrow_state = next_account_info(accs)?;
    let escrow_vault = next_account_info(accs)?;
    let token_program = next_account_info(accs)?;
    let system_program = next_account_info(accs)?;
    let rent_sysvar = next_account_info(accs)?;

    if !maker.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if token_program.key.as_ref().ne(spl_token::id().as_ref()) {
        // not the right way to compare but still
        return Err(ProgramError::IncorrectProgramId);
    }

    let (escrow_pda, bump) = Pubkey::find_program_address(
        &[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
        ],
        program_id,
    );

    if escrow_pda != *escrow_state.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if escrow_state.owner != system_program.key {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::from_account_info(rent_sysvar)?;
    let escrow_lamports = rent.minimum_balance(Escrow::LEN);

    invoke_signed(
        &system_instruction::create_account(
            maker.key,
            escrow_state.key,
            escrow_lamports,
            Escrow::LEN as u64,
            program_id,
        ),
        &[maker.clone(), escrow_state.clone(), system_program.clone()],
        &[&[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
            &[bump],
        ]],
    )?;

    // create escrow vault
    let vault_lamports = rent.minimum_balance(TokenAccount::LEN);

    invoke(
        &system_instruction::create_account(
            maker.key,
            escrow_vault.key,
            vault_lamports,
            TokenAccount::LEN as u64,
            token_program.key,
        ),
        &[maker.clone(), escrow_vault.clone(), system_program.clone()],
    )?;

    invoke(
        &initialize_account3(
            token_program.key,
            escrow_vault.key,
            mint_a.key,
            escrow_state.key,
        )?,
        &[
            escrow_vault.clone(),
            mint_a.clone(),
            escrow_state.clone(),
            token_program.clone(),
        ],
    )?;

    // write escrow
    let escrow = Escrow {
        owner: *maker.key,
        mint_a: *mint_a.key,
        mint_b: *mint_b.key,
        amount: amount_offered,
        receive_amount: amount_required,
        bump,
    };
    escrow.serialize(&mut &mut escrow_state.data.borrow_mut()[..])?;

    // transfer tokens
    let mint = Mint::unpack(&mint_a.try_borrow_data()?)?;
    let ix = transfer_checked(
        token_program.key,
        maker_token_a.key,
        mint_a.key,
        escrow_vault.key,
        maker.key,
        &[],
        amount_offered,
        mint.decimals,
    )?;

    invoke(
        &ix,
        &[
            maker.clone(),
            maker_token_a.clone(),
            mint_a.clone(),
            escrow_vault.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}

pub fn take(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    Ok(())
}

pub fn refund(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    Ok(())
}
