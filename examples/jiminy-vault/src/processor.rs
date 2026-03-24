use jiminy::prelude::*;

use crate::state::*;

/// Instruction discriminators (byte 0 of instruction_data).
const IX_INIT_VAULT: u8 = 0;
const IX_DEPOSIT: u8 = 1;
const IX_WITHDRAW: u8 = 2;
const IX_CLOSE_VAULT: u8 = 3;

/// Route to the correct handler based on the first byte of instruction data.
pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut ix = SliceCursor::new(instruction_data);
    let tag = ix.read_u8()?;

    match tag {
        IX_INIT_VAULT => process_init_vault(program_id, accounts, &ix),
        IX_DEPOSIT => process_deposit(program_id, accounts, &ix),
        IX_WITHDRAW => process_withdraw(program_id, accounts, &ix),
        IX_CLOSE_VAULT => process_close_vault(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// ── InitVault ────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer, writable] payer
//   1. [writable]          vault (uninitialized)
//   2. []                  system_program
//
// Data (after tag byte):
//   [0..32] authority pubkey

fn process_init_vault(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let payer = accs.next_writable_signer()?;
    let vault = accs.next_writable()?;
    let _system = accs.next_system_program()?;

    check_uninitialized(vault)?;

    // Read authority from instruction data - cursor is past the tag byte.
    let mut args = SliceCursor::new(ix.data_from_position());
    let authority = args.read_address()?;

    // Create and initialize the vault account.
    init_account!(payer, vault, program_id, Vault)?;
    let mut raw = vault.try_borrow_mut()?;
    let v = Vault::overlay_mut(&mut raw)?;
    v.balance = LeU64::from(0u64);
    v.authority = authority;

    Ok(())
}

// ── Deposit ──────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer, writable] depositor
//   1. [writable]          vault
//
// Data (after tag byte):
//   [0..8] u64 amount

fn process_deposit(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let depositor = accs.next_writable_signer()?;
    let vault = accs.next_writable_account(program_id, VAULT_DISC, VAULT_LEN)?;

    let mut args = SliceCursor::new(ix.data_from_position());
    let amount = args.read_u64()?;
    require!(amount > 0, ProgramError::InvalidArgument);

    // Transfer lamports from depositor to vault.
    let new_depositor_lamports = checked_sub(depositor.lamports(), amount)?;
    let new_vault_lamports = checked_add(vault.lamports(), amount)?;
    depositor.set_lamports(new_depositor_lamports);
    vault.set_lamports(new_vault_lamports);

    // Update stored balance using split_fields_mut for borrow-splitting.
    // Each FieldMut holds an independent &mut [u8] subslice - no aliasing.
    let mut raw = vault.try_borrow_mut()?;
    Vault::load_checked(&raw)?;
    let (_header, mut balance, _authority) = Vault::split_fields_mut(&mut raw)?;
    let new_balance = checked_add(balance.read_u64(), amount)?;
    balance.write_u64(new_balance);

    Ok(())
}

// ── Withdraw ─────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]          authority
//   1. [writable]        vault
//   2. [writable]        recipient
//
// Data (after tag byte):
//   [0..8] u64 amount

fn process_withdraw(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let authority = accs.next_signer()?;
    let vault = accs.next_writable_account(program_id, VAULT_DISC, VAULT_LEN)?;
    let recipient = accs.next_writable()?;

    require_accounts_ne!(vault, recipient, ProgramError::InvalidArgument);

    let mut args = SliceCursor::new(ix.data_from_position());
    let amount = args.read_u64()?;
    require!(amount > 0, ProgramError::InvalidArgument);

    // Validate authority using split_fields for read-only borrow-splitting.
    {
        let data = vault.try_borrow()?;
        Vault::load_checked(&data)?;
        let (_header, balance_field, authority_field) = Vault::split_fields(&data)?;

        check_has_one(authority_field.as_address(), authority)?;
        require_gte!(balance_field.read_u64(), amount, ProgramError::InsufficientFunds);
        check_lamports_gte(vault, amount)?;
    } // data borrow dropped

    // Transfer lamports.
    let new_vault_lamports = checked_sub(vault.lamports(), amount)?;
    let new_recipient_lamports = checked_add(recipient.lamports(), amount)?;
    vault.set_lamports(new_vault_lamports);
    recipient.set_lamports(new_recipient_lamports);

    // Update stored balance via FieldMut.
    let mut raw = vault.try_borrow_mut()?;
    let (_header, mut balance_field, _authority) = Vault::split_fields_mut(&mut raw)?;
    let new_balance = checked_sub(balance_field.read_u64(), amount)?;
    balance_field.write_u64(new_balance);

    Ok(())
}

// ── CloseVault ───────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]          authority
//   1. [writable]        vault
//   2. [writable]        destination (receives remaining lamports)

fn process_close_vault(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let authority = accs.next_signer()?;
    let vault = accs.next_writable_account(program_id, VAULT_DISC, VAULT_LEN)?;
    let destination = accs.next_writable()?;

    require_accounts_ne!(vault, destination, ProgramError::InvalidArgument);

    // Validate authority.
    {
        let data = vault.try_borrow()?;
        let v = Vault::load_checked(&data)?;
        check_has_one(&v.authority, authority)?;
    }

    safe_close(vault, destination)?;

    Ok(())
}
