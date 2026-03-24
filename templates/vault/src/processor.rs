use jiminy::prelude::*;

use crate::state::*;

// ── Instruction tags ─────────────────────────────────────────────────────────

const IX_INIT: u8 = 0;
const IX_DEPOSIT: u8 = 1;
const IX_WITHDRAW: u8 = 2;
const IX_CLOSE: u8 = 3;

// ── Dispatch ─────────────────────────────────────────────────────────────────

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut ix = SliceCursor::new(instruction_data);
    let tag = ix.read_u8()?;

    match tag {
        IX_INIT => init(program_id, accounts, &ix),
        IX_DEPOSIT => deposit(program_id, accounts, &ix),
        IX_WITHDRAW => withdraw(program_id, accounts, &ix),
        IX_CLOSE => close(program_id, accounts),
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
// Data: authority [u8; 32]

fn init(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let payer = accs.next_writable_signer()?;
    let vault = accs.next_writable()?;
    let _system = accs.next_system_program()?;

    check_uninitialized(vault)?;

    let mut args = SliceCursor::new(ix.data_from_position());
    let authority = args.read_address()?;

    // Create the account and write the Jiminy header.
    init_account!(payer, vault, program_id, Vault)?;

    // Write fields.
    let mut raw = vault.try_borrow_mut()?;
    let v = Vault::overlay_mut(&mut raw)?;
    v.balance = 0;
    v.authority = authority;

    Ok(())
}

// ── Deposit ──────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer, writable] depositor
//   1. [writable]          vault
//
// Data: amount u64

fn deposit(
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

    // Transfer lamports.
    let new_depositor = checked_sub(depositor.lamports(), amount)?;
    let new_vault = checked_add(vault.lamports(), amount)?;
    depositor.set_lamports(new_depositor);
    vault.set_lamports(new_vault);

    // Update stored balance.
    let mut raw = vault.try_borrow_mut()?;
    let v = Vault::load_checked_mut(&mut raw)?;
    v.balance = checked_add(v.balance, amount)?;

    Ok(())
}

// ── Withdraw ─────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]   authority
//   1. [writable] vault
//   2. [writable] recipient
//
// Data: amount u64

fn withdraw(
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

    // Validate authority and balance.
    {
        let data = vault.try_borrow()?;
        let v = Vault::load_checked(&data)?;
        check_has_one(&v.authority, authority)?;
        require_gte!(v.balance, amount, ProgramError::InsufficientFunds);
        check_lamports_gte(vault, amount)?;
    }

    // Transfer lamports.
    let new_vault = checked_sub(vault.lamports(), amount)?;
    let new_recipient = checked_add(recipient.lamports(), amount)?;
    vault.set_lamports(new_vault);
    recipient.set_lamports(new_recipient);

    // Update stored balance.
    let mut raw = vault.try_borrow_mut()?;
    let v = Vault::load_checked_mut(&mut raw)?;
    v.balance = checked_sub(v.balance, amount)?;

    Ok(())
}

// ── CloseVault ───────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]   authority
//   1. [writable] vault
//   2. [writable] destination

fn close(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let authority = accs.next_signer()?;
    let vault = accs.next_writable_account(program_id, VAULT_DISC, VAULT_LEN)?;
    let destination = accs.next_writable()?;

    require_accounts_ne!(vault, destination, ProgramError::InvalidArgument);

    {
        let data = vault.try_borrow()?;
        let v = Vault::load_checked(&data)?;
        check_has_one(&v.authority, authority)?;
    }

    safe_close(vault, destination)?;

    Ok(())
}
