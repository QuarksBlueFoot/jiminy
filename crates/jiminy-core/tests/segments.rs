//! Tests for the segmented ABI: descriptors, tables, slices, and the
//! `segmented_layout!` macro.

use jiminy_core::account::*;
use jiminy_core::segmented_layout;
use pinocchio::Address;

// ── Test element types ───────────────────────────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    pub price: [u8; 8],
    pub qty: [u8; 8],
}

unsafe impl Pod for Order {}
impl FixedLayout for Order {
    const SIZE: usize = 16;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub key: [u8; 4],
}

unsafe impl Pod for Entry {}
impl FixedLayout for Entry {
    const SIZE: usize = 4;
}

// ── Test layouts ─────────────────────────────────────────────────────────────

segmented_layout! {
    pub struct OrderBook, discriminator = 5, version = 1 {
        header:     AccountHeader = 16,
        market:     Address       = 32,
    } segments {
        bids: Order = 16,
        asks: Order = 16,
    }
}

segmented_layout! {
    pub struct Registry, discriminator = 10, version = 1 {
        header: AccountHeader = 16,
    } segments {
        entries: Entry = 4,
    }
}

// ── Helper ───────────────────────────────────────────────────────────────────

#[repr(C, align(8))]
struct AlignedBuf<const N: usize>([u8; N]);

impl<const N: usize> AlignedBuf<N> {
    fn new() -> Self {
        Self([0u8; N])
    }
    fn as_slice(&self) -> &[u8] {
        &self.0
    }
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 1. SegmentDescriptor
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn descriptor_size_is_8() {
    assert_eq!(SEGMENT_DESC_SIZE, 8);
    assert_eq!(core::mem::size_of::<SegmentDescriptor>(), 8);
}

#[test]
fn descriptor_alignment_is_1() {
    assert_eq!(core::mem::align_of::<SegmentDescriptor>(), 1);
}

#[test]
fn descriptor_new_and_accessors() {
    let desc = SegmentDescriptor::new(128, 10, 16);
    assert_eq!(desc.offset(), 128);
    assert_eq!(desc.count(), 10);
    assert_eq!(desc.element_size(), 16);
    assert_eq!(desc.data_len(), 160);
    assert_eq!(desc.byte_range(), Some((128, 288)));
}

#[test]
fn descriptor_zero_count() {
    let desc = SegmentDescriptor::new(64, 0, 8);
    assert_eq!(desc.data_len(), 0);
    assert_eq!(desc.byte_range(), Some((64, 64)));
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. SegmentTable
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn table_from_bytes_too_small() {
    let data = [0u8; 7]; // Need 8 for 1 descriptor.
    assert!(SegmentTable::from_bytes(&data, 1).is_err());
}

#[test]
fn table_from_bytes_over_max() {
    let data = [0u8; 256];
    assert!(SegmentTable::from_bytes(&data, MAX_SEGMENTS + 1).is_err());
}

#[test]
fn table_zero_segments() {
    let data = [0u8; 0];
    let table = SegmentTable::from_bytes(&data, 0).unwrap();
    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
}

#[test]
fn table_read_descriptors() {
    let mut buf = [0u8; 16];
    // Descriptor 0: offset=48, count=3, elem_size=16
    buf[0..4].copy_from_slice(&48u32.to_le_bytes());
    buf[4..6].copy_from_slice(&3u16.to_le_bytes());
    buf[6..8].copy_from_slice(&16u16.to_le_bytes());
    // Descriptor 1: offset=96, count=2, elem_size=16
    buf[8..12].copy_from_slice(&96u32.to_le_bytes());
    buf[12..14].copy_from_slice(&2u16.to_le_bytes());
    buf[14..16].copy_from_slice(&16u16.to_le_bytes());

    let table = SegmentTable::from_bytes(&buf, 2).unwrap();
    assert_eq!(table.len(), 2);

    let d0 = table.descriptor(0).unwrap();
    assert_eq!(d0.offset(), 48);
    assert_eq!(d0.count(), 3);
    assert_eq!(d0.element_size(), 16);

    let d1 = table.descriptor(1).unwrap();
    assert_eq!(d1.offset(), 96);
    assert_eq!(d1.count(), 2);
}

#[test]
fn table_descriptor_out_of_bounds() {
    let buf = [0u8; 8];
    let table = SegmentTable::from_bytes(&buf, 1).unwrap();
    assert!(table.descriptor(1).is_err());
}

#[test]
fn table_validate_good() {
    let mut buf = [0u8; 16];
    // Seg 0 at offset 16, count 2, elem_size 4 → occupies [16..24)
    buf[0..4].copy_from_slice(&16u32.to_le_bytes());
    buf[4..6].copy_from_slice(&2u16.to_le_bytes());
    buf[6..8].copy_from_slice(&4u16.to_le_bytes());
    // Seg 1 at offset 24, count 1, elem_size 4 → occupies [24..28)
    buf[8..12].copy_from_slice(&24u32.to_le_bytes());
    buf[12..14].copy_from_slice(&1u16.to_le_bytes());
    buf[14..16].copy_from_slice(&4u16.to_le_bytes());

    let table = SegmentTable::from_bytes(&buf, 2).unwrap();
    assert!(table.validate(28, &[4, 4], 16).is_ok());
}

#[test]
fn table_validate_overlap() {
    let mut buf = [0u8; 16];
    // Seg 0 at offset 16, count 3, elem_size 4 → occupies [16..28)
    buf[0..4].copy_from_slice(&16u32.to_le_bytes());
    buf[4..6].copy_from_slice(&3u16.to_le_bytes());
    buf[6..8].copy_from_slice(&4u16.to_le_bytes());
    // Seg 1 at offset 20, count 1, elem_size 4 → overlaps with seg 0
    buf[8..12].copy_from_slice(&20u32.to_le_bytes());
    buf[12..14].copy_from_slice(&1u16.to_le_bytes());
    buf[14..16].copy_from_slice(&4u16.to_le_bytes());

    let table = SegmentTable::from_bytes(&buf, 2).unwrap();
    assert!(table.validate(100, &[4, 4], 0).is_err());
}

#[test]
fn table_validate_wrong_elem_size() {
    let mut buf = [0u8; 8];
    buf[0..4].copy_from_slice(&8u32.to_le_bytes());
    buf[4..6].copy_from_slice(&1u16.to_le_bytes());
    buf[6..8].copy_from_slice(&8u16.to_le_bytes()); // elem_size = 8

    let table = SegmentTable::from_bytes(&buf, 1).unwrap();
    // Expected size is 4, but descriptor says 8.
    assert!(table.validate(100, &[4], 0).is_err());
}

#[test]
fn table_validate_zero_elem_size() {
    let mut buf = [0u8; 8];
    buf[0..4].copy_from_slice(&8u32.to_le_bytes());
    buf[4..6].copy_from_slice(&1u16.to_le_bytes());
    buf[6..8].copy_from_slice(&0u16.to_le_bytes()); // elem_size = 0

    let table = SegmentTable::from_bytes(&buf, 1).unwrap();
    assert!(table.validate(100, &[0], 0).is_err()); // zero elem_size rejected
}

#[test]
fn table_validate_exceeds_account() {
    let mut buf = [0u8; 8];
    buf[0..4].copy_from_slice(&8u32.to_le_bytes());
    buf[4..6].copy_from_slice(&10u16.to_le_bytes());
    buf[6..8].copy_from_slice(&4u16.to_le_bytes()); // needs 40 bytes at offset 8 → total 48

    let table = SegmentTable::from_bytes(&buf, 1).unwrap();
    assert!(table.validate(40, &[4], 0).is_err()); // account only 40 bytes, need 48
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. SegmentTableMut
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn table_mut_init_and_read() {
    let mut buf = [0u8; 32];
    let specs = [(4u16, 3u16), (8u16, 2u16)];
    let data_start = 48u32;

    let table = SegmentTableMut::init(&mut buf, data_start, &specs).unwrap();

    let d0 = table.descriptor(0).unwrap();
    assert_eq!(d0.offset(), 48);
    assert_eq!(d0.count(), 3);
    assert_eq!(d0.element_size(), 4);

    let d1 = table.descriptor(1).unwrap();
    // offset = 48 + 3*4 = 60
    assert_eq!(d1.offset(), 60);
    assert_eq!(d1.count(), 2);
    assert_eq!(d1.element_size(), 8);
}

#[test]
fn table_mut_set_descriptor() {
    let mut buf = [0u8; 16];
    let mut table = SegmentTableMut::from_bytes(&mut buf, 2).unwrap();

    let desc = SegmentDescriptor::new(100, 5, 12);
    table.set_descriptor(0, &desc).unwrap();

    let read_back = table.descriptor(0).unwrap();
    assert_eq!(read_back.offset(), 100);
    assert_eq!(read_back.count(), 5);
    assert_eq!(read_back.element_size(), 12);
}

#[test]
fn table_mut_set_out_of_bounds() {
    let mut buf = [0u8; 8];
    let mut table = SegmentTableMut::from_bytes(&mut buf, 1).unwrap();
    assert!(table.set_descriptor(1, &SegmentDescriptor::new(0, 0, 1)).is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. SegmentSlice / SegmentSliceMut
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn slice_from_descriptor_good() {
    let mut data = AlignedBuf::<128>::new();
    let buf = data.as_mut_slice();

    // Place 2 Entry (4-byte) elements at offset 16.
    let desc = SegmentDescriptor::new(16, 2, 4);
    buf[16] = 0xAA;
    buf[17] = 0xBB;
    buf[18] = 0xCC;
    buf[19] = 0xDD;
    buf[20] = 0x11;
    buf[21] = 0x22;
    buf[22] = 0x33;
    buf[23] = 0x44;

    let slice = SegmentSlice::<Entry>::from_descriptor(data.as_slice(), &desc).unwrap();
    assert_eq!(slice.len(), 2);
    assert!(!slice.is_empty());

    let e0 = slice.read(0).unwrap();
    assert_eq!(e0.key, [0xAA, 0xBB, 0xCC, 0xDD]);

    let e1 = slice.read(1).unwrap();
    assert_eq!(e1.key, [0x11, 0x22, 0x33, 0x44]);
}

#[test]
fn slice_wrong_element_size() {
    let data = [0u8; 64];
    let desc = SegmentDescriptor::new(0, 1, 8); // 8 != Entry::SIZE (4)
    assert!(SegmentSlice::<Entry>::from_descriptor(&data, &desc).is_err());
}

#[test]
fn slice_out_of_bounds_data() {
    let data = [0u8; 10];
    let desc = SegmentDescriptor::new(0, 3, 4); // needs 12 bytes
    assert!(SegmentSlice::<Entry>::from_descriptor(&data, &desc).is_err());
}

#[test]
fn slice_read_out_of_bounds_index() {
    let data = [0u8; 64];
    let desc = SegmentDescriptor::new(0, 2, 4);
    let slice = SegmentSlice::<Entry>::from_descriptor(&data, &desc).unwrap();
    assert!(slice.read(2).is_err());
}

#[test]
fn slice_empty_segment() {
    let data = [0u8; 64];
    let desc = SegmentDescriptor::new(0, 0, 4);
    let slice = SegmentSlice::<Entry>::from_descriptor(&data, &desc).unwrap();
    assert!(slice.is_empty());
    assert_eq!(slice.len(), 0);
    assert_eq!(slice.iter().count(), 0);
}

#[test]
fn slice_iterate() {
    let mut data = [0u8; 64];
    // 3 entries at offset 0.
    data[0..4].copy_from_slice(&[1, 0, 0, 0]);
    data[4..8].copy_from_slice(&[2, 0, 0, 0]);
    data[8..12].copy_from_slice(&[3, 0, 0, 0]);

    let desc = SegmentDescriptor::new(0, 3, 4);
    let slice = SegmentSlice::<Entry>::from_descriptor(&data, &desc).unwrap();

    let items: Vec<Entry> = slice.iter().collect();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].key, [1, 0, 0, 0]);
    assert_eq!(items[1].key, [2, 0, 0, 0]);
    assert_eq!(items[2].key, [3, 0, 0, 0]);
}

#[test]
fn slice_mut_set_and_read() {
    let mut data = [0u8; 64];
    let desc = SegmentDescriptor::new(0, 2, 4);

    let mut slice = SegmentSliceMut::<Entry>::from_descriptor(&mut data, &desc).unwrap();
    let entry = Entry { key: [0xDE, 0xAD, 0xBE, 0xEF] };
    slice.set(1, &entry).unwrap();

    let read_back = slice.read(1).unwrap();
    assert_eq!(read_back.key, [0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn slice_mut_out_of_bounds_set() {
    let mut data = [0u8; 64];
    let desc = SegmentDescriptor::new(0, 2, 4);
    let mut slice = SegmentSliceMut::<Entry>::from_descriptor(&mut data, &desc).unwrap();
    assert!(slice.set(2, &Entry { key: [0; 4] }).is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// 5. segmented_layout! macro
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn macro_constants() {
    assert_eq!(OrderBook::DISC, 5);
    assert_eq!(OrderBook::VERSION, 1);
    assert_eq!(OrderBook::FIXED_LEN, 48);  // 16 header + 32 market
    assert_eq!(OrderBook::SEGMENT_COUNT, 2);
    assert_eq!(OrderBook::TABLE_OFFSET, 48);
    assert_eq!(OrderBook::DATA_START_OFFSET, 48 + 2 * 8); // 64
    assert_eq!(OrderBook::MIN_ACCOUNT_SIZE, 64);
}

#[test]
fn macro_segment_sizes() {
    let sizes = OrderBook::segment_sizes();
    assert_eq!(sizes, &[16, 16]); // Order::SIZE = 16 for both segments
}

#[test]
fn macro_segmented_layout_id_differs_from_base() {
    // The SEGMENTED_LAYOUT_ID includes seg: entries, so it must differ
    // from the standard LAYOUT_ID (which only has fixed fields).
    assert_ne!(OrderBook::LAYOUT_ID, OrderBook::SEGMENTED_LAYOUT_ID);
}

#[test]
fn macro_compute_account_size() {
    // 2 bids, 3 asks  →  64 + 2*16 + 3*16 = 64 + 32 + 48 = 144
    let size = OrderBook::compute_account_size(&[2, 3]).unwrap();
    assert_eq!(size, 144);
}

#[test]
fn macro_compute_account_size_zero_counts() {
    let size = OrderBook::compute_account_size(&[0, 0]).unwrap();
    assert_eq!(size, OrderBook::DATA_START_OFFSET);
}

#[test]
fn macro_compute_account_size_wrong_count() {
    assert!(OrderBook::compute_account_size(&[1]).is_err()); // only 1, need 2
}

#[test]
fn macro_init_and_validate_segments() {
    let counts = [3u16, 2u16];
    let size = OrderBook::compute_account_size(&counts).unwrap();
    let mut data = vec![0u8; size];

    OrderBook::init_segments(&mut data, &counts).unwrap();
    OrderBook::validate_segments(&data).unwrap();

    // Read back descriptors.
    let table = OrderBook::segment_table(&data).unwrap();

    let d0 = table.descriptor(0).unwrap();
    assert_eq!(d0.offset(), OrderBook::DATA_START_OFFSET as u32);
    assert_eq!(d0.count(), 3);
    assert_eq!(d0.element_size(), 16);

    let d1 = table.descriptor(1).unwrap();
    assert_eq!(d1.offset(), OrderBook::DATA_START_OFFSET as u32 + 3 * 16);
    assert_eq!(d1.count(), 2);
    assert_eq!(d1.element_size(), 16);
}

#[test]
fn macro_round_trip_segment_data() {
    let counts = [2u16, 1u16];
    let size = OrderBook::compute_account_size(&counts).unwrap();
    let mut data = vec![0u8; size];

    OrderBook::init_segments(&mut data, &counts).unwrap();

    // Write orders into the bids segment.
    let table = OrderBook::segment_table(&data).unwrap();
    let bids_desc = table.descriptor(0).unwrap();
    let asks_desc = table.descriptor(1).unwrap();

    let bid0 = Order { price: [1; 8], qty: [10; 8] };
    let bid1 = Order { price: [2; 8], qty: [20; 8] };
    let ask0 = Order { price: [3; 8], qty: [30; 8] };

    // Write via SegmentSliceMut.
    {
        let mut bids = SegmentSliceMut::<Order>::from_descriptor(&mut data, &bids_desc).unwrap();
        bids.set(0, &bid0).unwrap();
        bids.set(1, &bid1).unwrap();
    }
    {
        let mut asks = SegmentSliceMut::<Order>::from_descriptor(&mut data, &asks_desc).unwrap();
        asks.set(0, &ask0).unwrap();
    }

    // Read back via SegmentSlice.
    let bids = SegmentSlice::<Order>::from_descriptor(&data, &bids_desc).unwrap();
    assert_eq!(bids.read(0).unwrap(), bid0);
    assert_eq!(bids.read(1).unwrap(), bid1);

    let asks = SegmentSlice::<Order>::from_descriptor(&data, &asks_desc).unwrap();
    assert_eq!(asks.read(0).unwrap(), ask0);
}

#[test]
fn macro_single_segment() {
    assert_eq!(Registry::SEGMENT_COUNT, 1);
    assert_eq!(Registry::FIXED_LEN, 16);
    assert_eq!(Registry::TABLE_OFFSET, 16);
    assert_eq!(Registry::DATA_START_OFFSET, 16 + 8); // 24

    let sizes = Registry::segment_sizes();
    assert_eq!(sizes, &[4]); // Entry::SIZE = 4

    let size = Registry::compute_account_size(&[5]).unwrap();
    assert_eq!(size, 24 + 5 * 4); // 44
}

#[test]
fn macro_init_segments_wrong_count() {
    let mut data = vec![0u8; 128];
    // OrderBook expects 2 segments, give 1.
    assert!(OrderBook::init_segments(&mut data, &[1]).is_err());
}

#[test]
fn segment_table_data_too_small() {
    let data = [0u8; 60]; // Need at least 64 for OrderBook.
    assert!(OrderBook::segment_table(&data).is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// 6. SegmentIter: ExactSizeIterator
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn iter_exact_size() {
    let mut data = [0u8; 64];
    data[0..4].copy_from_slice(&[1, 0, 0, 0]);
    data[4..8].copy_from_slice(&[2, 0, 0, 0]);

    let desc = SegmentDescriptor::new(0, 2, 4);
    let slice = SegmentSlice::<Entry>::from_descriptor(&data, &desc).unwrap();
    let iter = slice.iter();
    assert_eq!(iter.len(), 2);
}

// ══════════════════════════════════════════════════════════════════════════════
// 7. Edge-case stress tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn descriptor_max_values() {
    let desc = SegmentDescriptor::new(u32::MAX, u16::MAX, u16::MAX);
    assert_eq!(desc.offset(), u32::MAX);
    assert_eq!(desc.count(), u16::MAX);
    assert_eq!(desc.element_size(), u16::MAX);
    // data_len overflows usize on 32-bit but not on 64-bit.
    // byte_range may overflow - that's OK, we just check it doesn't panic.
    let _br = desc.byte_range();
}

#[test]
fn descriptor_byte_range_overflow() {
    // offset near u32::MAX + large count*size should overflow on usize.
    let desc = SegmentDescriptor::new(u32::MAX, u16::MAX, u16::MAX);
    // data_len = 65535 * 65535 = 4294836225, offset = 4294967295.
    // On 64-bit: start + len = ~8.5 billion, fits in usize → Some.
    // On 32-bit: start + len overflows usize → None.
    // We just verify no panic and consistent behavior.
    let br = desc.byte_range();
    if cfg!(target_pointer_width = "64") {
        assert!(br.is_some());
    } else {
        assert!(br.is_none());
    }
}

#[test]
fn table_validate_out_of_order_segments() {
    let mut buf = [0u8; 16];
    // Seg 0 at offset 32 (comes after seg 1).
    buf[0..4].copy_from_slice(&32u32.to_le_bytes());
    buf[4..6].copy_from_slice(&1u16.to_le_bytes());
    buf[6..8].copy_from_slice(&4u16.to_le_bytes());
    // Seg 1 at offset 16 (comes before seg 0).
    buf[8..12].copy_from_slice(&16u32.to_le_bytes());
    buf[12..14].copy_from_slice(&1u16.to_le_bytes());
    buf[14..16].copy_from_slice(&4u16.to_le_bytes());

    let table = SegmentTable::from_bytes(&buf, 2).unwrap();
    assert!(table.validate(100, &[4, 4], 0).is_err());
}

#[test]
fn init_segments_produces_contiguous_layout() {
    // 3 segments with different element sizes.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Big {
        data: [u8; 64],
    }
    unsafe impl Pod for Big {}
    impl FixedLayout for Big { const SIZE: usize = 64; }

    segmented_layout! {
        pub struct MultiSeg, discriminator = 20, version = 1 {
            header: AccountHeader = 16,
        } segments {
            small: Entry = 4,
            medium: Order = 16,
            big: Big = 64,
        }
    }

    assert_eq!(MultiSeg::SEGMENT_COUNT, 3);
    let counts = [5u16, 3, 2];
    let size = MultiSeg::compute_account_size(&counts).unwrap();
    let mut data = vec![0u8; size];
    MultiSeg::init_segments(&mut data, &counts).unwrap();
    MultiSeg::validate_segments(&data).unwrap();

    // Verify segments are contiguous with no gaps.
    let table = MultiSeg::segment_table(&data).unwrap();
    let d0 = table.descriptor(0).unwrap();
    let d1 = table.descriptor(1).unwrap();
    let d2 = table.descriptor(2).unwrap();

    let end0 = d0.offset() as usize + d0.data_len();
    let end1 = d1.offset() as usize + d1.data_len();

    assert_eq!(d1.offset() as usize, end0, "seg1 should start where seg0 ends");
    assert_eq!(d2.offset() as usize, end1, "seg2 should start where seg1 ends");
    assert_eq!(d2.offset() as usize + d2.data_len(), size, "seg2 end should be account end");
}

#[test]
fn empty_segments_validate_ok() {
    let size = OrderBook::compute_account_size(&[0, 0]).unwrap();
    let mut data = vec![0u8; size];
    OrderBook::init_segments(&mut data, &[0, 0]).unwrap();
    OrderBook::validate_segments(&data).unwrap();

    let table = OrderBook::segment_table(&data).unwrap();
    let d0 = table.descriptor(0).unwrap();
    let d1 = table.descriptor(1).unwrap();
    assert_eq!(d0.count(), 0);
    assert_eq!(d1.count(), 0);
    assert_eq!(d0.data_len(), 0);
    assert_eq!(d1.data_len(), 0);
}

#[test]
fn slice_mut_write_all_then_read_all() {
    let counts = [4u16];
    let size = Registry::compute_account_size(&counts).unwrap();
    let mut data = vec![0u8; size];
    Registry::init_segments(&mut data, &counts).unwrap();

    let table = Registry::segment_table(&data).unwrap();
    let desc = table.descriptor(0).unwrap();

    // Write 4 entries.
    {
        let mut slice = SegmentSliceMut::<Entry>::from_descriptor(&mut data, &desc).unwrap();
        for i in 0..4 {
            slice.set(i, &Entry { key: [(i as u8) + 1, 0, 0, 0] }).unwrap();
        }
    }

    // Read all back via iterator.
    let slice = SegmentSlice::<Entry>::from_descriptor(&data, &desc).unwrap();
    let entries: Vec<Entry> = slice.iter().collect();
    assert_eq!(entries.len(), 4);
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry.key[0], (i as u8) + 1);
    }
}

#[test]
fn segmented_layout_id_deterministic() {
    // Same declaration should produce same LAYOUT_ID every time.
    let id1 = OrderBook::SEGMENTED_LAYOUT_ID;
    let id2 = OrderBook::SEGMENTED_LAYOUT_ID;
    assert_eq!(id1, id2);
    // And it should be non-zero (not all zeroes).
    assert_ne!(id1, [0u8; 8]);
}

#[test]
fn different_segment_types_produce_different_ids() {
    assert_ne!(OrderBook::SEGMENTED_LAYOUT_ID, Registry::SEGMENTED_LAYOUT_ID);
}

// ══════════════════════════════════════════════════════════════════════════════
// 8. Named segment index constants
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn named_segment_indices() {
    assert_eq!(OrderBook::bids, 0);
    assert_eq!(OrderBook::asks, 1);
    assert_eq!(Registry::entries, 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// 9. Typed segment accessors: segment() / segment_mut()
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn segment_accessor_read() {
    let counts = [2u16, 1];
    let size = OrderBook::compute_account_size(&counts).unwrap();
    let mut data = vec![0u8; size];
    OrderBook::init_segments(&mut data, &counts).unwrap();

    // Write via low-level.
    let table = OrderBook::segment_table(&data).unwrap();
    let desc = table.descriptor(OrderBook::bids).unwrap();
    let order = Order { price: [7; 8], qty: [99; 8] };
    {
        let mut slice = SegmentSliceMut::<Order>::from_descriptor(&mut data, &desc).unwrap();
        slice.set(0, &order).unwrap();
    }

    // Read via named accessor.
    let bids = OrderBook::segment::<Order>(&data, OrderBook::bids).unwrap();
    assert_eq!(bids.read(0).unwrap(), order);
}

#[test]
fn segment_accessor_mut() {
    let counts = [2u16, 0];
    let size = OrderBook::compute_account_size(&counts).unwrap();
    let mut data = vec![0u8; size];
    OrderBook::init_segments(&mut data, &counts).unwrap();

    let order = Order { price: [5; 8], qty: [10; 8] };
    {
        let mut bids = OrderBook::segment_mut::<Order>(&mut data, OrderBook::bids).unwrap();
        bids.set(0, &order).unwrap();
    }

    let bids = OrderBook::segment::<Order>(&data, OrderBook::bids).unwrap();
    assert_eq!(bids.read(0).unwrap(), order);
}

// ══════════════════════════════════════════════════════════════════════════════
// 10. push / swap_remove
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn push_single_element() {
    // Start with capacity for 3 entries, 0 initial count.
    let counts = [0u16];
    let size = Registry::compute_account_size(&[3]).unwrap();
    let mut data = vec![0u8; size];
    Registry::init_segments(&mut data, &counts).unwrap();

    let e = Entry { key: [0xAA, 0xBB, 0xCC, 0xDD] };
    Registry::push::<Entry>(&mut data, Registry::entries, &e).unwrap();

    let entries = Registry::segment::<Entry>(&data, Registry::entries).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries.read(0).unwrap(), e);
}

#[test]
fn push_multiple_elements() {
    let size = Registry::compute_account_size(&[5]).unwrap();
    let mut data = vec![0u8; size];
    Registry::init_segments(&mut data, &[0]).unwrap();

    for i in 0..5u8 {
        Registry::push::<Entry>(&mut data, Registry::entries, &Entry { key: [i, 0, 0, 0] }).unwrap();
    }

    let entries = Registry::segment::<Entry>(&data, Registry::entries).unwrap();
    assert_eq!(entries.len(), 5);
    for i in 0..5u16 {
        assert_eq!(entries.read(i).unwrap().key[0], i as u8);
    }
}

#[test]
fn push_exceeds_capacity() {
    let size = Registry::compute_account_size(&[2]).unwrap();
    let mut data = vec![0u8; size];
    Registry::init_segments(&mut data, &[0]).unwrap();

    Registry::push::<Entry>(&mut data, Registry::entries, &Entry { key: [1; 4] }).unwrap();
    Registry::push::<Entry>(&mut data, Registry::entries, &Entry { key: [2; 4] }).unwrap();
    // Third push should fail - no room.
    assert!(Registry::push::<Entry>(&mut data, Registry::entries, &Entry { key: [3; 4] }).is_err());
}

#[test]
fn swap_remove_last() {
    let size = Registry::compute_account_size(&[3]).unwrap();
    let mut data = vec![0u8; size];
    Registry::init_segments(&mut data, &[0]).unwrap();

    let a = Entry { key: [1, 0, 0, 0] };
    let b = Entry { key: [2, 0, 0, 0] };
    let c = Entry { key: [3, 0, 0, 0] };
    Registry::push::<Entry>(&mut data, 0, &a).unwrap();
    Registry::push::<Entry>(&mut data, 0, &b).unwrap();
    Registry::push::<Entry>(&mut data, 0, &c).unwrap();

    // Remove last element.
    let removed = Registry::swap_remove::<Entry>(&mut data, 0, 2).unwrap();
    assert_eq!(removed, c);

    let entries = Registry::segment::<Entry>(&data, 0).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries.read(0).unwrap(), a);
    assert_eq!(entries.read(1).unwrap(), b);
}

#[test]
fn swap_remove_middle() {
    let size = Registry::compute_account_size(&[3]).unwrap();
    let mut data = vec![0u8; size];
    Registry::init_segments(&mut data, &[0]).unwrap();

    let a = Entry { key: [1, 0, 0, 0] };
    let b = Entry { key: [2, 0, 0, 0] };
    let c = Entry { key: [3, 0, 0, 0] };
    Registry::push::<Entry>(&mut data, 0, &a).unwrap();
    Registry::push::<Entry>(&mut data, 0, &b).unwrap();
    Registry::push::<Entry>(&mut data, 0, &c).unwrap();

    // Remove middle element - last (c) swaps into index 1.
    let removed = Registry::swap_remove::<Entry>(&mut data, 0, 1).unwrap();
    assert_eq!(removed, b);

    let entries = Registry::segment::<Entry>(&data, 0).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries.read(0).unwrap(), a);
    assert_eq!(entries.read(1).unwrap(), c);
}

#[test]
fn swap_remove_only_element() {
    let size = Registry::compute_account_size(&[1]).unwrap();
    let mut data = vec![0u8; size];
    Registry::init_segments(&mut data, &[0]).unwrap();

    let e = Entry { key: [42, 0, 0, 0] };
    Registry::push::<Entry>(&mut data, 0, &e).unwrap();

    let removed = Registry::swap_remove::<Entry>(&mut data, 0, 0).unwrap();
    assert_eq!(removed, e);

    let entries = Registry::segment::<Entry>(&data, 0).unwrap();
    assert_eq!(entries.len(), 0);
    assert!(entries.is_empty());
}

#[test]
fn swap_remove_out_of_bounds() {
    let size = Registry::compute_account_size(&[2]).unwrap();
    let mut data = vec![0u8; size];
    Registry::init_segments(&mut data, &[0]).unwrap();

    Registry::push::<Entry>(&mut data, 0, &Entry { key: [1; 4] }).unwrap();
    assert!(Registry::swap_remove::<Entry>(&mut data, 0, 1).is_err());
}

#[test]
fn push_then_swap_remove_all() {
    let size = OrderBook::compute_account_size(&[4, 0]).unwrap();
    let mut data = vec![0u8; size];
    // Use init_segments_with_capacity to space offsets by max capacity,
    // enabling safe push without overlapping segment 1.
    OrderBook::init_segments_with_capacity(&mut data, &[4, 0]).unwrap();

    // Push 4 orders into bids.
    for i in 0..4u8 {
        OrderBook::push::<Order>(
            &mut data,
            OrderBook::bids,
            &Order { price: [i; 8], qty: [i + 10; 8] },
        ).unwrap();
    }

    let bids = OrderBook::segment::<Order>(&data, OrderBook::bids).unwrap();
    assert_eq!(bids.len(), 4);

    // Remove all via swap_remove (always remove index 0).
    for _ in 0..4 {
        OrderBook::swap_remove::<Order>(&mut data, OrderBook::bids, 0).unwrap();
    }

    let bids = OrderBook::segment::<Order>(&data, OrderBook::bids).unwrap();
    assert!(bids.is_empty());
}

// ══════════════════════════════════════════════════════════════════════════════
// 11. Validate with min_offset (prefix overlap check)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn validate_rejects_segment_overlapping_prefix() {
    let mut buf = [0u8; 8];
    // Segment at offset 4, count 1, elem_size 4 → occupies [4..8).
    // But if min_offset = 8 (data starts after table), this should fail.
    buf[0..4].copy_from_slice(&4u32.to_le_bytes());
    buf[4..6].copy_from_slice(&1u16.to_le_bytes());
    buf[6..8].copy_from_slice(&4u16.to_le_bytes());

    let table = SegmentTable::from_bytes(&buf, 1).unwrap();
    // With min_offset = 0, it's fine.
    assert!(table.validate(64, &[4], 0).is_ok());
    // With min_offset = 8, segment at offset 4 is rejected.
    assert!(table.validate(64, &[4], 8).is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// 12. Push overlap protection
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn push_rejects_overlap_into_next_segment() {
    // Allocate space for 2 bids and 2 asks, but init with capacity [1, 2].
    // Segment 0 has capacity for 1 element, segment 1 starts right after.
    let size = OrderBook::compute_account_size(&[2, 2]).unwrap();
    let mut data = vec![0u8; size];
    OrderBook::init_segments_with_capacity(&mut data, &[1, 2]).unwrap();

    // First push succeeds (within segment 0's capacity).
    OrderBook::push::<Order>(
        &mut data, OrderBook::bids,
        &Order { price: [1; 8], qty: [2; 8] },
    ).unwrap();

    // Second push would overflow into segment 1: must fail.
    let result = OrderBook::push::<Order>(
        &mut data, OrderBook::bids,
        &Order { price: [3; 8], qty: [4; 8] },
    );
    assert!(result.is_err());
}

#[test]
fn init_segments_with_capacity_enables_push() {
    let size = OrderBook::compute_account_size(&[3, 2]).unwrap();
    let mut data = vec![0u8; size];
    OrderBook::init_segments_with_capacity(&mut data, &[3, 2]).unwrap();

    // Push 3 bids and 2 asks - all within capacity.
    for i in 0..3u8 {
        OrderBook::push::<Order>(
            &mut data, OrderBook::bids,
            &Order { price: [i; 8], qty: [10; 8] },
        ).unwrap();
    }
    for i in 0..2u8 {
        OrderBook::push::<Order>(
            &mut data, OrderBook::asks,
            &Order { price: [i + 100; 8], qty: [20; 8] },
        ).unwrap();
    }

    let bids = OrderBook::segment::<Order>(&data, OrderBook::bids).unwrap();
    assert_eq!(bids.len(), 3);
    let asks = OrderBook::segment::<Order>(&data, OrderBook::asks).unwrap();
    assert_eq!(asks.len(), 2);

    // Verify data integrity: bids[0] should not be corrupted by asks writes.
    let bid0 = bids.read(0).unwrap();
    assert_eq!(bid0.price, [0; 8]);
}

// ══════════════════════════════════════════════════════════════════════════════
// 13. jiminy_interface! version parameter
// ══════════════════════════════════════════════════════════════════════════════

// (See account_abi.rs for interface version tests)
