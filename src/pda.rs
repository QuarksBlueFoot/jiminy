//! PDA derivation utilities.
//!
//! Macros and helpers for deriving program addresses without manual
//! seed-array construction. Wraps `pinocchio_pubkey::derive_address`,
//! `pinocchio_pubkey::derive_address_const`, and `Address::find_program_address`.
//!
//! # On-chain vs off-chain
//!
//! - [`find_pda!`] uses the `find_program_address` syscall to search for the
//!   canonical bump. Only available on-chain (`target_os = "solana"`).
//! - [`derive_pda!`] uses `pinocchio_pubkey::derive_address` (sha256 syscall,
//!   no curve check). Way cheaper (~100 CU vs ~1500 CU) when the bump is known.
//! - [`derive_pda_const!`] is the compile-time version for `const`/`static` contexts.
//! - [`derive_ata`] / [`derive_ata_with_program`] derive Associated Token Account
//!   addresses using pinocchio_pubkey's fast derivation.

use pinocchio::{error::ProgramError, Address};

/// Derive the associated token account (ATA) address for a wallet + mint pair.
///
/// Uses the standard ATA derivation seeds:
/// `[wallet, token_program_id, mint]` against the ATA program.
///
/// Returns the derived address and bump. Uses `find_program_address`
/// under the hood so you get the canonical bump.
///
/// ```rust,ignore
/// let (ata_address, bump) = derive_ata(wallet.address(), mint.address())?;
/// check_pda(user_token_account, &ata_address)?;
/// ```
#[inline(always)]
pub fn derive_ata(
    wallet: &Address,
    mint: &Address,
) -> Result<(Address, u8), ProgramError> {
    derive_ata_with_program(wallet, mint, &crate::programs::TOKEN)
}

/// Derive an ATA address with an explicit token program (SPL Token or Token-2022).
///
/// Same as [`derive_ata`] but lets you specify which token program the
/// mint belongs to. Use this for Token-2022 mints.
///
/// ```rust,ignore
/// let (ata, bump) = derive_ata_with_program(
///     wallet.address(),
///     mint.address(),
///     &programs::TOKEN_2022,
/// )?;
/// ```
#[inline(always)]
pub fn derive_ata_with_program(
    wallet: &Address,
    mint: &Address,
    token_program: &Address,
) -> Result<(Address, u8), ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let seeds: &[&[u8]] = &[
            wallet.as_ref(),
            token_program.as_ref(),
            mint.as_ref(),
        ];
        let (address, bump) = Address::find_program_address(seeds, &crate::programs::ASSOCIATED_TOKEN);
        Ok((address, bump))
    }
    #[cfg(not(target_os = "solana"))]
    {
        let _ = (wallet, mint, token_program);
        Err(ProgramError::InvalidSeeds)
    }
}

/// Derive an ATA address with a known bump. Skips the bump search.
///
/// Uses `pinocchio_pubkey::derive_address` which is ~1500 CU cheaper
/// than `find_program_address`. Only use when you already know the bump.
///
/// ```rust,ignore
/// let ata = derive_ata_with_bump(wallet.address(), mint.address(), bump);
/// check_pda(token_account, &ata)?;
/// ```
#[inline(always)]
pub fn derive_ata_with_bump(
    wallet: &Address,
    mint: &Address,
    bump: u8,
) -> Address {
    Address::new_from_array(pinocchio_pubkey::derive_address(
        &[wallet.as_ref(), crate::programs::TOKEN.as_array().as_ref(), mint.as_ref()],
        Some(bump),
        crate::programs::ASSOCIATED_TOKEN.as_array(),
    ))
}

/// Derive an ATA address at compile time. Requires known bump.
///
/// ```rust,ignore
/// const MY_ATA: Address = derive_ata_const!(
///     WALLET_BYTES,
///     MINT_BYTES,
///     BUMP,
/// );
/// ```
#[macro_export]
macro_rules! derive_ata_const {
    ($wallet:expr, $mint:expr, $bump:expr) => {{
        const TOKEN_BYTES: [u8; 32] = $crate::programs::TOKEN.to_bytes();
        const ATA_BYTES: [u8; 32] = $crate::programs::ASSOCIATED_TOKEN.to_bytes();
        ::pinocchio::Address::new_from_array(::pinocchio_pubkey::derive_address_const(
            &[&$wallet, &TOKEN_BYTES, &$mint],
            Some($bump),
            &ATA_BYTES,
        ))
    }};
}

// ---- Macros ----------------------------------------------------------------

/// Find a PDA and return `(Address, u8)` with the canonical bump.
///
/// Uses the `find_program_address` syscall. Only available on-chain.
///
/// ```rust,ignore
/// let (pda, bump) = find_pda!(program_id, b"vault", authority.as_ref());
/// ```
#[macro_export]
macro_rules! find_pda {
    ($program_id:expr, $($seed:expr),+ $(,)?) => {{
        #[cfg(target_os = "solana")]
        {
            let seeds: &[&[u8]] = &[$($seed.as_ref()),+];
            ::pinocchio::Address::find_program_address(seeds, $program_id)
        }
        #[cfg(not(target_os = "solana"))]
        {
            let _ = ($program_id, $($seed),+);
            unreachable!("find_pda! is only available on target solana")
        }
    }};
}

/// Derive a PDA with a known bump. Cheap (~100 CU, no curve check).
///
/// Wraps `pinocchio_pubkey::derive_address`. The bump is appended
/// automatically. Returns `Address`.
///
/// ```rust,ignore
/// let pda = derive_pda!(program_id, bump, b"vault", authority.as_ref());
/// check_pda(vault_account, &pda)?;
/// ```
#[macro_export]
macro_rules! derive_pda {
    ($program_id:expr, $bump:expr, $($seed:expr),+ $(,)?) => {{
        ::pinocchio::Address::new_from_array(::pinocchio_pubkey::derive_address(
            &[$($seed.as_ref()),+],
            Some($bump),
            ($program_id).as_array(),
        ))
    }};
}

/// Derive a PDA at compile time. Requires `const` seeds and bump.
///
/// Uses `pinocchio_pubkey::derive_address_const` (pure-Rust SHA-256, no
/// syscall cost at runtime). The result is a `const Address`.
///
/// ```rust,ignore
/// const VAULT_PDA: Address = derive_pda_const!(
///     PROGRAM_ID_BYTES,
///     BUMP,
///     b"vault",
///     AUTHORITY_BYTES,
/// );
/// ```
#[macro_export]
macro_rules! derive_pda_const {
    ($program_id:expr, $bump:expr, $($seed:expr),+ $(,)?) => {
        ::pinocchio::Address::new_from_array(::pinocchio_pubkey::derive_address_const(
            &[$(&$seed),+],
            Some($bump),
            &$program_id,
        ))
    };
}
