use crate::error::EscrowError;
use crate::instruction::EscrowInstruction;
use crate::state::EscrowAccount;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

use spl_token_2022::{
    instruction::initialize_account3, instruction::transfer_checked,
    state::Account as TokenAccount, state::Mint, ID as TOKEN_2022_ID,
};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = EscrowInstruction::try_from_slice(input)?;

    match instruction {
        EscrowInstruction::Make {
            amount,
            receive_amount,
        } => make(program_id, accounts, amount, receive_amount),
        EscrowInstruction::Take { amount } => take(program_id, accounts, amount),
        EscrowInstruction::Refund => refund(program_id, accounts),
    }
}

pub fn make(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    receive_amount: u64,
) -> ProgramResult {
    let acc = &mut accounts.iter();

    let maker = next_account_info(acc)?;
    let mint_a = next_account_info(acc)?;
    let mint_b = next_account_info(acc)?;
    let maker_token_a = next_account_info(acc)?;
    let escrow_token_a = next_account_info(acc)?;
    let escrow_account = next_account_info(acc)?;
    let token_program = next_account_info(acc)?;
    let system_program = next_account_info(acc)?;
    let rent_sysvar = next_account_info(acc)?;

    // Derive and validate escrow account pda
    let (expected_pda, bump) = Pubkey::find_program_address(
        &[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
        ],
        program_id,
    );

    if expected_pda != *escrow_account.key {
        msg!("Invalid escrow PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    // create escrow token account manually
    let rent = &Rent::from_account_info(rent_sysvar)?;
    let lamports = rent.minimum_balance(TokenAccount::LEN);

    invoke_signed(
        &system_instruction::create_account(
            maker.key,
            escrow_token_a.key,
            lamports,
            TokenAccount::LEN as u64,
            token_program.key,
        ),
        &[
            maker.clone(),
            escrow_token_a.clone(),
            system_program.clone(),
        ],
        &[&[
            b"escrow",
            maker.key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
            &[bump],
        ]],
    )?;

    invoke(
        &initialize_account3(
            token_program.key,
            escrow_token_a.key,
            mint_a.key,
            escrow_account.key,
        )?,
        &[
            escrow_token_a.clone(),
            mint_a.clone(),
            escrow_account.clone(),
            token_program.clone(),
        ],
    )?;

    let escrow = EscrowAccount {
        maker: *maker.key,
        mint_a: *mint_a.key,
        mint_b: *mint_b.key,
        amount,
        receive_amount,
        bump,
    };

    escrow.serialize(&mut &mut escrow_account.data.borrow_mut()[..])?;

    let mint_data = &mint_a.try_borrow_data()?;
    let mint = Mint::unpack(mint_data)?;
    let decimals = mint.decimals;

    let ix = transfer_checked(
        token_program.key,
        maker_token_a.key,
        mint_a.key,
        escrow_token_a.key,
        maker.key,
        &[],
        amount,
        decimals,
    )?;

    invoke(
        &ix,
        &[
            maker.clone(),
            maker_token_a.clone(),
            mint_a.clone(),
            escrow_token_a.clone(),
            token_program.clone(),
        ],
    )?;

    msg!(
        "Escrow created: Offering {} tokens for {}",
        amount,
        receive_amount
    );

    Ok(())
}

pub fn take(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let acc = &mut accounts.iter();

    let taker = next_account_info(acc)?;
    let maker = next_account_info(acc)?;
    let escrow_account = next_account_info(acc)?;
    let escrow_token_a = next_account_info(acc)?;
    let taker_token_a = next_account_info(acc)?;
    let taker_token_b = next_account_info(acc)?;
    let maker_token_b = next_account_info(acc)?;
    let mint_a = next_account_info(acc)?;
    let mint_b = next_account_info(acc)?;
    let token_program = next_account_info(acc)?;
    let system_program = next_account_info(acc)?;

    let escrow_data = &mut &**escrow_account.data.borrow();
    let escrow: EscrowAccount = EscrowAccount::deserialize(escrow_data)?;

    if amount != escrow.amount {
        msg!("Invalid amount");
        return Err(ProgramError::InvalidArgument);
    }

    // transfer mint_b token from taker to maker
    //first calculate decimals
    let mint_b_data = &mint_b.try_borrow_data()?;
    let mint = Mint::unpack(mint_b_data)?;
    let decimal_b = mint.decimals;
    // instruction
    let ix_b = transfer_checked(
        token_program.key,
        taker_token_b.key,
        mint_b.key,
        maker_token_b.key,
        taker.key,
        &[],
        escrow.receive_amount,
        decimal_b,
    )?;

    invoke(
        &ix_b,
        &[
            taker.clone(),
            taker_token_b.clone(),
            maker_token_b.clone(),
            mint_b.clone(),
            token_program.clone(),
        ],
    )?;

    // transfer mint_a tokens from escrow token account to taker_a account
    let (expected_pda, bump) = Pubkey::find_program_address(
        &[
            b"escrow",
            escrow.maker.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
        ],
        program_id,
    );

    if expected_pda != *escrow_account.key {
        msg!("Invalid Escrow PDA");
        return Err(ProgramError::InvalidSeeds);
    }
    // calculate decimals
    let mint_a_data = &mint_a.try_borrow_data()?; // read raw bytes of mint account
    let mint = Mint::unpack(mint_a_data)?; // deserialize bytes into Mint struct
    let decimal_a = mint.decimals;

    let ix_a = transfer_checked(
        token_program.key,
        escrow_token_a.key,
        mint_a.key,
        taker_token_a.key,
        escrow_account.key,
        &[],
        amount,
        decimal_a,
    )?;

    invoke_signed(
        &ix_a,
        &[
            escrow_account.clone(),
            escrow_token_a.clone(),
            taker_token_a.clone(),
            mint_a.clone(),
            token_program.clone(),
        ],
        &[&[
            b"escrow",
            escrow.maker.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
            &[escrow.bump],
        ]],
    )?;

    msg!("Escrow taken successfully: exchanged {} tokens", amount);
    Ok(())
}
