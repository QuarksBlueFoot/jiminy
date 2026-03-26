use jiminy::prelude::*;

use crate::state::*;

const IX_INIT_POOL: u8 = 0;
const IX_STAKE: u8 = 1;
const IX_UNSTAKE: u8 = 2;

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut ix = SliceCursor::new(instruction_data);
    let tag = ix.read_u8()?;

    match tag {
        IX_INIT_POOL => init_pool(program_id, accounts, &ix),
        IX_STAKE => stake(program_id, accounts, &ix),
        IX_UNSTAKE => unstake(program_id, accounts, &ix),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// ── InitPool ─────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer, writable] payer
//   1. [writable]          pool (uninitialized)
//   2. []                  system_program
//
// Data: authority [u8;32], max_stakers u16

fn init_pool(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let payer = accs.next_writable_signer()?;
    let pool = accs.next_writable()?;
    let _system = accs.next_system_program()?;

    check_uninitialized(pool)?;

    let mut args = SliceCursor::new(ix.data_from_position());
    let authority = args.read_address()?;
    let max_stakers = args.read_u16()?;

    require!(max_stakers > 0, ProgramError::InvalidArgument);

    // Compute account size for the desired capacity.
    let account_size = StakePool::compute_account_size(&[max_stakers])?;

    // Create the account via CPI.
    safe_create_account(payer, pool, account_size, program_id)?;

    // Write header + fixed fields.
    let mut raw = pool.try_borrow_mut()?;
    zero_init(&mut raw);
    write_header(
        &mut raw,
        StakePool::DISC,
        StakePool::VERSION,
        &StakePool::SEGMENTED_LAYOUT_ID,
    )?;

    let p = StakePool::overlay_mut(&mut raw)?;
    p.authority = authority;
    p.total_staked = 0;

    // Initialize the segment table with 0 entries, capacity-ready.
    StakePool::init_segments_with_capacity(&mut raw, &[max_stakers])?;

    Ok(())
}

// ── Stake ────────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]   staker
//   1. [writable] pool
//   2. []         clock (Clock sysvar)
//
// Data: amount u64

fn stake(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let staker = accs.next_signer()?;
    let pool_acc = accs.next_writable_account(program_id, POOL_DISC, StakePool::FIXED_LEN)?;
    let clock = accs.next()?;

    let mut args = SliceCursor::new(ix.data_from_position());
    let amount = args.read_u64()?;
    require!(amount > 0, ProgramError::InvalidArgument);

    // Read the current epoch from the Clock sysvar.
    let (_, timestamp) = read_clock(clock)?;

    let mut raw = pool_acc.try_borrow_mut()?;

    // Read current segment descriptor for stakes.
    let table = StakePool::segment_table(&raw)?;
    let desc = table.descriptor(0)?;
    let current_count = desc.count();
    let entry_offset = desc.offset() as usize + (current_count as usize) * StakeEntry::SIZE;

    // Write the new stake entry at the end of the segment.
    let new_entry = StakeEntry {
        staker: *staker.key(),
        amount,
        start_epoch: timestamp as u64,
    };

    // SAFETY: StakeEntry is #[repr(C)] and Copy with only plain data fields
    // (Address, u64, u64). We convert it to a byte slice of exactly SIZE bytes,
    // which is safe because the struct has no padding requirements beyond repr(C).
    let entry_bytes = unsafe {
        core::slice::from_raw_parts(
            &new_entry as *const StakeEntry as *const u8,
            StakeEntry::SIZE,
        )
    };
    raw[entry_offset..entry_offset + StakeEntry::SIZE].copy_from_slice(entry_bytes);

    // Increment the element count in the descriptor.
    let updated = SegmentDescriptor::new(
        desc.offset(),
        current_count + 1,
        desc.capacity(),
        desc.element_size(),
    );
    let mut table_mut = StakePool::segment_table_mut(&mut raw)?;
    table_mut.set_descriptor(0, &updated)?;

    // Update total_staked.
    let pool = StakePool::overlay_mut(&mut raw)?;
    pool.total_staked = checked_add(pool.total_staked, amount)?;

    Ok(())
}

// ── Unstake ──────────────────────────────────────────────────────────────────
//
// Accounts:
//   0. [signer]   staker
//   1. [writable] pool
//
// Data: index u16

fn unstake(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &SliceCursor,
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let staker = accs.next_signer()?;
    let pool_acc = accs.next_writable_account(program_id, POOL_DISC, StakePool::FIXED_LEN)?;

    let mut args = SliceCursor::new(ix.data_from_position());
    let index = args.read_u16()? as usize;

    let mut raw = pool_acc.try_borrow_mut()?;

    // Read the stakes segment descriptor.
    let table = StakePool::segment_table(&raw)?;
    let desc = table.descriptor(0)?;
    let count = desc.count() as usize;

    require!(index < count, ProgramError::InvalidArgument);

    // Read the entry to verify staker ownership.
    let entry_offset = desc.offset() as usize + index * StakeEntry::SIZE;
    // SAFETY: StakeEntry is #[repr(C)], Copy, align(1) with only plain data fields.
    // Bounds are guaranteed: index < count (checked above), and the segment
    // descriptor offset + count * SIZE fits within the account data.
    let entry = unsafe {
        &*(raw[entry_offset..].as_ptr() as *const StakeEntry)
    };
    require_keys_eq!(entry.staker, *staker.key(), ProgramError::IllegalOwner);

    let amount = entry.amount;

    // Swap-remove: copy last entry into this slot, then zero the tail.
    if index < count - 1 {
        let last_offset = desc.offset() as usize + (count - 1) * StakeEntry::SIZE;
        let mut buf = [0u8; 48];
        buf.copy_from_slice(&raw[last_offset..last_offset + StakeEntry::SIZE]);
        raw[entry_offset..entry_offset + StakeEntry::SIZE].copy_from_slice(&buf);
    }

    // Zero the now-unused last slot.
    let last_offset = desc.offset() as usize + (count - 1) * StakeEntry::SIZE;
    for b in &mut raw[last_offset..last_offset + StakeEntry::SIZE] {
        *b = 0;
    }

    // Decrement the element count in the descriptor. Capacity stays the same.
    let updated = SegmentDescriptor::new(
        desc.offset(),
        (count - 1) as u16,
        desc.capacity(),
        desc.element_size(),
    );
    let mut table_mut = StakePool::segment_table_mut(&mut raw)?;
    table_mut.set_descriptor(0, &updated)?;

    // Update total_staked.
    let pool = StakePool::overlay_mut(&mut raw)?;
    pool.total_staked = checked_sub(pool.total_staked, amount)?;

    Ok(())
}
