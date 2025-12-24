use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::{self, ProgramResult},
    instruction::Instruction,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction, system_program,
    sysvar::{rent::Rent, Sysvar},
};

use spl_token::{
    solana_program::program_pack::Pack,
    state::{Account as TokenAccount, Mint},
};

use crate::state::Escrow;
use crate::{error::EscrowError, instructions::EscrowInstructions};

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
        // check if the tx is signed by maker
        return Err(ProgramError::MissingRequiredSignature);
    }

    if system_program.key != &system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if token_program.key != &spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if mint_a.key == mint_b.key {
        return Err(ProgramError::InvalidArgument);
    }

    if maker_token_a.owner != token_program.key {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if mint_a.owner != token_program.key || mint_b.owner != token_program.key {
        return Err(ProgramError::InvalidAccountOwner);
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

    let maker_token_a_account = TokenAccount::unpack(&maker_token_a.data.borrow_mut())?;
    if maker_token_a_account.owner != *maker.key {
        return Err(ProgramError::IllegalOwner);
    }
    if maker_token_a_account.mint != *mint_a.key {
        return Err(ProgramError::InvalidAccountData);
    }
    if maker_token_a_account.amount < amount_offered {
        return Err(ProgramError::InsufficientFunds);
    }

    if escrow_state.lamports() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    if escrow_vault.lamports() != 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::from_account_info(rent_sysvar)?;

    let escrow_lamports = rent.minimum_balance(Escrow::LEN);
    invoke(
        &system_instruction::create_account(
            maker.key,
            escrow_state.key,
            escrow_lamports,
            Escrow::LEN as u64,
            program_id,
        ),
        &[maker.clone(), escrow_state.clone(), system_program.clone()],
    )?;

    if !rent.is_exempt(escrow_state.lamports(), Escrow::LEN) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    let vault_lamports = rent.minimum_balance(TokenAccount::LEN);
    invoke(
        &system_instruction::create_account(
            maker.key,
            escrow_vault.key,
            vault_lamports,
            TokenAccount::LEN as u64,
            token_program.key,
        ),
        &[
            maker.clone(),
            escrow_vault.clone(),
            system_program.clone(), // no need to declare token_program here as it is only the owner(pubkey)
        ],
    )?;
    if !rent.is_exempt(escrow_vault.lamports(), TokenAccount::LEN) {
        return Err(ProgramError::AccountNotRentExempt);
    }
    invoke(
        &spl_token::instruction::initialize_account3(
            token_program.key,
            escrow_vault.key,
            mint_a.key,
            &escrow_pda,
        )?,
        &[
            escrow_vault.clone(),
            mint_a.clone(),
            escrow_state.clone(),
            token_program.clone(), // here the owner is needed(just how initialize_account3 works)
        ],
    )?;

    let mint_info = Mint::unpack(&mint_a.data.borrow())?;
    invoke(
        &spl_token::instruction::transfer_checked(
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

// transfer tokens from taker_token_b to maker_token_b && escrow_vault to taker_token_a, close the escrow and transfer lamports back to maker
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
    if token_program.key != &spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    if mint_a.key == mint_b.key {
        return Err(ProgramError::InvalidArgument);
    }
    if taker_token_a.owner != token_program.key
        || taker_token_b.owner != token_program.key
        || maker_token_b.owner != token_program.key
    {
        return Err(ProgramError::InvalidAccountOwner);
    }

    if escrow_state.owner != program_id {
        return Err(ProgramError::InvalidAccountOwner);
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

    if escrow_pda != *escrow_state.key {
        return Err(ProgramError::InvalidSeeds);
    }
    if amount != escrow.receive_amount {
        return Err(ProgramError::InvalidArgument);
    }

    let (vault_pda, vault_bump) =
        Pubkey::find_program_address(&[b"vault", escrow_state.key.as_ref()], program_id);

    if vault_pda != *escrow_vault.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if escrow.owner != *maker.key {
        return Err(ProgramError::IllegalOwner);
    }
    if escrow.mint_a != *mint_a.key || escrow.mint_b != *mint_b.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let taker_b_account = TokenAccount::unpack(&taker_token_b.data.borrow())?;
    if taker_b_account.owner != *taker.key {
        return Err(ProgramError::IllegalOwner);
    }
    if taker_b_account.mint != *mint_b.key {
        return Err(ProgramError::InvalidAccountData);
    }
    if taker_b_account.amount < escrow.receive_amount {
        return Err(ProgramError::InsufficientFunds);
    }

    let taker_a_account = TokenAccount::unpack(&taker_token_a.data.borrow())?;
    if taker_a_account.owner != *taker.key {
        return Err(ProgramError::IllegalOwner);
    }
    if taker_a_account.mint != *mint_a.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let maker_b_account = TokenAccount::unpack(&maker_token_b.data.borrow())?;
    if maker_b_account.owner != *maker.key {
        return Err(ProgramError::IllegalOwner);
    }
    if maker_b_account.mint != *mint_b.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let vault_account = TokenAccount::unpack(&escrow_vault.data.borrow())?;
    if vault_account.owner != escrow_pda {
        return Err(ProgramError::IllegalOwner);
    }
    if vault_account.mint != *mint_a.key {
        return Err(ProgramError::InvalidAccountData);
    }
    if vault_account.amount < escrow.amount {
        return Err(ProgramError::InsufficientFunds);
    }

    let mint_a_info = Mint::unpack(&mint_a.data.borrow())?;
    let mint_b_info = Mint::unpack(&mint_b.data.borrow())?;

    // transfer tokens from taker_token_b_account to maker_token_b_account
    invoke(
        &spl_token::instruction::transfer_checked(
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

    // transfer tokens from escrow_vault to taker_token_a_account
    invoke_signed(
        &spl_token::instruction::transfer_checked(
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

    // close escrow_vault and return lamports back to the maker
    invoke_signed(
        &spl_token::instruction::close_account(
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

    // close the escrow_pda(optional) as it is one time pda

    **maker.lamports.borrow_mut() += escrow_state.lamports();
    **escrow_state.lamports.borrow_mut() = 0;

    escrow_state.data.borrow_mut().fill(0);
    Ok(())
}

pub fn refund(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    Ok(())
}
