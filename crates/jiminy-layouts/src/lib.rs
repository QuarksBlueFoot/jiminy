//! # jiminy-layouts
//!
//! Standard zero-copy account layouts for well-known Solana programs.
//!
//! This crate provides `#[repr(C)]` structs with [`Pod`] and [`FixedLayout`]
//! implementations for SPL Token accounts, Mint accounts, and other
//! widely-used on-chain data structures. These layouts are compatible
//! with `jiminy-core`'s `pod_from_bytes()` for direct zero-copy access.
//!
//! ## Why a separate crate?
//!
//! `jiminy-solana` provides *function-based* field readers (e.g.,
//! `token_account_owner(account)`). This crate provides *struct-based*
//! overlays that map the entire account into a typed struct - useful when
//! you need to read multiple fields efficiently.
//!
//! ## Layouts
//!
//! | Struct | Program | Size |
//! |--------|---------|------|
//! | [`SplTokenAccount`] | SPL Token | 165 bytes |
//! | [`SplMint`] | SPL Token | 82 bytes |
//! | [`SplMultisig`] | SPL Token | 355 bytes |
//! | [`NonceAccount`] | System program | 80 bytes |
//! | [`StakeState`] | Stake program | 200 bytes |
//!
//! ## Coverage philosophy
//!
//! This crate targets account types that programs commonly *read*
//! cross-program. SPL Token accounts dominate that set. Nonce and
//! Stake accounts are included because staking programs and
//! durable-transaction workflows frequently inspect them.
//! Additional layouts (e.g., Metaplex Token Metadata) can be added
//! as the ecosystem matures.
//!
//! ## Example
//!
//! ```rust,ignore
//! use jiminy_layouts::SplTokenAccount;
//! use jiminy_core::account::{pod_from_bytes, FixedLayout};
//!
//! let data: &[u8] = &account.data;
//! let token = pod_from_bytes::<SplTokenAccount>(data)?;
//! let owner = token.owner;
//! let amount = u64::from_le_bytes(token.amount);
//! ```
//!
//! ## Important
//!
//! These are **external** (non-Jiminy) account layouts - they do NOT have
//! the Jiminy 16-byte header. They are meant for reading accounts owned
//! by other programs (SPL Token, System, Stake, etc.).

#![no_std]

use jiminy_core::account::{Pod, FixedLayout};

// ── SPL Token Account ────────────────────────────────────────────────────────

/// Zero-copy overlay for an SPL Token account (165 bytes).
///
/// Layout:
/// ```text
///   0..32   mint           [u8; 32]
///  32..64   owner          [u8; 32]
///  64..72   amount         [u8; 8]   (u64 LE)
///  72..76   delegate_tag   [u8; 4]   (u32 LE: 0=None, 1=Some)
///  76..108  delegate       [u8; 32]
/// 108..109  state          u8        (0=uninit, 1=init, 2=frozen)
/// 109..113  is_native_tag  [u8; 4]   (u32 LE: 0=None, 1=Some)
/// 113..121  native_amount  [u8; 8]   (u64 LE)
/// 121..129  delegated_amt  [u8; 8]   (u64 LE)
/// 129..133  close_auth_tag [u8; 4]   (u32 LE: 0=None, 1=Some)
/// 133..165  close_auth     [u8; 32]
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SplTokenAccount {
    /// Mint address associated with this token account.
    pub mint: [u8; 32],
    /// Owner of this token account.
    pub owner: [u8; 32],
    /// Token balance (u64 LE).
    pub amount: [u8; 8],
    /// Delegate option tag (u32 LE: 0=None, 1=Some).
    pub delegate_tag: [u8; 4],
    /// Delegate address (valid only if delegate_tag == 1).
    pub delegate: [u8; 32],
    /// Account state: 0=Uninitialized, 1=Initialized, 2=Frozen.
    pub state: u8,
    /// Is-native option tag (u32 LE).
    pub is_native_tag: [u8; 4],
    /// Native SOL amount (u64 LE, valid only if is_native_tag == 1).
    pub native_amount: [u8; 8],
    /// Delegated amount (u64 LE).
    pub delegated_amount: [u8; 8],
    /// Close authority option tag (u32 LE).
    pub close_authority_tag: [u8; 4],
    /// Close authority address (valid only if close_authority_tag == 1).
    pub close_authority: [u8; 32],
}

// SAFETY: SplTokenAccount is #[repr(C)], Copy, all fields are byte arrays,
// and all bit patterns are valid.
unsafe impl Pod for SplTokenAccount {}
impl FixedLayout for SplTokenAccount { const SIZE: usize = 165; }

impl SplTokenAccount {
    /// Read the token amount as a u64.
    #[inline(always)]
    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    /// Check whether a delegate is set.
    #[inline(always)]
    pub fn has_delegate(&self) -> bool {
        u32::from_le_bytes(self.delegate_tag) == 1
    }

    /// Check whether this is a native (SOL-wrapped) token account.
    #[inline(always)]
    pub fn is_native(&self) -> bool {
        u32::from_le_bytes(self.is_native_tag) == 1
    }

    /// Check whether a close authority is set.
    #[inline(always)]
    pub fn has_close_authority(&self) -> bool {
        u32::from_le_bytes(self.close_authority_tag) == 1
    }

    /// Check if the account is initialized.
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.state == 1
    }

    /// Check if the account is frozen.
    #[inline(always)]
    pub fn is_frozen(&self) -> bool {
        self.state == 2
    }
}

// ── SPL Mint ─────────────────────────────────────────────────────────────────

/// Zero-copy overlay for an SPL Token mint account (82 bytes).
///
/// Layout:
/// ```text
///  0..4    mint_authority_tag  [u8; 4] (u32 LE: 0=None, 1=Some)
///  4..36   mint_authority      [u8; 32]
/// 36..44   supply              [u8; 8] (u64 LE)
/// 44       decimals            u8
/// 45       is_initialized      u8 (bool)
/// 46..50   freeze_auth_tag     [u8; 4] (u32 LE: 0=None, 1=Some)
/// 50..82   freeze_authority    [u8; 32]
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SplMint {
    /// Mint authority option tag (u32 LE).
    pub mint_authority_tag: [u8; 4],
    /// Mint authority address (valid only if tag == 1).
    pub mint_authority: [u8; 32],
    /// Total supply (u64 LE).
    pub supply: [u8; 8],
    /// Number of decimals.
    pub decimals: u8,
    /// Whether the mint is initialized.
    pub is_initialized: u8,
    /// Freeze authority option tag (u32 LE).
    pub freeze_authority_tag: [u8; 4],
    /// Freeze authority address (valid only if tag == 1).
    pub freeze_authority: [u8; 32],
}

// SAFETY: SplMint is #[repr(C)], Copy, all fields are byte arrays/u8.
unsafe impl Pod for SplMint {}
impl FixedLayout for SplMint { const SIZE: usize = 82; }

impl SplMint {
    /// Read the total supply as a u64.
    #[inline(always)]
    pub fn supply(&self) -> u64 {
        u64::from_le_bytes(self.supply)
    }

    /// Check whether a mint authority is set.
    #[inline(always)]
    pub fn has_mint_authority(&self) -> bool {
        u32::from_le_bytes(self.mint_authority_tag) == 1
    }

    /// Check whether a freeze authority is set.
    #[inline(always)]
    pub fn has_freeze_authority(&self) -> bool {
        u32::from_le_bytes(self.freeze_authority_tag) == 1
    }
}

// ── SPL Multisig ─────────────────────────────────────────────────────────────

/// Zero-copy overlay for an SPL Token multisig account (355 bytes).
///
/// Layout:
/// ```text
///   0       m                u8  (signatures required)
///   1       n                u8  (total signers)
///   2       is_initialized   u8  (bool)
///   3..355  signers          [u8; 352] (11 × 32-byte addresses)
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SplMultisig {
    /// Number of signers required.
    pub m: u8,
    /// Total number of valid signers.
    pub n: u8,
    /// Whether the multisig is initialized.
    pub is_initialized: u8,
    /// Signer addresses (up to 11, 32 bytes each).
    pub signers: [u8; 352],
}

// SAFETY: SplMultisig is #[repr(C)], Copy, all fields are u8/byte arrays.
unsafe impl Pod for SplMultisig {}
impl FixedLayout for SplMultisig { const SIZE: usize = 355; }

impl SplMultisig {
    /// Get the address of signer at index `i`.
    ///
    /// Returns `None` if `i >= n` or `i >= 11`.
    #[inline(always)]
    pub fn signer(&self, i: usize) -> Option<&[u8; 32]> {
        if i >= self.n as usize || i >= 11 {
            return None;
        }
        let start = i * 32;
        // SAFETY: i < 11 so start+32 <= 352
        Some(unsafe { &*(self.signers.as_ptr().add(start) as *const [u8; 32]) })
    }
}

// ── System Nonce Account ─────────────────────────────────────────────────────

/// Zero-copy overlay for a system program durable nonce account (80 bytes).
///
/// Layout:
/// ```text
///   0..4    version        [u8; 4]  (u32 LE: 0=Uninitialized, 1=Current)
///   4..8    state          [u8; 4]  (u32 LE: 0=Uninitialized, 1=Initialized)
///   8..40   authority      [u8; 32]
///  40..72   blockhash      [u8; 32]
///  72..80   lamports_per_sig [u8; 8] (u64 LE)
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NonceAccount {
    /// Nonce version (u32 LE).
    pub version: [u8; 4],
    /// Nonce state (u32 LE: 0=Uninitialized, 1=Initialized).
    pub state: [u8; 4],
    /// Authority authorized to advance the nonce.
    pub authority: [u8; 32],
    /// Stored durable blockhash.
    pub blockhash: [u8; 32],
    /// Lamports per signature at the time the nonce was stored (u64 LE).
    pub lamports_per_signature: [u8; 8],
}

// SAFETY: NonceAccount is #[repr(C)], Copy, all fields are byte arrays,
// and all bit patterns are valid.
unsafe impl Pod for NonceAccount {}
impl FixedLayout for NonceAccount { const SIZE: usize = 80; }

impl NonceAccount {
    /// Check whether the nonce is initialized.
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        u32::from_le_bytes(self.state) == 1
    }

    /// Read lamports per signature as a u64.
    #[inline(always)]
    pub fn lamports_per_signature(&self) -> u64 {
        u64::from_le_bytes(self.lamports_per_signature)
    }
}

// ── Stake Account ────────────────────────────────────────────────────────────

/// Zero-copy overlay for the fixed prefix of a stake account (200 bytes).
///
/// Covers the `Meta` portion of StakeState::Stake which is the section
/// most programs need to read. The full StakeState (Stake variant) is
/// 200 bytes total when including the Delegation.
///
/// Layout:
/// ```text
///   0..4    state             [u8; 4]  (u32 LE: enum discriminant)
///   4..12   rent_exempt_reserve [u8; 8] (u64 LE)
///  12..44   authorized_staker [u8; 32]
///  44..76   authorized_withdrawer [u8; 32]
///  76..84   lockup_timestamp  [u8; 8] (i64 LE: Unix timestamp)
///  84..92   lockup_epoch      [u8; 8] (u64 LE)
///  92..124  lockup_custodian  [u8; 32]
/// 124..156  voter_pubkey      [u8; 32]
/// 156..164  stake_amount      [u8; 8] (u64 LE)
/// 164..172  activation_epoch  [u8; 8] (u64 LE)
/// 172..180  deactivation_epoch [u8; 8] (u64 LE)
/// 180..188  warmup_cooldown_rate [u8; 8] (f64 LE)
/// 188..196  credits_observed  [u8; 8] (u64 LE)
/// 196..200  _padding          [u8; 4]
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StakeState {
    /// StakeState enum discriminant (u32 LE):
    /// 0=Uninitialized, 1=Initialized, 2=Stake, 3=RewardsPool.
    pub state: [u8; 4],
    /// Rent-exempt reserve (u64 LE).
    pub rent_exempt_reserve: [u8; 8],
    /// Authorized staker pubkey.
    pub authorized_staker: [u8; 32],
    /// Authorized withdrawer pubkey.
    pub authorized_withdrawer: [u8; 32],
    /// Lockup Unix timestamp (i64 LE).
    pub lockup_timestamp: [u8; 8],
    /// Lockup epoch (u64 LE).
    pub lockup_epoch: [u8; 8],
    /// Lockup custodian pubkey.
    pub lockup_custodian: [u8; 32],
    /// Voter pubkey (valid when state == 2).
    pub voter_pubkey: [u8; 32],
    /// Delegated stake amount (u64 LE).
    pub stake_amount: [u8; 8],
    /// Activation epoch (u64 LE).
    pub activation_epoch: [u8; 8],
    /// Deactivation epoch (u64 LE, u64::MAX if not deactivating).
    pub deactivation_epoch: [u8; 8],
    /// Warmup/cooldown rate (f64 LE).
    pub warmup_cooldown_rate: [u8; 8],
    /// Credits observed (u64 LE).
    pub credits_observed: [u8; 8],
    /// Padding to 200 bytes.
    pub _padding: [u8; 4],
}

// SAFETY: StakeState is #[repr(C)], Copy, all fields are byte arrays,
// and all bit patterns are valid.
unsafe impl Pod for StakeState {}
impl FixedLayout for StakeState { const SIZE: usize = 200; }

impl StakeState {
    /// Get the state discriminant.
    #[inline(always)]
    pub fn state_kind(&self) -> u32 {
        u32::from_le_bytes(self.state)
    }

    /// Check whether the stake is in the `Stake` state (delegated).
    #[inline(always)]
    pub fn is_delegated(&self) -> bool {
        self.state_kind() == 2
    }

    /// Read the delegated stake amount as a u64.
    #[inline(always)]
    pub fn stake_amount(&self) -> u64 {
        u64::from_le_bytes(self.stake_amount)
    }

    /// Read the activation epoch as a u64.
    #[inline(always)]
    pub fn activation_epoch(&self) -> u64 {
        u64::from_le_bytes(self.activation_epoch)
    }

    /// Read the deactivation epoch as a u64.
    /// Returns `u64::MAX` if not deactivating.
    #[inline(always)]
    pub fn deactivation_epoch(&self) -> u64 {
        u64::from_le_bytes(self.deactivation_epoch)
    }
}

// ── Compile-time size assertions ─────────────────────────────────────────────

const _: () = assert!(core::mem::size_of::<SplTokenAccount>() == 165);
const _: () = assert!(core::mem::size_of::<SplMint>() == 82);
const _: () = assert!(core::mem::size_of::<SplMultisig>() == 355);
const _: () = assert!(core::mem::size_of::<NonceAccount>() == 80);
const _: () = assert!(core::mem::size_of::<StakeState>() == 200);

// ── Compile-time alignment assertions ────────────────────────────────────────

const _: () = assert!(core::mem::align_of::<SplTokenAccount>() == 1);
const _: () = assert!(core::mem::align_of::<SplMint>() == 1);
const _: () = assert!(core::mem::align_of::<SplMultisig>() == 1);
const _: () = assert!(core::mem::align_of::<NonceAccount>() == 1);
const _: () = assert!(core::mem::align_of::<StakeState>() == 1);

#[cfg(test)]
mod tests {
    use super::*;
    use jiminy_core::account::pod_from_bytes;

    // ── Helper: write bytes and cast via pod_from_bytes ──────────────────

    fn offset_of<T, F>(base: *const T, field: *const F) -> usize {
        (field as usize) - (base as usize)
    }

    // ── SplTokenAccount ──────────────────────────────────────────────────

    #[test]
    fn token_account_size() {
        assert_eq!(SplTokenAccount::SIZE, 165);
        assert_eq!(core::mem::size_of::<SplTokenAccount>(), 165);
    }

    #[test]
    fn token_account_field_offsets() {
        let t = SplTokenAccount {
            mint: [0; 32],
            owner: [0; 32],
            amount: [0; 8],
            delegate_tag: [0; 4],
            delegate: [0; 32],
            state: 0,
            is_native_tag: [0; 4],
            native_amount: [0; 8],
            delegated_amount: [0; 8],
            close_authority_tag: [0; 4],
            close_authority: [0; 32],
        };
        let base = &t as *const SplTokenAccount;
        assert_eq!(offset_of(base, &t.mint as *const _), 0);
        assert_eq!(offset_of(base, &t.owner as *const _), 32);
        assert_eq!(offset_of(base, &t.amount as *const _), 64);
        assert_eq!(offset_of(base, &t.delegate_tag as *const _), 72);
        assert_eq!(offset_of(base, &t.delegate as *const _), 76);
        assert_eq!(offset_of(base, &t.state as *const _), 108);
        assert_eq!(offset_of(base, &t.is_native_tag as *const _), 109);
        assert_eq!(offset_of(base, &t.native_amount as *const _), 113);
        assert_eq!(offset_of(base, &t.delegated_amount as *const _), 121);
        assert_eq!(offset_of(base, &t.close_authority_tag as *const _), 129);
        assert_eq!(offset_of(base, &t.close_authority as *const _), 133);
    }

    #[test]
    fn token_account_pod_roundtrip() {
        let mut buf = [0u8; 165];
        // Write a known mint at byte 0.
        buf[0..32].copy_from_slice(&[0xAA; 32]);
        // Write amount at byte 64.
        buf[64..72].copy_from_slice(&1_000_000u64.to_le_bytes());
        // Write state=Initialized at byte 108.
        buf[108] = 1;
        // Write delegate_tag=1 at byte 72.
        buf[72..76].copy_from_slice(&1u32.to_le_bytes());
        // Write delegate at byte 76.
        buf[76..108].copy_from_slice(&[0xBB; 32]);

        let token = pod_from_bytes::<SplTokenAccount>(&buf).unwrap();
        assert_eq!(token.mint, [0xAA; 32]);
        assert_eq!(token.amount(), 1_000_000);
        assert!(token.is_initialized());
        assert!(!token.is_frozen());
        assert!(token.has_delegate());
        assert_eq!(token.delegate, [0xBB; 32]);
        assert!(!token.is_native());
        assert!(!token.has_close_authority());
    }

    // ── SplMint ──────────────────────────────────────────────────────────

    #[test]
    fn mint_size() {
        assert_eq!(SplMint::SIZE, 82);
        assert_eq!(core::mem::size_of::<SplMint>(), 82);
    }

    #[test]
    fn mint_field_offsets() {
        let m = SplMint {
            mint_authority_tag: [0; 4],
            mint_authority: [0; 32],
            supply: [0; 8],
            decimals: 0,
            is_initialized: 0,
            freeze_authority_tag: [0; 4],
            freeze_authority: [0; 32],
        };
        let base = &m as *const SplMint;
        assert_eq!(offset_of(base, &m.mint_authority_tag as *const _), 0);
        assert_eq!(offset_of(base, &m.mint_authority as *const _), 4);
        assert_eq!(offset_of(base, &m.supply as *const _), 36);
        assert_eq!(offset_of(base, &m.decimals as *const _), 44);
        assert_eq!(offset_of(base, &m.is_initialized as *const _), 45);
        assert_eq!(offset_of(base, &m.freeze_authority_tag as *const _), 46);
        assert_eq!(offset_of(base, &m.freeze_authority as *const _), 50);
    }

    #[test]
    fn mint_pod_roundtrip() {
        let mut buf = [0u8; 82];
        // mint_authority_tag=1 at 0..4
        buf[0..4].copy_from_slice(&1u32.to_le_bytes());
        // mint_authority at 4..36
        buf[4..36].copy_from_slice(&[0xCC; 32]);
        // supply at 36..44
        buf[36..44].copy_from_slice(&1_000_000_000u64.to_le_bytes());
        // decimals at 44
        buf[44] = 9;
        // is_initialized at 45
        buf[45] = 1;

        let mint = pod_from_bytes::<SplMint>(&buf).unwrap();
        assert!(mint.has_mint_authority());
        assert_eq!(mint.mint_authority, [0xCC; 32]);
        assert_eq!(mint.supply(), 1_000_000_000);
        assert_eq!(mint.decimals, 9);
        assert!(!mint.has_freeze_authority());
    }

    // ── SplMultisig ──────────────────────────────────────────────────────

    #[test]
    fn multisig_size() {
        assert_eq!(SplMultisig::SIZE, 355);
        assert_eq!(core::mem::size_of::<SplMultisig>(), 355);
    }

    #[test]
    fn multisig_field_offsets() {
        let ms = SplMultisig {
            m: 0,
            n: 0,
            is_initialized: 0,
            signers: [0; 352],
        };
        let base = &ms as *const SplMultisig;
        assert_eq!(offset_of(base, &ms.m as *const _), 0);
        assert_eq!(offset_of(base, &ms.n as *const _), 1);
        assert_eq!(offset_of(base, &ms.is_initialized as *const _), 2);
        assert_eq!(offset_of(base, &ms.signers as *const _), 3);
    }

    #[test]
    fn multisig_signer_access() {
        let mut ms = SplMultisig {
            m: 2,
            n: 3,
            is_initialized: 1,
            signers: [0; 352],
        };
        ms.signers[0..32].copy_from_slice(&[0x11; 32]);
        ms.signers[32..64].copy_from_slice(&[0x22; 32]);
        ms.signers[64..96].copy_from_slice(&[0x33; 32]);

        assert_eq!(ms.signer(0).unwrap(), &[0x11; 32]);
        assert_eq!(ms.signer(1).unwrap(), &[0x22; 32]);
        assert_eq!(ms.signer(2).unwrap(), &[0x33; 32]);
        assert!(ms.signer(3).is_none()); // n=3, so index 3 is out
        assert!(ms.signer(11).is_none()); // hard cap
    }

    // ── NonceAccount ─────────────────────────────────────────────────────

    #[test]
    fn nonce_account_size() {
        assert_eq!(NonceAccount::SIZE, 80);
        assert_eq!(core::mem::size_of::<NonceAccount>(), 80);
    }

    #[test]
    fn nonce_account_field_offsets() {
        let n = NonceAccount {
            version: [0; 4],
            state: [0; 4],
            authority: [0; 32],
            blockhash: [0; 32],
            lamports_per_signature: [0; 8],
        };
        let base = &n as *const NonceAccount;
        assert_eq!(offset_of(base, &n.version as *const _), 0);
        assert_eq!(offset_of(base, &n.state as *const _), 4);
        assert_eq!(offset_of(base, &n.authority as *const _), 8);
        assert_eq!(offset_of(base, &n.blockhash as *const _), 40);
        assert_eq!(offset_of(base, &n.lamports_per_signature as *const _), 72);
    }

    #[test]
    fn nonce_account_pod_roundtrip() {
        let mut buf = [0u8; 80];
        // version=1 at 0..4
        buf[0..4].copy_from_slice(&1u32.to_le_bytes());
        // state=1 (init) at 4..8
        buf[4..8].copy_from_slice(&1u32.to_le_bytes());
        // authority at 8..40
        buf[8..40].copy_from_slice(&[0xDD; 32]);
        // lamports_per_sig at 72..80
        buf[72..80].copy_from_slice(&5000u64.to_le_bytes());

        let nonce = pod_from_bytes::<NonceAccount>(&buf).unwrap();
        assert!(nonce.is_initialized());
        assert_eq!(nonce.authority, [0xDD; 32]);
        assert_eq!(nonce.lamports_per_signature(), 5000);
    }

    #[test]
    fn nonce_account_initialized() {
        let mut nonce = NonceAccount {
            version: 1u32.to_le_bytes(),
            state: 0u32.to_le_bytes(),
            authority: [0; 32],
            blockhash: [0; 32],
            lamports_per_signature: [0; 8],
        };
        assert!(!nonce.is_initialized());
        nonce.state = 1u32.to_le_bytes();
        assert!(nonce.is_initialized());
    }

    // ── StakeState ───────────────────────────────────────────────────────

    #[test]
    fn stake_state_size() {
        assert_eq!(StakeState::SIZE, 200);
        assert_eq!(core::mem::size_of::<StakeState>(), 200);
    }

    #[test]
    fn stake_state_field_offsets() {
        let s = StakeState {
            state: [0; 4],
            rent_exempt_reserve: [0; 8],
            authorized_staker: [0; 32],
            authorized_withdrawer: [0; 32],
            lockup_timestamp: [0; 8],
            lockup_epoch: [0; 8],
            lockup_custodian: [0; 32],
            voter_pubkey: [0; 32],
            stake_amount: [0; 8],
            activation_epoch: [0; 8],
            deactivation_epoch: [0; 8],
            warmup_cooldown_rate: [0; 8],
            credits_observed: [0; 8],
            _padding: [0; 4],
        };
        let base = &s as *const StakeState;
        assert_eq!(offset_of(base, &s.state as *const _), 0);
        assert_eq!(offset_of(base, &s.rent_exempt_reserve as *const _), 4);
        assert_eq!(offset_of(base, &s.authorized_staker as *const _), 12);
        assert_eq!(offset_of(base, &s.authorized_withdrawer as *const _), 44);
        assert_eq!(offset_of(base, &s.lockup_timestamp as *const _), 76);
        assert_eq!(offset_of(base, &s.lockup_epoch as *const _), 84);
        assert_eq!(offset_of(base, &s.lockup_custodian as *const _), 92);
        assert_eq!(offset_of(base, &s.voter_pubkey as *const _), 124);
        assert_eq!(offset_of(base, &s.stake_amount as *const _), 156);
        assert_eq!(offset_of(base, &s.activation_epoch as *const _), 164);
        assert_eq!(offset_of(base, &s.deactivation_epoch as *const _), 172);
        assert_eq!(offset_of(base, &s.warmup_cooldown_rate as *const _), 180);
        assert_eq!(offset_of(base, &s.credits_observed as *const _), 188);
        assert_eq!(offset_of(base, &s._padding as *const _), 196);
    }

    #[test]
    fn stake_state_pod_roundtrip() {
        let mut buf = [0u8; 200];
        // state=2 (Stake) at 0..4
        buf[0..4].copy_from_slice(&2u32.to_le_bytes());
        // voter_pubkey at 124..156
        buf[124..156].copy_from_slice(&[0xEE; 32]);
        // stake_amount at 156..164
        buf[156..164].copy_from_slice(&5_000_000u64.to_le_bytes());
        // activation_epoch at 164..172
        buf[164..172].copy_from_slice(&42u64.to_le_bytes());
        // deactivation_epoch at 172..180 (u64::MAX = not deactivating)
        buf[172..180].copy_from_slice(&u64::MAX.to_le_bytes());

        let stake = pod_from_bytes::<StakeState>(&buf).unwrap();
        assert!(stake.is_delegated());
        assert_eq!(stake.voter_pubkey, [0xEE; 32]);
        assert_eq!(stake.stake_amount(), 5_000_000);
        assert_eq!(stake.activation_epoch(), 42);
        assert_eq!(stake.deactivation_epoch(), u64::MAX);
    }

    #[test]
    fn stake_state_delegated() {
        let mut stake = StakeState {
            state: 2u32.to_le_bytes(),
            rent_exempt_reserve: [0; 8],
            authorized_staker: [0; 32],
            authorized_withdrawer: [0; 32],
            lockup_timestamp: [0; 8],
            lockup_epoch: [0; 8],
            lockup_custodian: [0; 32],
            voter_pubkey: [0; 32],
            stake_amount: 1_000_000u64.to_le_bytes(),
            activation_epoch: 100u64.to_le_bytes(),
            deactivation_epoch: u64::MAX.to_le_bytes(),
            warmup_cooldown_rate: [0; 8],
            credits_observed: [0; 8],
            _padding: [0; 4],
        };
        assert!(stake.is_delegated());
        assert_eq!(stake.stake_amount(), 1_000_000);
        assert_eq!(stake.activation_epoch(), 100);
        assert_eq!(stake.deactivation_epoch(), u64::MAX);

        stake.state = 1u32.to_le_bytes(); // Initialized, not Stake
        assert!(!stake.is_delegated());
    }

    // ── Cross-layout: raw bytes vs pod_from_bytes ────────────────────────

    #[test]
    fn token_account_bytes_match_struct() {
        let token = SplTokenAccount {
            mint: [1; 32],
            owner: [2; 32],
            amount: 42u64.to_le_bytes(),
            delegate_tag: 0u32.to_le_bytes(),
            delegate: [0; 32],
            state: 1,
            is_native_tag: 0u32.to_le_bytes(),
            native_amount: [0; 8],
            delegated_amount: [0; 8],
            close_authority_tag: 0u32.to_le_bytes(),
            close_authority: [0; 32],
        };
        // Cast struct to bytes and back via pod_from_bytes.
        let bytes = unsafe {
            core::slice::from_raw_parts(
                &token as *const SplTokenAccount as *const u8,
                165,
            )
        };
        let roundtrip = pod_from_bytes::<SplTokenAccount>(bytes).unwrap();
        assert_eq!(roundtrip.mint, [1; 32]);
        assert_eq!(roundtrip.owner, [2; 32]);
        assert_eq!(roundtrip.amount(), 42);
        assert!(roundtrip.is_initialized());
    }
}
