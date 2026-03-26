/**
 * Segment table reading for Jiminy segmented accounts.
 *
 * Segmented accounts store a fixed core region followed by a segment
 * descriptor table. Each 12-byte descriptor encodes:
 *
 * ```
 * Byte   Field         Type
 * 0-3    offset        u32 (LE) — byte offset from account start
 * 4-5    count         u16 (LE) — number of live elements
 * 6-7    capacity      u16 (LE) — maximum element capacity
 * 8-9    element_size  u16 (LE) — size of each element in bytes
 * 10-11  flags         u16 (LE) — reserved for future use (zero)
 * ```
 */

/** Size of a single segment descriptor in bytes. */
export const SEGMENT_DESCRIPTOR_SIZE = 12;

/** A decoded segment descriptor. */
export interface SegmentDescriptor {
  /** Byte offset from start of account data to the first element. */
  offset: number;
  /** Number of live elements in this segment. */
  count: number;
  /** Maximum element capacity. */
  capacity: number;
  /** Size of each element in bytes. */
  elementSize: number;
  /** Reserved flags (zero). */
  flags: number;
}

/**
 * Read a single segment descriptor at the given position.
 *
 * @param data - Account data buffer.
 * @param pos - Byte offset of the descriptor.
 * @returns Decoded descriptor.
 * @throws If data is too short.
 */
export function readSegmentDescriptor(
  data: Uint8Array,
  pos: number,
): SegmentDescriptor {
  if (data.length < pos + SEGMENT_DESCRIPTOR_SIZE) {
    throw new Error(
      `Account data too short for segment descriptor at offset ${pos}`,
    );
  }
  const view = new DataView(data.buffer, data.byteOffset + pos, SEGMENT_DESCRIPTOR_SIZE);
  return {
    offset: view.getUint32(0, true),
    count: view.getUint16(4, true),
    capacity: view.getUint16(6, true),
    elementSize: view.getUint16(8, true),
    flags: view.getUint16(10, true),
  };
}

/**
 * Read the full segment descriptor table starting after the fixed region.
 *
 * @param data - Account data buffer.
 * @param tableOffset - Byte offset where the segment table begins.
 * @param segmentCount - Number of segment descriptors to read.
 * @returns Array of decoded descriptors.
 * @throws If data is too short for the table.
 */
export function readSegmentTable(
  data: Uint8Array,
  tableOffset: number,
  segmentCount: number,
): SegmentDescriptor[] {
  const tableEnd = tableOffset + segmentCount * SEGMENT_DESCRIPTOR_SIZE;
  if (data.length < tableEnd) {
    throw new Error(
      `Account data too short for segment table: need ${tableEnd} bytes, got ${data.length}`,
    );
  }

  const descriptors: SegmentDescriptor[] = [];
  for (let i = 0; i < segmentCount; i++) {
    descriptors.push(
      readSegmentDescriptor(data, tableOffset + i * SEGMENT_DESCRIPTOR_SIZE),
    );
  }
  return descriptors;
}

/**
 * Extract the raw element data for a segment.
 *
 * @param data - Account data buffer.
 * @param descriptor - The segment descriptor.
 * @returns Array of Uint8Array slices, one per element.
 * @throws If data is too short for the segment's elements.
 */
export function readSegmentElements(
  data: Uint8Array,
  descriptor: SegmentDescriptor,
): Uint8Array[] {
  const endOffset =
    descriptor.offset + descriptor.count * descriptor.elementSize;
  if (data.length < endOffset) {
    throw new Error(
      `Account data too short for segment elements: need ${endOffset} bytes, got ${data.length}`,
    );
  }

  const elements: Uint8Array[] = [];
  for (let i = 0; i < descriptor.count; i++) {
    const start = descriptor.offset + i * descriptor.elementSize;
    elements.push(data.slice(start, start + descriptor.elementSize));
  }
  return elements;
}
