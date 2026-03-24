import { describe, it, expect } from 'vitest';
import { checkDiscriminator, checkLayoutId, checkHeader } from '../src/checks.js';

describe('checks', () => {
  const data = new Uint8Array(56);
  data[0] = 1; // discriminator
  // layout_id at bytes 4-11
  const lid = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89];
  for (let i = 0; i < 8; i++) data[4 + i] = lid[i];

  it('checkDiscriminator passes on match', () => {
    expect(() => checkDiscriminator(data, 1)).not.toThrow();
  });

  it('checkDiscriminator throws on mismatch', () => {
    expect(() => checkDiscriminator(data, 2)).toThrow('Discriminator mismatch');
  });

  it('checkLayoutId passes on match', () => {
    expect(() => checkLayoutId(data, lid)).not.toThrow();
  });

  it('checkLayoutId throws on mismatch', () => {
    const bad = [0, 0, 0, 0, 0, 0, 0, 0];
    expect(() => checkLayoutId(data, bad)).toThrow('Layout ID mismatch');
  });

  it('checkLayoutId throws on wrong length', () => {
    expect(() => checkLayoutId(data, [1, 2, 3])).toThrow('8 bytes');
  });

  it('checkHeader passes on valid data', () => {
    expect(() => checkHeader(data, 1, lid, 56)).not.toThrow();
  });

  it('checkHeader throws on short data', () => {
    expect(() => checkHeader(data.slice(0, 10), 1, lid, 56)).toThrow('too short');
  });

  it('checkHeader throws on wrong disc', () => {
    expect(() => checkHeader(data, 2, lid, 16)).toThrow('Discriminator');
  });

  it('checkHeader throws on wrong layout id', () => {
    expect(() => checkHeader(data, 1, [0, 0, 0, 0, 0, 0, 0, 0], 16)).toThrow('Layout ID');
  });
});
