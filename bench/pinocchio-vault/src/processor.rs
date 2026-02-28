use pinocchio::{
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    AccountView, Address, ProgramResult,
};

const VAULT_DISC: u8 = 1;
const VAULT_LEN: usize = 41; // 1 disc + 8 balance + 32 authority
const SYSTEM_PROGRAM_ID: Address = Address::new_from_array([0u8; 32]);

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }

    match instruction_data[0] {
        0 => process_init_vault(program_id, accounts, &instruction_data[1..]),
        1 => process_deposit(program_id, accounts, &instruction_data[1..]),
        2 => process_withdraw(program_id, accounts, &instruction_data[1..]),
        3 => process_close_vault(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn process_init_vault(
    program_id: &Address,
    accounts: &[AccountView],
    args: &[u8],
) -> ProgramResult {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let payer = &accounts[0];
    let vault = &accounts[1];
    let system = &accounts[2];

    if !payer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !payer.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if !vault.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if *system.address() != SYSTEM_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !vault.is_data_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    if args.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let authority: [u8; 32] = args[0..32].try_into().unwrap();

    // CPI: create account.
    let lamports = rent_exempt_min(VAULT_LEN);
    let mut ix_data = [0u8; 52];
    ix_data[0..4].copy_from_slice(&0u32.to_le_bytes());
    ix_data[4..12].copy_from_slice(&lamports.to_le_bytes());
    ix_data[12..20].copy_from_slice(&(VAULT_LEN as u64).to_le_bytes());
    ix_data[20..52].copy_from_slice(program_id.as_array());

    let ix = InstructionView {
        program_id: &SYSTEM_PROGRAM_ID,
        accounts: &[
            InstructionAccount::writable_signer(payer.address()),
            InstructionAccount::writable_signer(vault.address()),
        ],
        data: &ix_data,
    };
    pinocchio::cpi::invoke(&ix, &[payer, vault])?;

    // Write vault data.
    let mut raw = vault.try_borrow_mut()?;
    raw.fill(0);
    raw[0] = VAULT_DISC;
    raw[1..9].copy_from_slice(&0u64.to_le_bytes()); // balance
    raw[9..41].copy_from_slice(&authority);

    Ok(())
}

fn process_deposit(
    program_id: &Address,
    accounts: &[AccountView],
    args: &[u8],
) -> ProgramResult {
    if accounts.len() < 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let depositor = &accounts[0];
    let vault = &accounts[1];

    if !depositor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !depositor.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if !vault.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if !vault.owned_by(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }

    {
        let data = vault.try_borrow()?;
        if data.len() < VAULT_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[0] != VAULT_DISC {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    if args.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(args[0..8].try_into().unwrap());
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    // Transfer lamports.
    let new_depositor = depositor
        .lamports()
        .checked_sub(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    let new_vault = vault
        .lamports()
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    depositor.set_lamports(new_depositor);
    vault.set_lamports(new_vault);

    // Update balance.
    let mut raw = vault.try_borrow_mut()?;
    let old_balance = u64::from_le_bytes(raw[1..9].try_into().unwrap());
    let new_balance = old_balance
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    raw[1..9].copy_from_slice(&new_balance.to_le_bytes());

    Ok(())
}

fn process_withdraw(
    program_id: &Address,
    accounts: &[AccountView],
    args: &[u8],
) -> ProgramResult {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let authority = &accounts[0];
    let vault = &accounts[1];
    let recipient = &accounts[2];

    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !vault.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if !vault.owned_by(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !recipient.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if vault.address() == recipient.address() {
        return Err(ProgramError::InvalidArgument);
    }

    if args.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(args[0..8].try_into().unwrap());
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }

    {
        let data = vault.try_borrow()?;
        if data.len() < VAULT_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[0] != VAULT_DISC {
            return Err(ProgramError::InvalidAccountData);
        }

        let balance = u64::from_le_bytes(data[1..9].try_into().unwrap());
        let stored_auth: [u8; 32] = data[9..41].try_into().unwrap();

        if stored_auth != *authority.address().as_array() {
            return Err(ProgramError::InvalidArgument);
        }
        if balance < amount {
            return Err(ProgramError::InsufficientFunds);
        }
        if vault.lamports() < amount {
            return Err(ProgramError::InsufficientFunds);
        }
    }

    let new_vault = vault
        .lamports()
        .checked_sub(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    let new_recipient = recipient
        .lamports()
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    vault.set_lamports(new_vault);
    recipient.set_lamports(new_recipient);

    let mut raw = vault.try_borrow_mut()?;
    let old_balance = u64::from_le_bytes(raw[1..9].try_into().unwrap());
    let new_balance = old_balance
        .checked_sub(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    raw[1..9].copy_from_slice(&new_balance.to_le_bytes());

    Ok(())
}

fn process_close_vault(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let authority = &accounts[0];
    let vault = &accounts[1];
    let destination = &accounts[2];

    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !vault.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if !vault.owned_by(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !destination.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if vault.address() == destination.address() {
        return Err(ProgramError::InvalidArgument);
    }

    {
        let data = vault.try_borrow()?;
        if data.len() < VAULT_LEN {
            return Err(ProgramError::AccountDataTooSmall);
        }
        if data[0] != VAULT_DISC {
            return Err(ProgramError::InvalidAccountData);
        }
        let stored_auth: [u8; 32] = data[9..41].try_into().unwrap();
        if stored_auth != *authority.address().as_array() {
            return Err(ProgramError::InvalidArgument);
        }
    }

    let lamports = vault.lamports();
    let new_dest = destination
        .lamports()
        .checked_add(lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    destination.set_lamports(new_dest);
    vault.set_lamports(0);
    unsafe { vault.close_unchecked() };

    Ok(())
}

#[inline(always)]
fn rent_exempt_min(data_len: usize) -> u64 {
    (128u64 + data_len as u64).saturating_mul(6960)
}
