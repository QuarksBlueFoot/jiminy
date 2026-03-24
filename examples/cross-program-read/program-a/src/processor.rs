use jiminy::prelude::*;
use crate::state::Vault;

/// Process Program A instructions.
///
/// - `0`: InitVault: create a new vault.
pub fn process(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.first() {
        Some(0) => init_vault(program_id, accounts, instruction_data),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn init_vault(
    program_id: &Address,
    accounts: &[AccountView],
    ix: &[u8],
) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let payer = accs.next_writable_signer()?;
    let vault = accs.next_writable()?;
    let _system = accs.next_system_program()?;

    check_uninitialized(vault)?;

    // Read authority from instruction data (bytes 1..33).
    let mut args = SliceCursor::new(&ix[1..]);
    let authority = args.read_address()?;

    // Create and initialize the vault account.
    init_account!(payer, vault, program_id, Vault)?;
    let mut raw = vault.try_borrow_mut()?;
    let v = Vault::overlay_mut(&mut raw)?;
    v.balance = 0;
    v.authority = authority;

    Ok(())
}

