use jiminy::prelude::*;

use crate::state::*;

const IX_CREATE_ESCROW: u8 = 0;
const IX_ACCEPT_ESCROW: u8 = 1;
const IX_CANCEL_ESCROW: u8 = 2;

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut ix = SliceCursor::new(instruction_data);
    let tag = ix.read_u8()?;

    match tag {
        IX_CREATE_ESCROW => process_create_escrow(program_id, accounts, &ix),
        IX_ACCEPT_ESCROW => process_accept_escrow(program_id, accounts),
        IX_CANCEL_ESCROW => process_cancel_escrow(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// ── CreateEscrow ─────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer, writable] creator
//   1. [writable]          escrow (uninitialized)
//   2. []                  system_program
//
// Data (after tag byte):
//   [0..8]   u64     amount
//   [8..40]  Address recipient
//   [40..48] i64     timeout_ts (0 = no timeout)

fn process_create_escrow(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let creator = accs.next_writable_signer()?;
    let escrow = accs.next_writable()?;
    let _system = accs.next_system_program()?;

    check_uninitialized(escrow)?;

    let mut args = SliceCursor::new(ix.data_from_position());
    let amount = args.read_u64()?;
    let recipient_addr = args.read_address()?;
    let timeout_ts = args.read_i64()?;

    require!(amount > 0, ProgramError::InvalidArgument);

    // CPI: create the escrow account with rent + escrowed amount.
    let rent = rent_exempt_min(ESCROW_LEN);
    let total_lamports = checked_add(rent, amount)?;
    CreateAccount {
        from: creator,
        to: escrow,
        lamports: total_lamports,
        space: ESCROW_LEN as u64,
        owner: program_id,
    }
    .invoke()?;

    // Initialize escrow data.
    let mut raw = escrow.try_borrow_mut()?;
    zero_init(&mut raw);
    write_header(&mut raw, ESCROW_DISC, ESCROW_VERSION, &ESCROW_LAYOUT_ID)?;
    let e = Escrow::overlay_mut(&mut raw)?;
    e.amount = amount;
    e.creator = *creator.address();
    e.recipient = recipient_addr;
    e.timeout = timeout_ts;

    Ok(())
}

// ── AcceptEscrow ─────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]          recipient
//   1. [writable]        escrow
//   2. [writable]        recipient_lamport_dest (can be same as recipient)

fn process_accept_escrow(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let recipient = accs.next_signer()?;
    let escrow = accs.next_writable_account(program_id, ESCROW_DISC, ESCROW_LEN)?;
    let destination = accs.next_writable()?;

    // Read and validate escrow.
    let amount;
    {
        let data = escrow.try_borrow()?;
        check_header(&data, ESCROW_DISC, ESCROW_VERSION, &ESCROW_LAYOUT_ID)?;
        let flags = read_header_flags(&data)? as u8;

        // Must not already be accepted.
        require!(!read_bit(flags, FLAG_ACCEPTED), ProgramError::InvalidAccountData);

        let e = Escrow::overlay(&data)?;
        amount = e.amount;

        // has_one: recipient must match.
        check_has_one(&e.recipient, recipient)?;
    }

    // Transfer escrowed amount to destination.
    let new_escrow_lamports = checked_sub(escrow.lamports(), amount)?;
    let new_dest_lamports = checked_add(destination.lamports(), amount)?;
    escrow.set_lamports(new_escrow_lamports);
    destination.set_lamports(new_dest_lamports);

    // Mark accepted flag.
    {
        let mut raw = escrow.try_borrow_mut()?;
        let flags = read_header_flags(&raw)? as u8;
        let new_flags = set_bit(flags, FLAG_ACCEPTED);
        raw[2] = new_flags;
    }

    Ok(())
}

// ── CancelEscrow ─────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]          creator
//   1. [writable]        escrow
//   2. [writable]        destination (receives remaining lamports)
//   3. []                linked_account (optional; if provided, must be closed)
//
// The creator can cancel if:
//   - The escrow has not been accepted, AND
//   - Either a timeout has passed, or the linked account (if provided) is closed.

fn process_cancel_escrow(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let creator = accs.next_signer()?;
    let escrow = accs.next_writable_account(program_id, ESCROW_DISC, ESCROW_LEN)?;
    let destination = accs.next_writable()?;

    require_accounts_ne!(escrow, destination, ProgramError::InvalidArgument);

    {
        let data = escrow.try_borrow()?;
        check_header(&data, ESCROW_DISC, ESCROW_VERSION, &ESCROW_LAYOUT_ID)?;
        let flags = read_header_flags(&data)? as u8;

        // Must not already be accepted.
        require!(!read_bit(flags, FLAG_ACCEPTED), ProgramError::InvalidAccountData);

        let e = Escrow::overlay(&data)?;

        // Creator must match.
        check_has_one(&e.creator, creator)?;
    }

    // If a linked account is provided, verify it's been closed.
    if accs.remaining() > 0 {
        let linked = accs.next()?;
        check_closed(linked)?;
    }

    safe_close(escrow, destination)?;

    Ok(())
}
