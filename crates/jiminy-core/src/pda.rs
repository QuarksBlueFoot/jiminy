//! PDA derivation utilities.
//!
//! Macros and helpers for deriving program addresses without manual
//! seed-array construction. Wraps [`derive_address`], [`derive_address_const`],
//! and `Address::find_program_address`.
//!
//! # On-chain vs off-chain
//!
//! - [`find_pda!`] uses the `find_program_address` syscall to search for the
//!   canonical bump. Only available on-chain (`target_os = "solana"`).
//! - [`derive_pda!`] uses [`derive_address`] (sha256 syscall,
//!   no curve check). Way cheaper (~100 CU vs ~1500 CU) when the bump is known.
//! - [`derive_pda_const!`] is the compile-time version for `const`/`static` contexts.
//! - [`derive_ata`] / [`derive_ata_with_program`] derive Associated Token Account
//!   addresses using fast PDA derivation.

use core::mem::MaybeUninit;

use pinocchio::{
    address::{MAX_SEEDS, PDA_MARKER},
    error::ProgramError,
    Address,
};
use sha2_const_stable::Sha256;

/// Derive a [program address](https://solana.com/docs/core/pda) from the
/// given seeds, optional bump, and program id.
///
/// This avoids the cost of the `create_program_address` syscall (~1500 CU) by
/// directly hashing seeds + bump + program_id + `"ProgramDerivedAddress"` via
/// the `sol_sha256` syscall.
///
/// # Important
///
/// **No curve-point validation** is performed. Only use when the seeds, bump,
/// and program id are known to produce a valid (off-curve) PDA.
#[inline(always)]
pub fn derive_address<const N: usize>(
    seeds: &[&[u8]; N],
    bump: Option<u8>,
    program_id: &[u8; 32],
) -> [u8; 32] {
    const {
        assert!(N < MAX_SEEDS, "number of seeds must be less than MAX_SEEDS");
    }

    const UNINIT: MaybeUninit<&[u8]> = MaybeUninit::<&[u8]>::uninit();
    let mut data = [UNINIT; MAX_SEEDS + 2];
    let mut i = 0;

    while i < N {
        unsafe {
            data.get_unchecked_mut(i).write(seeds.get_unchecked(i));
        }
        i += 1;
    }

    let bump_seed = [bump.unwrap_or_default()];

    unsafe {
        if bump.is_some() {
            data.get_unchecked_mut(i).write(&bump_seed);
            i += 1;
        }
        data.get_unchecked_mut(i).write(program_id.as_ref());
        data.get_unchecked_mut(i + 1).write(PDA_MARKER.as_ref());
    }

    #[cfg(target_os = "solana")]
    {
        let mut pda = MaybeUninit::<[u8; 32]>::uninit();

        unsafe {
            pinocchio::syscalls::sol_sha256(
                data.as_ptr() as *const u8,
                (i + 2) as u64,
                pda.as_mut_ptr() as *mut u8,
            );
        }

        unsafe { pda.assume_init() }
    }

    #[cfg(not(target_os = "solana"))]
    {
        let _ = data;
        unreachable!("deriving a pda is only available on target `solana`");
    }
}

/// Compile-time version of [`derive_address`].
///
/// Uses pure-Rust SHA-256 (`sha2-const-stable`) so the result is computed at
/// compile time with zero runtime cost. Seeds, bump, and program id must all
/// be `const`.
#[inline(always)]
pub const fn derive_address_const<const N: usize>(
    seeds: &[&[u8]; N],
    bump: Option<u8>,
    program_id: &[u8; 32],
) -> [u8; 32] {
    const {
        assert!(N < MAX_SEEDS, "number of seeds must be less than MAX_SEEDS");
    }

    let mut hasher = Sha256::new();
    let mut i = 0;

    while i < seeds.len() {
        hasher = hasher.update(seeds[i]);
        i += 1;
    }

    if let Some(bump) = bump {
        hasher
            .update(&[bump])
            .update(program_id)
            .update(PDA_MARKER)
            .finalize()
    } else {
        hasher.update(program_id).update(PDA_MARKER).finalize()
    }
}

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
#[cfg(feature = "programs")]
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
#[cfg(feature = "programs")]
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
/// Uses [`derive_address`] which is ~1500 CU cheaper
/// than `find_program_address`. Only use when you already know the bump.
///
/// ```rust,ignore
/// let ata = derive_ata_with_bump(wallet.address(), mint.address(), bump);
/// check_pda(token_account, &ata)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn derive_ata_with_bump(
    wallet: &Address,
    mint: &Address,
    bump: u8,
) -> Address {
    Address::new_from_array(derive_address(
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
#[cfg(feature = "programs")]
#[macro_export]
macro_rules! derive_ata_const {
    ($wallet:expr, $mint:expr, $bump:expr) => {{
        const TOKEN_BYTES: [u8; 32] = $crate::programs::TOKEN.to_bytes();
        const ATA_BYTES: [u8; 32] = $crate::programs::ASSOCIATED_TOKEN.to_bytes();
        ::pinocchio::Address::new_from_array($crate::pda::derive_address_const(
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
/// Wraps [`derive_address`]. The bump is appended
/// automatically. Returns `Address`.
///
/// ```rust,ignore
/// let pda = derive_pda!(program_id, bump, b"vault", authority.as_ref());
/// check_pda(vault_account, &pda)?;
/// ```
#[macro_export]
macro_rules! derive_pda {
    ($program_id:expr, $bump:expr, $($seed:expr),+ $(,)?) => {{
        ::pinocchio::Address::new_from_array($crate::pda::derive_address(
            &[$($seed.as_ref()),+],
            Some($bump),
            ($program_id).as_array(),
        ))
    }};
}

/// Derive a PDA at compile time. Requires `const` seeds and bump.
///
/// Uses [`derive_address_const`] (pure-Rust SHA-256, no
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
        ::pinocchio::Address::new_from_array($crate::pda::derive_address_const(
            &[$(&$seed),+],
            Some($bump),
            &$program_id,
        ))
    };
}

/// Verify a token account is the correct ATA for a wallet + mint pair.
///
/// Derives the canonical ATA address using `find_program_address` and
/// compares it to the account's address. Uses the SPL Token program.
///
/// This catches a critical exploit vector: an attacker passing their own
/// token account (with the right mint/owner) instead of the canonical ATA.
///
/// ```rust,ignore
/// check_ata(user_token_account, user.address(), mint.address())?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_ata(
    account: &pinocchio::AccountView,
    wallet: &Address,
    mint: &Address,
) -> pinocchio::ProgramResult {
    let (expected, _) = derive_ata(wallet, mint)?;
    if *account.address() != expected {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(())
}

/// Verify a token account is the correct ATA for a specific token program.
///
/// Same as [`check_ata`] but for Token-2022 or any explicit token program.
///
/// ```rust,ignore
/// check_ata_with_program(user_ata, wallet.address(), mint.address(), &programs::TOKEN_2022)?;
/// ```
#[cfg(feature = "programs")]
#[inline(always)]
pub fn check_ata_with_program(
    account: &pinocchio::AccountView,
    wallet: &Address,
    mint: &Address,
    token_program: &Address,
) -> pinocchio::ProgramResult {
    let (expected, _) = derive_ata_with_program(wallet, mint, token_program)?;
    if *account.address() != expected {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(())
}
