//! Macro ergonomics and migration-bridge tests.

use jiminy_core::abi::LeU64;
use jiminy_core::account::{FixedLayout, Pod};
use jiminy_core::{
    assert_legacy_layout, require, require_accounts_ne, require_eq, require_flag, require_gt,
    require_gte, require_keys_eq, require_keys_neq, require_lt, require_lte, require_neq, Address,
    ProgramError, ProgramResult,
};

#[repr(C)]
#[derive(Clone, Copy)]
struct LegacyVaultV1 {
    authority: [u8; 32],
    balance: LeU64,
}

unsafe impl Pod for LegacyVaultV1 {}
impl FixedLayout for LegacyVaultV1 {
    const SIZE: usize = 40;
}

assert_legacy_layout!(LegacyVaultV1, size = 40, max_align = 1,);

struct FakeAccount(Address);

impl FakeAccount {
    fn address(&self) -> &Address {
        &self.0
    }
}

fn exercise_require_trailing_commas() -> ProgramResult {
    require!(true, ProgramError::InvalidArgument,);
    require_eq!(7u64, 7u64, ProgramError::InvalidArgument,);
    require_neq!(7u64, 8u64, ProgramError::InvalidArgument,);
    require_gte!(8u64, 7u64, ProgramError::InvalidArgument,);
    require_gt!(8u64, 7u64, ProgramError::InvalidArgument,);
    require_lte!(7u64, 8u64, ProgramError::InvalidArgument,);
    require_lt!(7u64, 8u64, ProgramError::InvalidArgument,);
    require_flag!(0b10u8, 1, ProgramError::InvalidArgument,);

    let a = FakeAccount(Address::new_from_array([1u8; 32]));
    let b = FakeAccount(Address::new_from_array([2u8; 32]));
    require_accounts_ne!(a, b, ProgramError::InvalidArgument,);

    Ok(())
}

fn exercise_key_macros() -> ProgramResult {
    let a = Address::new_from_array([3u8; 32]);
    let b = Address::new_from_array([3u8; 32]);
    let c = Address::new_from_array([4u8; 32]);

    require_keys_eq!(a, b, ProgramError::InvalidArgument,);
    require_keys_eq!(&a, &b, ProgramError::InvalidArgument,);
    require_keys_eq!(a, &b, ProgramError::InvalidArgument,);
    require_keys_eq!(&a, b, ProgramError::InvalidArgument,);
    require_keys_neq!(a, c, ProgramError::InvalidArgument,);
    require_keys_neq!(&a, &c, ProgramError::InvalidArgument,);

    Ok(())
}

#[test]
fn require_macros_accept_trailing_commas() {
    exercise_require_trailing_commas().unwrap();
}

#[test]
fn key_macros_accept_owned_and_borrowed_addresses() {
    exercise_key_macros().unwrap();
}

#[test]
fn assert_legacy_layout_checks_size_and_traits() {
    assert_eq!(core::mem::size_of::<LegacyVaultV1>(), LegacyVaultV1::SIZE);
    assert_eq!(LegacyVaultV1::SIZE, 40);
    assert_eq!(core::mem::align_of::<LegacyVaultV1>(), 1);
}
