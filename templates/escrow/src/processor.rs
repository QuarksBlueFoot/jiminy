use jiminy::prelude::*;

use crate::state::*;

const IX_CREATE: u8 = 0;
const IX_ACCEPT: u8 = 1;
const IX_CANCEL: u8 = 2;

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut ix = SliceCursor::new(instruction_data);
    let tag = ix.read_u8()?;

    match tag {
        IX_CREATE => create(program_id, accounts, &ix),
        IX_ACCEPT => accept(program_id, accounts),
        IX_CANCEL => cancel(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// ── Create ───────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer, writable] creator
//   1. [writable]          escrow (uninitialized)
//   2. []                  system_program
//
// Data: recipient [u8;32], amount u64, deadline i64

fn create(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let creator_acc = accs.next_writable_signer()?;
    let escrow = accs.next_writable()?;
    let _system = accs.next_system_program()?;

    check_uninitialized(escrow)?;

    let mut args = SliceCursor::new(ix.data_from_position());
    let recipient = args.read_address()?;
    let amount = args.read_u64()?;
    let deadline = args.read_i64()?;

    require!(amount > 0, ProgramError::InvalidArgument);

    // Create account and write header.
    init_account!(creator_acc, escrow, program_id, Escrow)?;

    // Fund the escrow.
    let new_creator = checked_sub(creator_acc.lamports(), amount)?;
    let new_escrow = checked_add(escrow.lamports(), amount)?;
    creator_acc.set_lamports(new_creator);
    escrow.set_lamports(new_escrow);

    // Write fields.
    let mut raw = escrow.try_borrow_mut()?;
    let e = Escrow::overlay_mut(&mut raw)?;
    e.amount = amount;
    e.creator = *creator_acc.address();
    e.recipient = recipient;
    e.deadline = deadline;

    Ok(())
}

// ── Accept ───────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]   recipient
//   1. [writable] escrow
//   2. [writable] destination (receives funds)

fn accept(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let recipient = accs.next_signer()?;
    let escrow = accs.next_writable_account(program_id, ESCROW_DISC, ESCROW_LEN)?;
    let destination = accs.next_writable()?;

    let amount;
    {
        let data = escrow.try_borrow()?;
        let e = Escrow::load_checked(&data)?;

        // Only the designated recipient can accept.
        check_has_one(&e.recipient, recipient)?;

        // Must not already be accepted.
        let flags = read_header_flags(&data)?;
        require!(flags & FLAG_ACCEPTED == 0, ProgramError::InvalidAccountData);

        amount = e.amount;
    }

    // Set accepted flag.
    {
        let mut data = escrow.try_borrow_mut()?;
        let flags = read_header_flags(&data)? | FLAG_ACCEPTED;
        data[2..4].copy_from_slice(&flags.to_le_bytes());
    }

    // Transfer funds and close.
    let new_escrow = checked_sub(escrow.lamports(), amount)?;
    let new_dest = checked_add(destination.lamports(), amount)?;
    escrow.set_lamports(new_escrow);
    destination.set_lamports(new_dest);

    safe_close(escrow, destination)?;

    Ok(())
}

// ── Cancel ───────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]   creator
//   1. [writable] escrow
//   2. [writable] destination (receives remaining lamports)
//   3. []         clock (Clock sysvar)

fn cancel(
    program_id: &Address,
    accounts: &[AccountView],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let creator = accs.next_signer()?;
    let escrow = accs.next_writable_account(program_id, ESCROW_DISC, ESCROW_LEN)?;
    let destination = accs.next_writable()?;
    let clock = accs.next()?;

    {
        let data = escrow.try_borrow()?;
        let e = Escrow::load_checked(&data)?;

        // Only the creator can cancel.
        check_has_one(&e.creator, creator)?;

        // Must not already be accepted.
        let flags = read_header_flags(&data)?;
        require!(flags & FLAG_ACCEPTED == 0, ProgramError::InvalidAccountData);

        // Must be past the deadline.
        check_after(clock, e.deadline)?;
    }

    safe_close(escrow, destination)?;

    Ok(())
}
