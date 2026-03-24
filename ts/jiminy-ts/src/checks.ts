/**
 * Validation helpers for Jiminy account headers.
 */

import { HEADER_SIZE } from './header.js';

/**
 * Check that byte 0 matches the expected discriminator.
 *
 * @param data - Account data buffer.
 * @param expected - Expected discriminator value.
 * @throws If discriminator does not match.
 */
export function checkDiscriminator(data: Uint8Array, expected: number): void {
  if (data.length < 1) {
    throw new Error('Empty account data');
  }
  if (data[0] !== expected) {
    throw new Error(
      `Discriminator mismatch: expected ${expected}, got ${data[0]}`,
    );
  }
}

/**
 * Check that bytes 4-11 match the expected layout ID.
 *
 * @param data - Account data buffer.
 * @param expected - Expected 8-byte layout ID.
 * @throws If layout ID does not match.
 */
export function checkLayoutId(
  data: Uint8Array,
  expected: Uint8Array | number[],
): void {
  if (data.length < 12) {
    throw new Error('Account data too short for layout_id check');
  }
  const exp = expected instanceof Uint8Array ? expected : new Uint8Array(expected);
  if (exp.length !== 8) {
    throw new Error(`Layout ID must be 8 bytes, got ${exp.length}`);
  }
  for (let i = 0; i < 8; i++) {
    if (data[4 + i] !== exp[i]) {
      throw new Error('Layout ID mismatch');
    }
  }
}

/**
 * Combined header check: validates minimum size, discriminator, and layout ID.
 *
 * @param data - Account data buffer.
 * @param discriminator - Expected discriminator value.
 * @param layoutId - Expected 8-byte layout ID.
 * @param minSize - Minimum account data size (default: HEADER_SIZE).
 * @throws If any check fails.
 */
export function checkHeader(
  data: Uint8Array,
  discriminator: number,
  layoutId: Uint8Array | number[],
  minSize: number = HEADER_SIZE,
): void {
  if (data.length < minSize) {
    throw new Error(
      `Account data too short: expected at least ${minSize} bytes, got ${data.length}`,
    );
  }
  checkDiscriminator(data, discriminator);
  checkLayoutId(data, layoutId);
}
