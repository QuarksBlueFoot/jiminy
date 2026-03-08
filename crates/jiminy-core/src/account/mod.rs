//! Account-centric types: header, zero-copy IO, lifecycle, POD, iteration.
//!
//! Everything that touches account data layout lives here.
//!
//! ```rust,ignore
//! use jiminy_core::account::{AccountReader, AccountWriter, AccountHeader};
//! ```

pub mod bits;
pub mod collection;
pub mod cursor;
pub mod header;
pub mod lifecycle;
pub mod list;
pub mod overlay;
pub mod pod;
pub mod reader;
pub mod writer;

// ── Re-exports ───────────────────────────────────────────────────────────────

pub use header::{
    AccountHeader, HEADER_LEN, body, body_mut, check_header, header_payload,
    header_payload_mut, read_data_len, read_header_flags, read_version,
    write_header, write_header_with_len,
};
pub use reader::AccountReader;
pub use writer::AccountWriter;
pub use cursor::{DataWriter, SliceCursor, write_discriminator, zero_init};
pub use pod::{Pod, FixedLayout, pod_from_bytes, pod_from_bytes_mut, pod_read, pod_write};
pub use collection::{ZeroCopySlice, ZeroCopySliceMut, ZeroCopyIter};
pub use lifecycle::{
    CLOSE_SENTINEL, safe_close, safe_close_with_sentinel, check_not_revived,
    check_alive, safe_realloc, safe_realloc_shrink,
};
pub use list::AccountList;
pub use bits::{
    check_any_flag, check_flags, clear_bit, read_bit, read_flags_at, set_bit,
    toggle_bit, write_flags_at,
};
