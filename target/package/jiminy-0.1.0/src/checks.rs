use pinocchio::{error::ProgramError, AccountView, Address, ProgramResult};

/// The canonical system program address (all-zero pubkey).
const SYSTEM_PROGRAM_ID: Address = Address::new_from_array([0u8; 32]);

/// Verify the account signed the transaction.
#[inline(always)]
pub fn check_signer(account: &AccountView) -> ProgramResult {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

/// Verify the account is marked writable in the transaction.
#[inline(always)]
pub fn check_writable(account: &AccountView) -> ProgramResult {
    if !account.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify the account is owned by `program_id`.
#[inline(always)]
pub fn check_owner(account: &AccountView, program_id: &Address) -> ProgramResult {
    if !account.owned_by(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify the account's address equals the expected PDA.
#[inline(always)]
pub fn check_pda(account: &AccountView, expected: &Address) -> ProgramResult {
    if *account.address() != *expected {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(())
}

/// Verify the account is the canonical system program.
#[inline(always)]
pub fn check_system_program(account: &AccountView) -> ProgramResult {
    if *account.address() != SYSTEM_PROGRAM_ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify the account has no data (uninitialized). Prevents reinitialization attacks.
#[inline(always)]
pub fn check_uninitialized(account: &AccountView) -> ProgramResult {
    if !account.is_data_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    Ok(())
}

/// Verify account data is at least `min_len` bytes.
#[inline(always)]
pub fn check_size(data: &[u8], min_len: usize) -> ProgramResult {
    if data.len() < min_len {
        return Err(ProgramError::AccountDataTooSmall);
    }
    Ok(())
}

/// Verify the first byte of account data matches the expected discriminator.
#[inline(always)]
pub fn check_discriminator(data: &[u8], expected: u8) -> ProgramResult {
    if data.is_empty() || data[0] != expected {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

/// Combined check: ownership + minimum size + discriminator.
///
/// Borrows account data internally; drops the borrow before returning so
/// the caller can re-borrow to read or mutate fields.
#[inline(always)]
pub fn check_account(
    account: &AccountView,
    program_id: &Address,
    discriminator: u8,
    min_len: usize,
) -> ProgramResult {
    check_owner(account, program_id)?;
    let data = account.try_borrow()?;
    check_size(&data, min_len)?;
    check_discriminator(&data, discriminator)?;
    Ok(()) // `data` drops here, releasing the borrow
}

/// Verify two addresses are equal.
#[inline(always)]
pub fn check_keys_eq(a: &Address, b: &Address) -> ProgramResult {
    if *a != *b {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

/// Verify the account is an executable program.
///
/// Use this when an instruction receives a program account as a parameter
/// (e.g. for a CPI target) and you want to confirm it actually is one.
/// Anchor has an `executable` constraint; this is the zero-copy equivalent.
#[inline(always)]
pub fn check_executable(account: &AccountView) -> ProgramResult {
    if !account.executable() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Verify `account` holds at least `min_lamports`.
///
/// Use this for collateral checks, fee escrow validation, or confirming
/// an account is above rent-exemption before transferring into it.
///
/// Anchor has no built-in constraint for this — you end up with inline
/// comparisons scattered through handler code. Here it's one line.
#[inline(always)]
pub fn check_lamports_gte(account: &AccountView, min_lamports: u64) -> ProgramResult {
    if account.lamports() < min_lamports {
        return Err(ProgramError::InsufficientFunds);
    }
    Ok(())
}

/// Verify an account is fully closed: zero lamports and empty data.
///
/// Useful in CPI-heavy programs where you need to confirm a previous
/// instruction already closed an account before you reuse its address
/// or proceed to a next step that assumes it's gone.
#[inline(always)]
pub fn check_closed(account: &AccountView) -> ProgramResult {
    if account.lamports() != 0 || !account.is_data_empty() {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
