/**
 * Standard layout readers for common Solana account types.
 *
 * Mirrors the Rust layouts in `jiminy-layouts`:
 * - SPL Token Account (165 bytes)
 * - SPL Mint (82 bytes)
 * - SPL Multisig (355 bytes)
 * - Nonce Account (80 bytes)
 * - Stake State (200 bytes)
 *
 * These are **external** account layouts. They do NOT have the Jiminy
 * 16-byte header. They are for reading accounts owned by SPL Token,
 * System, or Stake programs.
 */

import { PublicKey } from '@solana/web3.js';

// ── SPL Token Account ────────────────────────────────────────────────────────

/** SPL Token Account size in bytes. */
export const SPL_TOKEN_SIZE = 165;

/** Decoded SPL Token Account. */
export interface SplTokenAccount {
  /** Mint address. */
  mint: PublicKey;
  /** Owner of this token account. */
  owner: PublicKey;
  /** Token balance. */
  amount: bigint;
  /** Whether a delegate is set. */
  hasDelegate: boolean;
  /** Delegate address (zero if not set). */
  delegate: PublicKey;
  /** Account state: 0=Uninitialized, 1=Initialized, 2=Frozen. */
  state: number;
  /** Whether this is a native (SOL-wrapped) token account. */
  isNative: boolean;
  /** Native SOL amount (valid only if isNative). */
  nativeAmount: bigint;
  /** Delegated amount. */
  delegatedAmount: bigint;
  /** Whether a close authority is set. */
  hasCloseAuthority: boolean;
  /** Close authority address (zero if not set). */
  closeAuthority: PublicKey;
}

/**
 * Decode an SPL Token Account from raw account data.
 *
 * @param data - Raw account data (165 bytes).
 * @returns Decoded token account.
 * @throws If data is too short.
 */
export function decodeSplTokenAccount(data: Uint8Array): SplTokenAccount {
  if (data.length < SPL_TOKEN_SIZE) {
    throw new Error(
      `SPL Token account data too short: expected ${SPL_TOKEN_SIZE} bytes, got ${data.length}`,
    );
  }
  const view = new DataView(data.buffer, data.byteOffset, data.length);
  return {
    mint: new PublicKey(data.slice(0, 32)),
    owner: new PublicKey(data.slice(32, 64)),
    amount: view.getBigUint64(64, true),
    hasDelegate: view.getUint32(72, true) === 1,
    delegate: new PublicKey(data.slice(76, 108)),
    state: data[108],
    isNative: view.getUint32(109, true) === 1,
    nativeAmount: view.getBigUint64(113, true),
    delegatedAmount: view.getBigUint64(121, true),
    hasCloseAuthority: view.getUint32(129, true) === 1,
    closeAuthority: new PublicKey(data.slice(133, 165)),
  };
}

// ── SPL Mint ─────────────────────────────────────────────────────────────────

/** SPL Mint size in bytes. */
export const SPL_MINT_SIZE = 82;

/** Decoded SPL Mint. */
export interface SplMint {
  /** Whether a mint authority is set. */
  hasMintAuthority: boolean;
  /** Mint authority address (zero if not set). */
  mintAuthority: PublicKey;
  /** Total supply. */
  supply: bigint;
  /** Number of decimals. */
  decimals: number;
  /** Whether the mint is initialized. */
  isInitialized: boolean;
  /** Whether a freeze authority is set. */
  hasFreezeAuthority: boolean;
  /** Freeze authority address (zero if not set). */
  freezeAuthority: PublicKey;
}

/**
 * Decode an SPL Mint account from raw account data.
 *
 * @param data - Raw account data (82 bytes).
 * @returns Decoded mint.
 * @throws If data is too short.
 */
export function decodeSplMint(data: Uint8Array): SplMint {
  if (data.length < SPL_MINT_SIZE) {
    throw new Error(
      `SPL Mint data too short: expected ${SPL_MINT_SIZE} bytes, got ${data.length}`,
    );
  }
  const view = new DataView(data.buffer, data.byteOffset, data.length);
  return {
    hasMintAuthority: view.getUint32(0, true) === 1,
    mintAuthority: new PublicKey(data.slice(4, 36)),
    supply: view.getBigUint64(36, true),
    decimals: data[44],
    isInitialized: data[45] !== 0,
    hasFreezeAuthority: view.getUint32(46, true) === 1,
    freezeAuthority: new PublicKey(data.slice(50, 82)),
  };
}

// ── SPL Multisig ─────────────────────────────────────────────────────────────

/** SPL Multisig account size in bytes. */
export const SPL_MULTISIG_SIZE = 355;

/** Decoded SPL Multisig. */
export interface SplMultisig {
  /** Number of signatures required. */
  m: number;
  /** Total number of valid signers. */
  n: number;
  /** Whether the multisig is initialized. */
  isInitialized: boolean;
  /** Signer addresses (up to 11). */
  signers: PublicKey[];
}

/**
 * Decode an SPL Multisig account from raw account data.
 *
 * @param data - Raw account data (355 bytes).
 * @returns Decoded multisig.
 * @throws If data is too short.
 */
export function decodeSplMultisig(data: Uint8Array): SplMultisig {
  if (data.length < SPL_MULTISIG_SIZE) {
    throw new Error(
      `SPL Multisig data too short: expected ${SPL_MULTISIG_SIZE} bytes, got ${data.length}`,
    );
  }
  const m = data[0];
  const n = data[1];
  const isInitialized = data[2] !== 0;
  const signers: PublicKey[] = [];
  for (let i = 0; i < Math.min(n, 11); i++) {
    const start = 3 + i * 32;
    signers.push(new PublicKey(data.slice(start, start + 32)));
  }
  return { m, n, isInitialized, signers };
}

// ── Nonce Account ────────────────────────────────────────────────────────────

/** System nonce account size in bytes. */
export const NONCE_ACCOUNT_SIZE = 80;

/** Decoded system nonce account. */
export interface NonceAccount {
  /** Nonce version. */
  version: number;
  /** Nonce state: 0=Uninitialized, 1=Initialized. */
  state: number;
  /** Authority authorized to advance the nonce. */
  authority: PublicKey;
  /** Stored durable blockhash. */
  blockhash: PublicKey;
  /** Lamports per signature at the time the nonce was stored. */
  lamportsPerSignature: bigint;
}

/**
 * Decode a system nonce account from raw account data.
 *
 * @param data - Raw account data (80 bytes).
 * @returns Decoded nonce account.
 * @throws If data is too short.
 */
export function decodeNonceAccount(data: Uint8Array): NonceAccount {
  if (data.length < NONCE_ACCOUNT_SIZE) {
    throw new Error(
      `Nonce account data too short: expected ${NONCE_ACCOUNT_SIZE} bytes, got ${data.length}`,
    );
  }
  const view = new DataView(data.buffer, data.byteOffset, data.length);
  return {
    version: view.getUint32(0, true),
    state: view.getUint32(4, true),
    authority: new PublicKey(data.slice(8, 40)),
    blockhash: new PublicKey(data.slice(40, 72)),
    lamportsPerSignature: view.getBigUint64(72, true),
  };
}

// ── Stake Account ────────────────────────────────────────────────────────────

/** Stake account size in bytes (fixed prefix covering Meta + Delegation). */
export const STAKE_STATE_SIZE = 200;

/** Decoded stake account (fixed prefix). */
export interface StakeState {
  /**
   * State discriminant:
   * 0=Uninitialized, 1=Initialized, 2=Stake, 3=RewardsPool.
   */
  state: number;
  /** Rent-exempt reserve. */
  rentExemptReserve: bigint;
  /** Authorized staker. */
  authorizedStaker: PublicKey;
  /** Authorized withdrawer. */
  authorizedWithdrawer: PublicKey;
  /** Lockup Unix timestamp. */
  lockupTimestamp: bigint;
  /** Lockup epoch. */
  lockupEpoch: bigint;
  /** Lockup custodian. */
  lockupCustodian: PublicKey;
  /** Voter pubkey (valid when state == 2). */
  voterPubkey: PublicKey;
  /** Delegated stake amount. */
  stakeAmount: bigint;
  /** Activation epoch. */
  activationEpoch: bigint;
  /** Deactivation epoch (u64 max if not deactivating). */
  deactivationEpoch: bigint;
}

/**
 * Decode a stake account from raw account data.
 *
 * @param data - Raw account data (200 bytes minimum).
 * @returns Decoded stake state.
 * @throws If data is too short.
 */
export function decodeStakeState(data: Uint8Array): StakeState {
  if (data.length < STAKE_STATE_SIZE) {
    throw new Error(
      `Stake account data too short: expected ${STAKE_STATE_SIZE} bytes, got ${data.length}`,
    );
  }
  const view = new DataView(data.buffer, data.byteOffset, data.length);
  return {
    state: view.getUint32(0, true),
    rentExemptReserve: view.getBigUint64(4, true),
    authorizedStaker: new PublicKey(data.slice(12, 44)),
    authorizedWithdrawer: new PublicKey(data.slice(44, 76)),
    lockupTimestamp: view.getBigInt64(76, true),
    lockupEpoch: view.getBigUint64(84, true),
    lockupCustodian: new PublicKey(data.slice(92, 124)),
    voterPubkey: new PublicKey(data.slice(124, 156)),
    stakeAmount: view.getBigUint64(156, true),
    activationEpoch: view.getBigUint64(164, true),
    deactivationEpoch: view.getBigUint64(172, true),
  };
}
