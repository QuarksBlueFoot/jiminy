import { describe, it, expect } from 'vitest';
import {
  HEADER_SIZE,
  decodeHeader,
  readDiscriminator,
  readVersion,
  readFlags,
  readLayoutId,
} from '../src/header.js';

describe('header', () => {
  function makeHeader(disc: number, ver: number, flags: number, layoutId: number[], reserved: number[]): Uint8Array {
    const buf = new Uint8Array(HEADER_SIZE);
    buf[0] = disc;
    buf[1] = ver;
    const view = new DataView(buf.buffer);
    view.setUint16(2, flags, true);
    for (let i = 0; i < 8; i++) buf[4 + i] = layoutId[i];
    for (let i = 0; i < 4; i++) buf[12 + i] = reserved[i];
    return buf;
  }

  const lid = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89];
  const res = [0, 0, 0, 0];
  const header = makeHeader(1, 2, 0x0003, lid, res);

  it('decodes a valid header', () => {
    const h = decodeHeader(header);
    expect(h.discriminator).toBe(1);
    expect(h.version).toBe(2);
    expect(h.flags).toBe(3);
    expect(Array.from(h.layoutId)).toEqual(lid);
    expect(Array.from(h.reserved)).toEqual(res);
  });

  it('throws on short data', () => {
    expect(() => decodeHeader(new Uint8Array(15))).toThrow('too short');
  });

  it('reads discriminator', () => {
    expect(readDiscriminator(header)).toBe(1);
  });

  it('reads version', () => {
    expect(readVersion(header)).toBe(2);
  });

  it('reads flags', () => {
    expect(readFlags(header)).toBe(3);
  });

  it('reads layout id', () => {
    expect(Array.from(readLayoutId(header))).toEqual(lid);
  });

  it('throws on empty data for readDiscriminator', () => {
    expect(() => readDiscriminator(new Uint8Array(0))).toThrow();
  });

  it('decodes header at non-zero offset', () => {
    const buf = new Uint8Array(20);
    buf[4] = 5; // disc at offset 4
    buf[5] = 3; // version
    const h = decodeHeader(buf, 4);
    expect(h.discriminator).toBe(5);
    expect(h.version).toBe(3);
  });

  it('reads flags = 0 as zero', () => {
    const h = makeHeader(0, 0, 0, [0,0,0,0,0,0,0,0], [0,0,0,0]);
    expect(readFlags(h)).toBe(0);
  });

  it('reads max flags value', () => {
    const h = makeHeader(0, 0, 0xFFFF, [0,0,0,0,0,0,0,0], [0,0,0,0]);
    expect(readFlags(h)).toBe(0xFFFF);
  });

  it('header values are independent', () => {
    const h1 = makeHeader(1, 1, 0, lid, res);
    const h2 = makeHeader(2, 2, 0, [0,0,0,0,0,0,0,0], res);
    expect(readDiscriminator(h1)).not.toBe(readDiscriminator(h2));
  });
});
