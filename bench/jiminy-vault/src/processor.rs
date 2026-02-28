use jiminy::prelude::*;

/// Same layout as bench-pinocchio-vault for fair comparison.
const VAULT_DISC: u8 = 1;
const VAULT_LEN: usize = 41; // 1 disc + 8 balance + 32 authority

pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut ix = SliceCursor::new(instruction_data);
    let tag = ix.read_u8()?;

    match tag {
        0 => process_init_vault(program_id, accounts, &ix),
        1 => process_deposit(program_id, accounts, &ix),
        2 => process_withdraw(program_id, accounts, &ix),
        3 => process_close_vault(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

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

    let mut args = SliceCursor::new(ix.data_from_position());
    let authority = args.read_address()?;

    let lamports = rent_exempt_min(VAULT_LEN);
    create_account(payer, vault, program_id, lamports, VAULT_LEN as u64)?;

    let mut raw = vault.try_borrow_mut()?;
    zero_init(&mut raw);
    write_discriminator(&mut raw, VAULT_DISC)?;
    let mut w = DataWriter::new(&mut raw[1..]);
    w.write_u64(0)?;
    w.write_address(&authority)?;

    Ok(())
}

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

    let new_depositor = checked_sub(depositor.lamports(), amount)?;
    let new_vault = checked_add(vault.lamports(), amount)?;
    depositor.set_lamports(new_depositor);
    vault.set_lamports(new_vault);

    let mut raw = vault.try_borrow_mut()?;
    let old_balance = u64::from_le_bytes(raw[1..9].try_into().unwrap());
    let new_balance = checked_add(old_balance, amount)?;
    raw[1..9].copy_from_slice(&new_balance.to_le_bytes());

    Ok(())
}

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

    {
        let data = vault.try_borrow()?;
        let mut cur = SliceCursor::new(&data[1..]);
        let balance = cur.read_u64()?;
        let stored_auth = cur.read_address()?;

        check_has_one(&stored_auth, authority)?;
        require_gte!(balance, amount, ProgramError::InsufficientFunds);
        check_lamports_gte(vault, amount)?;
    }

    let new_vault = checked_sub(vault.lamports(), amount)?;
    let new_recipient = checked_add(recipient.lamports(), amount)?;
    vault.set_lamports(new_vault);
    recipient.set_lamports(new_recipient);

    let mut raw = vault.try_borrow_mut()?;
    let old_balance = u64::from_le_bytes(raw[1..9].try_into().unwrap());
    let new_balance = checked_sub(old_balance, amount)?;
    raw[1..9].copy_from_slice(&new_balance.to_le_bytes());

    Ok(())
}

fn process_close_vault(
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
        let mut cur = SliceCursor::new(&data[1..]);
        let _balance = cur.read_u64()?;
        let stored_auth = cur.read_address()?;
        check_has_one(&stored_auth, authority)?;
    }

    safe_close(vault, destination)?;
    Ok(())
}

fn create_account(
    payer: &AccountView,
    new_account: &AccountView,
    owner: &Address,
    lamports: u64,
    space: u64,
) -> ProgramResult {
    let mut ix_data = [0u8; 52];
    ix_data[0..4].copy_from_slice(&0u32.to_le_bytes());
    ix_data[4..12].copy_from_slice(&lamports.to_le_bytes());
    ix_data[12..20].copy_from_slice(&space.to_le_bytes());
    ix_data[20..52].copy_from_slice(owner.as_array());

    let system = jiminy::programs::SYSTEM;
    let ix = InstructionView {
        program_id: &system,
        accounts: &[
            InstructionAccount::writable_signer(payer.address()),
            InstructionAccount::writable_signer(new_account.address()),
        ],
        data: &ix_data,
    };
    cpi::invoke(&ix, &[payer, new_account])
}
