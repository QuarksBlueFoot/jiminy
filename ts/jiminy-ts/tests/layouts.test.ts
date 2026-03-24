import { describe, it, expect } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import {
  SPL_TOKEN_SIZE,
  SPL_MINT_SIZE,
  SPL_MULTISIG_SIZE,
  NONCE_ACCOUNT_SIZE,
  STAKE_STATE_SIZE,
  decodeSplTokenAccount,
  decodeSplMint,
  decodeSplMultisig,
  decodeNonceAccount,
  decodeStakeState,
} from '../src/layouts.js';

describe('layouts', () => {
  // Helper: write a u64 LE into buffer at offset
  function writeU64(buf: Uint8Array, offset: number, val: bigint): void {
    const view = new DataView(buf.buffer, buf.byteOffset);
    view.setBigUint64(offset, val, true);
  }

  // Helper: write a u32 LE into buffer at offset
  function writeU32(buf: Uint8Array, offset: number, val: number): void {
    const view = new DataView(buf.buffer, buf.byteOffset);
    view.setUint32(offset, val, true);
  }

  describe('SplTokenAccount', () => {
    it('decodes an initialized token account', () => {
      const data = new Uint8Array(SPL_TOKEN_SIZE);
      // mint at 0..32 — all 1s
      data.fill(1, 0, 32);
      // owner at 32..64 — all 2s
      data.fill(2, 32, 64);
      // amount at 64..72
      writeU64(data, 64, 1000n);
      // delegate_tag at 72 = 1 (Some)
      writeU32(data, 72, 1);
      // delegate at 76..108 — all 3s
      data.fill(3, 76, 108);
      // state at 108 = 1 (Initialized)
      data[108] = 1;
      // is_native_tag at 109 = 0 (None)
      writeU32(data, 109, 0);

      const account = decodeSplTokenAccount(data);
      expect(account.amount).toBe(1000n);
      expect(account.hasDelegate).toBe(true);
      expect(account.state).toBe(1);
      expect(account.isNative).toBe(false);
      expect(account.mint).toBeInstanceOf(PublicKey);
      expect(account.owner).toBeInstanceOf(PublicKey);
    });

    it('rejects short data', () => {
      expect(() => decodeSplTokenAccount(new Uint8Array(100))).toThrow('too short');
    });
  });

  describe('SplMint', () => {
    it('decodes a mint', () => {
      const data = new Uint8Array(SPL_MINT_SIZE);
      writeU32(data, 0, 1); // has mint authority
      data.fill(0xAA, 4, 36); // mint authority
      writeU64(data, 36, 1000000n); // supply
      data[44] = 6; // decimals
      data[45] = 1; // isInitialized
      writeU32(data, 46, 0); // no freeze authority

      const mint = decodeSplMint(data);
      expect(mint.hasMintAuthority).toBe(true);
      expect(mint.supply).toBe(1000000n);
      expect(mint.decimals).toBe(6);
      expect(mint.isInitialized).toBe(true);
      expect(mint.hasFreezeAuthority).toBe(false);
    });

    it('rejects short data', () => {
      expect(() => decodeSplMint(new Uint8Array(50))).toThrow('too short');
    });
  });

  describe('SplMultisig', () => {
    it('decodes a multisig with 3 signers', () => {
      const data = new Uint8Array(SPL_MULTISIG_SIZE);
      data[0] = 2; // m
      data[1] = 3; // n
      data[2] = 1; // isInitialized

      const ms = decodeSplMultisig(data);
      expect(ms.m).toBe(2);
      expect(ms.n).toBe(3);
      expect(ms.isInitialized).toBe(true);
      expect(ms.signers).toHaveLength(3);
      expect(ms.signers[0]).toBeInstanceOf(PublicKey);
    });

    it('rejects short data', () => {
      expect(() => decodeSplMultisig(new Uint8Array(100))).toThrow('too short');
    });
  });

  describe('NonceAccount', () => {
    it('decodes an initialized nonce account', () => {
      const data = new Uint8Array(NONCE_ACCOUNT_SIZE);
      writeU32(data, 0, 1); // version
      writeU32(data, 4, 1); // state = Initialized
      data.fill(0x11, 8, 40); // authority
      data.fill(0x22, 40, 72); // blockhash
      writeU64(data, 72, 5000n); // lamportsPerSignature

      const nonce = decodeNonceAccount(data);
      expect(nonce.version).toBe(1);
      expect(nonce.state).toBe(1);
      expect(nonce.lamportsPerSignature).toBe(5000n);
      expect(nonce.authority).toBeInstanceOf(PublicKey);
    });

    it('rejects short data', () => {
      expect(() => decodeNonceAccount(new Uint8Array(50))).toThrow('too short');
    });
  });

  describe('StakeState', () => {
    it('decodes a delegated stake account', () => {
      const data = new Uint8Array(STAKE_STATE_SIZE);
      writeU32(data, 0, 2); // state = Stake (delegated)
      writeU64(data, 4, 2282880n); // rent_exempt_reserve
      data.fill(0x33, 12, 44); // authorized_staker
      data.fill(0x44, 44, 76); // authorized_withdrawer
      data.fill(0x55, 124, 156); // voter_pubkey
      writeU64(data, 156, 1000000000n); // stake_amount
      writeU64(data, 164, 100n); // activation_epoch
      // deactivation_epoch = u64::MAX
      const max64 = 0xFFFFFFFFFFFFFFFFn;
      writeU64(data, 172, max64);

      const stake = decodeStakeState(data);
      expect(stake.state).toBe(2);
      expect(stake.rentExemptReserve).toBe(2282880n);
      expect(stake.stakeAmount).toBe(1000000000n);
      expect(stake.activationEpoch).toBe(100n);
      expect(stake.deactivationEpoch).toBe(max64);
      expect(stake.voterPubkey).toBeInstanceOf(PublicKey);
    });

    it('rejects short data', () => {
      expect(() => decodeStakeState(new Uint8Array(100))).toThrow('too short');
    });
  });
});
