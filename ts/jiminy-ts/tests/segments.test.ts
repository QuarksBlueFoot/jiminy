import { describe, it, expect } from 'vitest';
import {
  SEGMENT_DESCRIPTOR_SIZE,
  readSegmentDescriptor,
  readSegmentTable,
  readSegmentElements,
} from '../src/segments.js';

describe('segments', () => {
  function makeDescriptor(offset: number, count: number, elementSize: number): Uint8Array {
    const buf = new Uint8Array(SEGMENT_DESCRIPTOR_SIZE);
    const view = new DataView(buf.buffer);
    view.setUint32(0, offset, true);
    view.setUint16(4, count, true);
    view.setUint16(6, elementSize, true);
    return buf;
  }

  it('reads a single descriptor', () => {
    const desc = makeDescriptor(100, 3, 48);
    const result = readSegmentDescriptor(desc, 0);
    expect(result.offset).toBe(100);
    expect(result.count).toBe(3);
    expect(result.elementSize).toBe(48);
  });

  it('throws on short data for descriptor', () => {
    expect(() => readSegmentDescriptor(new Uint8Array(4), 0)).toThrow('too short');
  });

  it('reads a segment table with 2 descriptors', () => {
    const d1 = makeDescriptor(100, 2, 32);
    const d2 = makeDescriptor(164, 3, 48);
    const buf = new Uint8Array(16);
    buf.set(d1, 0);
    buf.set(d2, 8);

    const table = readSegmentTable(buf, 0, 2);
    expect(table).toHaveLength(2);
    expect(table[0].offset).toBe(100);
    expect(table[1].offset).toBe(164);
  });

  it('throws on short data for table', () => {
    expect(() => readSegmentTable(new Uint8Array(10), 0, 2)).toThrow('too short');
  });

  it('reads segment elements', () => {
    // 3 elements of 4 bytes each, starting at offset 10
    const buf = new Uint8Array(22);
    buf[10] = 0xAA;
    buf[14] = 0xBB;
    buf[18] = 0xCC;

    const elements = readSegmentElements(buf, { offset: 10, count: 3, elementSize: 4 });
    expect(elements).toHaveLength(3);
    expect(elements[0][0]).toBe(0xAA);
    expect(elements[1][0]).toBe(0xBB);
    expect(elements[2][0]).toBe(0xCC);
  });

  it('throws if elements exceed data length', () => {
    const buf = new Uint8Array(10);
    expect(() =>
      readSegmentElements(buf, { offset: 5, count: 2, elementSize: 4 }),
    ).toThrow('too short');
  });

  it('returns empty array for zero-count segment', () => {
    const buf = new Uint8Array(20);
    const elements = readSegmentElements(buf, { offset: 0, count: 0, elementSize: 4 });
    expect(elements).toHaveLength(0);
  });

  it('reads descriptor at non-zero offset', () => {
    const buf = new Uint8Array(24);
    const desc = makeDescriptor(200, 5, 16);
    buf.set(desc, 8);
    const result = readSegmentDescriptor(buf, 8);
    expect(result.offset).toBe(200);
    expect(result.count).toBe(5);
    expect(result.elementSize).toBe(16);
  });

  it('reads single-element segment', () => {
    const buf = new Uint8Array(20);
    buf[5] = 0xFF;
    const elements = readSegmentElements(buf, { offset: 5, count: 1, elementSize: 1 });
    expect(elements).toHaveLength(1);
    expect(elements[0][0]).toBe(0xFF);
  });
});
