use borsh::{BorshDeserialize, BorshSerialize};

use solana_cpi::{invoke, invoke_signed};
use solana_system_interface::instruction as system_instruction;

use solana_account_info::{next_account_info, AccountInfo};
use solana_program_entrypoint::ProgramResult;
use solana_program_error::ProgramError;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_system_interface::program as system_program;
use solana_sysvar::{rent::Rent, SysvarSerialize};

use spl_token_interface::{
    instruction,
    state::{Account as TokenAccount, Mint},
    ID as TOKEN_PROGRAM_ID,
};

use crate::{instructions::EscrowInstructions, state::Escrow};

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
        } => make(program_id, accounts, amount_offered, amount_required),
        EscrowInstructions::Take { amount } => take(program_id, accounts, amount),
        EscrowInstructions::Refund => refund(program_id, accounts),
    }
}

pub fn make(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount_offered: u64,
    amount_required: u64,
) -> ProgramResult {
    if amount_offered == 0 || amount_required == 0 {
        return Err(ProgramError::InvalidArgument);
    }

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

    if system_program.key != &system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if token_program.key != &TOKEN_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    if mint_a.key == mint_b.key {
        return Err(ProgramError::InvalidArgument);
    }

    if maker_token_a.owner != token_program.key {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let maker_token_a_account = TokenAccount::unpack(&maker_token_a.data.borrow())?;
    if maker_token_a_account.owner != *maker.key {
        return Err(ProgramError::IllegalOwner);
    }
    if maker_token_a_account.mint != *mint_a.key {
        return Err(ProgramError::InvalidAccountData);
    }
    if maker_token_a_account.amount < amount_offered {
        return Err(ProgramError::InsufficientFunds);
    }

    let (escrow_pda, escrow_bump) = Pubkey::find_program_address(
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

    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[b"vault", escrow_state.key.as_ref()], program_id);

    if vault_pda != *escrow_vault.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if escrow_state.lamports() != 0 || escrow_vault.lamports() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::from_account_info(rent_sysvar)?;

    invoke_signed(
        &system_instruction::create_account(
            maker.key,
            escrow_state.key,
            rent.minimum_balance(Escrow::LEN),
            Escrow::LEN as u64,
            program_id,
        ),
        &[maker.clone(), escrow_state.clone(), system_program.clone()],
        &[&[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
            &[escrow_bump],
        ]],
    )?;

    invoke_signed(
        &system_instruction::create_account(
            maker.key,
            escrow_vault.key,
            rent.minimum_balance(TokenAccount::LEN),
            TokenAccount::LEN as u64,
            token_program.key,
        ),
        &[maker.clone(), escrow_vault.clone(), system_program.clone()],
        &[&[b"vault", escrow_state.key.as_ref(), &[vault_bump]]],
    )?;

    invoke(
        &instruction::initialize_account3(
            token_program.key,
            escrow_vault.key,
            mint_a.key,
            &escrow_pda,
        )?,
        &[
            escrow_vault.clone(),
            mint_a.clone(),
            escrow_state.clone(),
            token_program.clone(),
        ],
    )?;

    let mint_info = Mint::unpack(&mint_a.data.borrow())?;

    invoke(
        &instruction::transfer_checked(
            token_program.key,
            maker_token_a.key,
            mint_a.key,
            escrow_vault.key,
            maker.key,
            &[maker.key],
            amount_offered,
            mint_info.decimals,
        )?,
        &[
            maker_token_a.clone(),
            mint_a.clone(),
            escrow_vault.clone(),
            maker.clone(),
            token_program.clone(),
        ],
    )?;

    let escrow = Escrow {
        owner: *maker.key,
        mint_a: *mint_a.key,
        mint_b: *mint_b.key,
        amount: amount_offered,
        receive_amount: amount_required,
        bump: escrow_bump,
        vault_bump,
    };

    escrow.serialize(&mut &mut escrow_state.data.borrow_mut()[..])?;
    Ok(())
}

pub fn take(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    let accs = &mut accounts.iter();

    let taker = next_account_info(accs)?;
    let maker = next_account_info(accs)?;
    let mint_a = next_account_info(accs)?;
    let mint_b = next_account_info(accs)?;
    let taker_token_a = next_account_info(accs)?;
    let taker_token_b = next_account_info(accs)?;
    let maker_token_b = next_account_info(accs)?;
    let escrow_state = next_account_info(accs)?;
    let escrow_vault = next_account_info(accs)?;
    let system_program = next_account_info(accs)?;
    let token_program = next_account_info(accs)?;

    if !taker.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if system_program.key != &system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    if token_program.key != &TOKEN_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    let escrow = Escrow::try_from_slice(&escrow_state.data.borrow())?;
    if amount != escrow.receive_amount {
        return Err(ProgramError::InvalidArgument);
    }

    let (escrow_pda, escrow_bump) = Pubkey::find_program_address(
        &[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
        ],
        program_id,
    );

    let mint_a_info = Mint::unpack(&mint_a.data.borrow())?;
    let mint_b_info = Mint::unpack(&mint_b.data.borrow())?;

    invoke(
        &instruction::transfer_checked(
            token_program.key,
            taker_token_b.key,
            mint_b.key,
            maker_token_b.key,
            taker.key,
            &[taker.key],
            escrow.receive_amount,
            mint_b_info.decimals,
        )?,
        &[
            taker_token_b.clone(),
            mint_b.clone(),
            maker_token_b.clone(),
            taker.clone(),
            token_program.clone(),
        ],
    )?;

    invoke_signed(
        &instruction::transfer_checked(
            token_program.key,
            escrow_vault.key,
            mint_a.key,
            taker_token_a.key,
            &escrow_pda,
            &[],
            escrow.amount,
            mint_a_info.decimals,
        )?,
        &[
            escrow_vault.clone(),
            mint_a.clone(),
            taker_token_a.clone(),
            escrow_state.clone(),
            token_program.clone(),
        ],
        &[&[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
            &[escrow_bump],
        ]],
    )?;

    invoke_signed(
        &instruction::close_account(
            token_program.key,
            escrow_vault.key,
            maker.key,
            &escrow_pda,
            &[],
        )?,
        &[
            escrow_vault.clone(),
            maker.clone(),
            escrow_state.clone(),
            token_program.clone(),
        ],
        &[&[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
            &[escrow_bump],
        ]],
    )?;

    **maker.lamports.borrow_mut() += escrow_state.lamports();
    **escrow_state.lamports.borrow_mut() = 0;
    escrow_state.data.borrow_mut().fill(0);

    Ok(())
}

pub fn refund(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accs = &mut accounts.iter();

    let maker = next_account_info(accs)?;
    let mint_a = next_account_info(accs)?;
    let mint_b = next_account_info(accs)?;
    let maker_token_a = next_account_info(accs)?;
    let escrow_state = next_account_info(accs)?;
    let escrow_vault = next_account_info(accs)?;
    let token_program = next_account_info(accs)?;

    if !maker.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if token_program.key != &TOKEN_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    let escrow = Escrow::try_from_slice(&escrow_state.data.borrow())?;

    let (escrow_pda, escrow_bump) = Pubkey::find_program_address(
        &[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
        ],
        program_id,
    );

    let mint_info = Mint::unpack(&mint_a.data.borrow())?;

    invoke_signed(
        &instruction::transfer_checked(
            token_program.key,
            escrow_vault.key,
            mint_a.key,
            maker_token_a.key,
            &escrow_pda,
            &[],
            escrow.amount,
            mint_info.decimals,
        )?,
        &[
            escrow_vault.clone(),
            mint_a.clone(),
            maker_token_a.clone(),
            escrow_state.clone(),
            token_program.clone(),
        ],
        &[&[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
            &[escrow_bump],
        ]],
    )?;

    invoke_signed(
        &instruction::close_account(
            token_program.key,
            escrow_vault.key,
            maker.key,
            &escrow_pda,
            &[],
        )?,
        &[
            escrow_vault.clone(),
            maker.clone(),
            escrow_state.clone(),
            token_program.clone(),
        ],
        &[&[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
            &[escrow_bump],
        ]],
    )?;

    **maker.try_borrow_mut_lamports()? += escrow_state.lamports();
    **escrow_state.try_borrow_mut_lamports()? = 0;
    escrow_state.data.borrow_mut().fill(0);

    Ok(())
}
