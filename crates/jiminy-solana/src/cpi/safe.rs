//! Safe CPI wrappers that bundle validation + invocation.
//!
//! These combine the most common check patterns with hopper-runtime system and
//! token CPI structs so you can't forget a writable or signer
//! check before issuing a CPI.
//!
//! All functions are `#[inline(always)]` and zero-copy.

use hopper_runtime::{ProgramError, AccountView, Address, ProgramResult};
use hopper_runtime::system::instructions::{CreateAccount, Transfer as SysTransfer};
use hopper_runtime::token::instructions::{
    Burn, CloseAccount, MintTo, Transfer as TkTransfer,
};

use jiminy_core::check::{check_signer, check_writable, rent_exempt_min};
use crate::token::{check_token_account_mint, check_token_account_owner};

/// Create an account via system program CPI with validation.
///
/// Checks:
/// - `payer` is a writable signer
/// - `new_account` is writable
/// - Computes rent-exempt lamports from `space`
///
/// ```rust,ignore
/// safe_create_account(payer, vault, VAULT_LEN, program_id)?;
/// ```
#[inline(always)]
pub fn safe_create_account(
    payer: &AccountView,
    new_account: &AccountView,
    space: usize,
    owner: &Address,
) -> ProgramResult {
    check_signer(payer)?;
    check_writable(payer)?;
    check_writable(new_account)?;

    let lamports = rent_exempt_min(space);
    CreateAccount {
        from: payer,
        to: new_account,
        lamports,
        space: space as u64,
        owner,
    }
    .invoke()
}

/// Create an account via CPI, signing with PDA seeds.
///
/// Same as [`safe_create_account`] but uses `invoke_signed` for PDA
/// accounts that need the program to sign.
///
/// ```rust,ignore
/// safe_create_account_signed(payer, pda_account, VAULT_LEN, program_id, &[&[b"vault", &[bump]]])?;
/// ```
#[inline(always)]
pub fn safe_create_account_signed(
    payer: &AccountView,
    new_account: &AccountView,
    space: usize,
    owner: &Address,
    signers: &[hopper_runtime::cpi::Signer],
) -> ProgramResult {
    check_signer(payer)?;
    check_writable(payer)?;
    check_writable(new_account)?;

    let lamports = rent_exempt_min(space);
    CreateAccount {
        from: payer,
        to: new_account,
        lamports,
        space: space as u64,
        owner,
    }
    .invoke_signed(signers)
}

/// Transfer SOL via system program CPI with validation.
///
/// Checks:
/// - `from` is a writable signer
/// - `to` is writable
/// - `amount` > 0
///
/// ```rust,ignore
/// safe_transfer_sol(payer, recipient, 1_000_000)?;
/// ```
#[inline(always)]
pub fn safe_transfer_sol(
    from: &AccountView,
    to: &AccountView,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    check_signer(from)?;
    check_writable(from)?;
    check_writable(to)?;

    SysTransfer {
        from,
        to,
        lamports: amount,
    }
    .invoke()
}

/// Transfer SPL tokens via token program CPI with validation.
///
/// Checks:
/// - `authority` is a signer
/// - `from` is writable
/// - `to` is writable
/// - `amount` > 0
///
/// ```rust,ignore
/// safe_transfer_tokens(source_ata, dest_ata, owner, 1_000_000)?;
/// ```
#[inline(always)]
pub fn safe_transfer_tokens(
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    check_signer(authority)?;
    check_writable(from)?;
    check_writable(to)?;

    TkTransfer {
        from,
        to,
        authority,
        amount,
    }
    .invoke()
}

/// Transfer SPL tokens with PDA signer seeds.
///
/// Same as [`safe_transfer_tokens`] but the authority is a PDA.
///
/// ```rust,ignore
/// safe_transfer_tokens_signed(pool_ata, user_ata, pool_authority, amount, &[&[b"pool", &[bump]]])?;
/// ```
#[inline(always)]
pub fn safe_transfer_tokens_signed(
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
    signers: &[hopper_runtime::cpi::Signer],
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    check_writable(from)?;
    check_writable(to)?;

    TkTransfer {
        from,
        to,
        authority,
        amount,
    }
    .invoke_signed(signers)
}

/// Burn SPL tokens with validation.
///
/// Checks:
/// - `authority` is a signer
/// - `account` is writable
/// - `amount` > 0
///
/// ```rust,ignore
/// safe_burn(token_account, mint, owner, burn_amount)?;
/// ```
#[inline(always)]
pub fn safe_burn(
    account: &AccountView,
    mint: &AccountView,
    authority: &AccountView,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    check_signer(authority)?;
    check_writable(account)?;
    check_writable(mint)?;

    Burn {
        account,
        mint,
        authority,
        amount,
    }
    .invoke()
}

/// Mint tokens to an account with validation.
///
/// Checks:
/// - `authority` is a signer (mint authority)
/// - `account` (destination) is writable
/// - `mint` is writable
/// - `amount` > 0
///
/// ```rust,ignore
/// safe_mint_to(mint, destination_ata, mint_authority, 1_000_000)?;
/// ```
#[inline(always)]
pub fn safe_mint_to(
    mint: &AccountView,
    account: &AccountView,
    authority: &AccountView,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    check_signer(authority)?;
    check_writable(mint)?;
    check_writable(account)?;

    MintTo {
        mint,
        account,
        mint_authority: authority,
        amount,
    }
    .invoke()
}

/// Mint tokens with PDA signer seeds.
#[inline(always)]
pub fn safe_mint_to_signed(
    mint: &AccountView,
    account: &AccountView,
    authority: &AccountView,
    amount: u64,
    signers: &[hopper_runtime::cpi::Signer],
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    check_writable(mint)?;
    check_writable(account)?;

    MintTo {
        mint,
        account,
        mint_authority: authority,
        amount,
    }
    .invoke_signed(signers)
}

/// Close a token account via token program CPI.
///
/// Checks:
/// - `authority` is a signer
/// - `account` is writable
/// - `destination` is writable
///
/// ```rust,ignore
/// safe_close_token_account(token_account, owner_wallet, owner)?;
/// ```
#[inline(always)]
pub fn safe_close_token_account(
    account: &AccountView,
    destination: &AccountView,
    authority: &AccountView,
) -> ProgramResult {
    check_signer(authority)?;
    check_writable(account)?;
    check_writable(destination)?;

    CloseAccount {
        account,
        destination,
        authority,
    }
    .invoke()
}

/// Validated token transfer: checks mint + owner before transferring.
///
/// A "paranoid" transfer that verifies source and destination token
/// accounts belong to the right mint and owners before doing the CPI.
/// Prevents cross-mint transfers and wrong-owner exploits.
///
/// ```rust,ignore
/// safe_checked_transfer(
///     source_ata, dest_ata, owner,
///     &usdc_mint, source_wallet.address(), dest_wallet.address(),
///     amount,
/// )?;
/// ```
#[inline(always)]
pub fn safe_checked_transfer(
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    expected_mint: &Address,
    expected_from_owner: &Address,
    expected_to_owner: &Address,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    check_token_account_mint(from, expected_mint)?;
    check_token_account_mint(to, expected_mint)?;
    check_token_account_owner(from, expected_from_owner)?;
    check_token_account_owner(to, expected_to_owner)?;
    check_signer(authority)?;
    check_writable(from)?;
    check_writable(to)?;

    TkTransfer {
        from,
        to,
        authority,
        amount,
    }
    .invoke()
}

/// Transfer lamports directly between two program-owned accounts.
///
/// This performs a direct lamport manipulation (no CPI to the system
/// program) which is valid when both accounts are owned by your program.
/// This is significantly cheaper than a system transfer CPI and is the
/// correct pattern for moving SOL between PDAs your program controls.
///
/// Checks:
/// - Both accounts are writable
/// - `amount` > 0
/// - `from` has sufficient lamports
/// - Addition to `to` doesn't overflow
///
/// ```rust,ignore
/// // Move 1 SOL from pool PDA to user PDA (both owned by your program)
/// transfer_lamports(pool, user_account, 1_000_000_000)?;
/// ```
#[inline(always)]
pub fn transfer_lamports(
    from: &AccountView,
    to: &AccountView,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    check_writable(from)?;
    check_writable(to)?;

    let from_lamports = from.lamports();
    if from_lamports < amount {
        return Err(ProgramError::InsufficientFunds);
    }
    let new_from = from_lamports - amount; // safe: checked above
    let new_to = jiminy_core::math::checked_add(to.lamports(), amount)?;
    from.set_lamports(new_from);
    to.set_lamports(new_to);
    Ok(())
}
