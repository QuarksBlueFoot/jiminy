use jiminy::prelude::*;
use crate::state::VaultView;

/// Hard-coded Program A address for this example.
/// In production, this would be a known const or passed as instruction data.
const PROGRAM_A_ID: Address = Address::new_from_array([
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
    0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20,
]);

/// Process Program B instructions.
///
/// - `0`: ReadVaultBalance: cross-program read from a Program A vault.
pub fn process(
    _program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.first() {
        Some(0) => read_vault_balance(accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

/// Read the balance of a Program A vault account.
///
/// This is the cross-program read pattern:
/// 1. `VaultView::load_foreign` validates `owner == PROGRAM_A_ID` and
///    `layout_id` matches, proving ABI compatibility.
/// 2. `VaultView::overlay` returns a typed `&VaultView` directly over
///    the borrowed bytes. Zero deserialization.
/// 3. Program B never imported Program A's crate. The only contract
///    is the byte layout.
fn read_vault_balance(accounts: &[AccountView]) -> ProgramResult {
    let mut accs = AccountList::new(accounts);
    let vault_account = accs.next()?;

    // Tier 2: Cross-program read.
    // Validates: owner == PROGRAM_A_ID, layout_id matches, size >= VaultView::LEN.
    let data = VaultView::load_foreign(vault_account, &PROGRAM_A_ID)?;
    let vault = VaultView::overlay(&data)?;

    // Use the foreign vault's data - zero copies, zero deserialization.
    // In a real program, you'd use vault.balance for further logic.
    let _balance = vault.balance;

    Ok(())
}
