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
    let system = accs.next_system_program()?;

    check_uninitialized(vault)?;

    // Read authority from instruction data — cursor is past the tag byte.
    let mut args = SliceCursor::new(&ix.data_from_position());
    let authority = args.read_address()?;

    // CPI: create the vault account.
    let lamports = rent_exempt_min(VAULT_LEN);
    create_account(payer, vault, system, program_id, lamports, VAULT_LEN as u64)?;

    // Initialize the vault data.
    let mut raw = vault.try_borrow_mut()?;
    zero_init(&mut raw);
    write_header(&mut raw, VAULT_DISC, VAULT_VERSION, 0)?;
    let mut w = DataWriter::new(header_payload_mut(&mut raw));
    w.write_u64(0)?; // balance
    w.write_address(&authority)?;

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

    let mut args = SliceCursor::new(&ix.data_from_position());
    let amount = args.read_u64()?;
    require!(amount > 0, ProgramError::InvalidArgument);

    // Transfer lamports from depositor to vault.
    let new_depositor_lamports = checked_sub(depositor.lamports(), amount)?;
    let new_vault_lamports = checked_add(vault.lamports(), amount)?;
    depositor.set_lamports(new_depositor_lamports);
    vault.set_lamports(new_vault_lamports);

    // Update stored balance.
    let mut raw = vault.try_borrow_mut()?;
    check_header(&raw, VAULT_DISC, VAULT_VERSION)?;
    let payload = header_payload_mut(&mut raw);
    let mut cur = SliceCursor::new(payload);
    let old_balance = cur.read_u64()?;
    let new_balance = checked_add(old_balance, amount)?;
    let mut w = DataWriter::new(payload);
    w.write_u64(new_balance)?;

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

    let mut args = SliceCursor::new(&ix.data_from_position());
    let amount = args.read_u64()?;
    require!(amount > 0, ProgramError::InvalidArgument);

    // Validate authority.
    {
        let data = vault.try_borrow()?;
        check_header(&data, VAULT_DISC, VAULT_VERSION)?;
        let payload = header_payload(&data);
        let mut cur = SliceCursor::new(payload);
        let balance = cur.read_u64()?;
        let stored_auth = cur.read_address()?;

        check_has_one(&stored_auth, authority)?;
        require_gte!(balance, amount, ProgramError::InsufficientFunds);
        check_lamports_gte(vault, amount)?;
    } // data borrow dropped

    // Transfer lamports.
    let new_vault_lamports = checked_sub(vault.lamports(), amount)?;
    let new_recipient_lamports = checked_add(recipient.lamports(), amount)?;
    vault.set_lamports(new_vault_lamports);
    recipient.set_lamports(new_recipient_lamports);

    // Update stored balance.
    let mut raw = vault.try_borrow_mut()?;
    let payload = header_payload_mut(&mut raw);
    let mut cur = SliceCursor::new(payload);
    let old_balance = cur.read_u64()?;
    let new_balance = checked_sub(old_balance, amount)?;
    let mut w = DataWriter::new(payload);
    w.write_u64(new_balance)?;

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
        check_header(&data, VAULT_DISC, VAULT_VERSION)?;
        let payload = header_payload(&data);
        let mut cur = SliceCursor::new(payload);
        let _balance = cur.read_u64()?;
        let stored_auth = cur.read_address()?;
        check_has_one(&stored_auth, authority)?;
    }

    safe_close(vault, destination)?;

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// CPI to the system program to create an account.
fn create_account(
    payer: &AccountView,
    new_account: &AccountView,
    _system_program: &AccountView,
    owner: &Address,
    lamports: u64,
    space: u64,
) -> ProgramResult {
    let ix = InstructionView {
        program_id: &jiminy::programs::SYSTEM,
        accounts: &[
            InstructionAccount::writable_signer(payer.address()),
            InstructionAccount::writable_signer(new_account.address()),
        ],
        data: &create_account_data(lamports, space, owner),
    };

    cpi::invoke(&ix, &[payer, new_account])
}

/// Build the 4 + 8 + 8 + 32 = 52 byte instruction data for CreateAccount.
fn create_account_data(lamports: u64, space: u64, owner: &Address) -> [u8; 52] {
    let mut data = [0u8; 52];
    // Instruction index 0 = CreateAccount
    data[0..4].copy_from_slice(&0u32.to_le_bytes());
    data[4..12].copy_from_slice(&lamports.to_le_bytes());
    data[12..20].copy_from_slice(&space.to_le_bytes());
    data[20..52].copy_from_slice(owner.as_array());
    data
}
