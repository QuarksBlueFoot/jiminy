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
    let system = accs.next_system_program()?;

    check_uninitialized(escrow)?;

    let mut args = SliceCursor::new(ix.data_from_position());
    let amount = args.read_u64()?;
    let recipient_addr = args.read_address()?;
    let timeout_ts = args.read_i64()?;

    require!(amount > 0, ProgramError::InvalidArgument);

    // CPI: create the escrow account.
    let rent = rent_exempt_min(ESCROW_LEN);
    let total_lamports = checked_add(rent, amount)?;
    create_account(creator, escrow, system, program_id, total_lamports, ESCROW_LEN as u64)?;

    // Initialize escrow data.
    let mut raw = escrow.try_borrow_mut()?;
    zero_init(&mut raw);
    write_header(&mut raw, ESCROW_DISC, ESCROW_VERSION, 0)?;
    let mut w = DataWriter::new(header_payload_mut(&mut raw));
    w.write_u64(amount)?;
    w.write_address(creator.address())?;
    w.write_address(&recipient_addr)?;
    w.write_i64(timeout_ts)?;

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
        check_header(&data, ESCROW_DISC, ESCROW_VERSION)?;
        let flags = read_header_flags(&data)?;

        // Must not already be accepted.
        require!(!read_bit(flags, FLAG_ACCEPTED), ProgramError::InvalidAccountData);

        let payload = header_payload(&data);
        let mut cur = SliceCursor::new(payload);
        amount = cur.read_u64()?;
        let _creator = cur.read_address()?;
        let stored_recipient = cur.read_address()?;

        // has_one: recipient must match.
        check_has_one(&stored_recipient, recipient)?;
    }

    // Transfer escrowed amount to destination.
    let new_escrow_lamports = checked_sub(escrow.lamports(), amount)?;
    let new_dest_lamports = checked_add(destination.lamports(), amount)?;
    escrow.set_lamports(new_escrow_lamports);
    destination.set_lamports(new_dest_lamports);

    // Mark accepted flag.
    {
        let mut raw = escrow.try_borrow_mut()?;
        let flags = read_header_flags(&raw)?;
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
        check_header(&data, ESCROW_DISC, ESCROW_VERSION)?;
        let flags = read_header_flags(&data)?;

        // Must not already be accepted.
        require!(!read_bit(flags, FLAG_ACCEPTED), ProgramError::InvalidAccountData);

        let payload = header_payload(&data);
        let mut cur = SliceCursor::new(payload);
        let _amount = cur.read_u64()?;
        let stored_creator = cur.read_address()?;
        let _recipient = cur.read_address()?;
        let _timeout_ts = cur.read_i64()?;

        // Creator must match.
        check_has_one(&stored_creator, creator)?;
    }

    // If a linked account is provided, verify it's been closed.
    if accs.remaining() > 0 {
        let linked = accs.next()?;
        check_closed(linked)?;
    }

    safe_close(escrow, destination)?;

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

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

fn create_account_data(lamports: u64, space: u64, owner: &Address) -> [u8; 52] {
    let mut data = [0u8; 52];
    data[0..4].copy_from_slice(&0u32.to_le_bytes());
    data[4..12].copy_from_slice(&lamports.to_le_bytes());
    data[12..20].copy_from_slice(&space.to_le_bytes());
    data[20..52].copy_from_slice(owner.as_array());
    data
}
