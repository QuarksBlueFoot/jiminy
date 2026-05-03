/**
 * Jiminy account header: 16 bytes at the start of every Jiminy-managed account.
 *
 * Layout:
 * ```
 * Byte   Field         Type
 * 0      discriminator u8
 * 1      version       u8
 * 2-3    flags         u16 (LE)
 * 4-11   layout_id     [u8; 8]
 * 12-15  reserved      [u8; 4]
 * ```
 */

/** Header size in bytes. */
export const HEADER_SIZE = 16;

/** Decoded Jiminy account header. */
export interface JiminyHeader {
  /** Account type discriminator (byte 0). */
  discriminator: number;
  /** Schema version (byte 1). */
  version: number;
  /** Flags bitfield (bytes 2-3, little-endian). */
  flags: number;
  /** Deterministic layout fingerprint (bytes 4-11). */
  layoutId: Uint8Array;
  /** Reserved bytes (bytes 12-15, must be zero). */
  reserved: Uint8Array;
}

/**
 * Decode a Jiminy header from raw account data.
 *
 * @param data - Account data buffer (must be at least 16 bytes).
 * @param offset - Byte offset to start reading (default 0).
 * @returns Decoded header.
 * @throws If data is too short.
 */
export function decodeHeader(data: Uint8Array, offset = 0): JiminyHeader {
  if (data.length < offset + HEADER_SIZE) {
    throw new Error(
      `Account data too short for header: need ${offset + HEADER_SIZE} bytes, got ${data.length}`,
    );
  }

  const view = new DataView(data.buffer, data.byteOffset + offset, HEADER_SIZE);

  return {
    discriminator: data[offset],
    version: data[offset + 1],
    flags: view.getUint16(2, true),
    layoutId: data.slice(offset + 4, offset + 12),
    reserved: data.slice(offset + 12, offset + 16),
  };
}

/** Read discriminator (byte 0) from raw data. */
export function readDiscriminator(data: Uint8Array): number {
  if (data.length < 1) throw new Error('Empty account data');
  return data[0];
}

/** Read version (byte 1) from raw data. */
export function readVersion(data: Uint8Array): number {
  if (data.length < 2) throw new Error('Account data too short for version');
  return data[1];
}

/** Read flags (bytes 2-3, LE) from raw data. */
export function readFlags(data: Uint8Array): number {
  if (data.length < 4) throw new Error('Account data too short for flags');
  const view = new DataView(data.buffer, data.byteOffset, data.length);
  return view.getUint16(2, true);
}

/** Read layout_id (bytes 4-11) from raw data. */
export function readLayoutId(data: Uint8Array): Uint8Array {
  if (data.length < 12) throw new Error('Account data too short for layout_id');
  return data.slice(4, 12);
}
