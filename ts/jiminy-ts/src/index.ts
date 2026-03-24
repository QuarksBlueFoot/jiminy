/**
 * @jiminy/ts — TypeScript runtime for Jiminy zero-copy Solana accounts.
 *
 * Provides:
 * - Jiminy header decoding and validation
 * - Layout ID verification
 * - Segment table reading for segmented accounts
 * - Standard layout readers for common Solana account types
 *
 * @packageDocumentation
 */

export {
  HEADER_SIZE,
  type JiminyHeader,
  decodeHeader,
  readDiscriminator,
  readVersion,
  readFlags,
  readLayoutId,
} from './header.js';

export {
  checkDiscriminator,
  checkLayoutId,
  checkHeader,
} from './checks.js';

export {
  type SegmentDescriptor,
  SEGMENT_DESCRIPTOR_SIZE,
  readSegmentDescriptor,
  readSegmentTable,
  readSegmentElements,
} from './segments.js';

export {
  SPL_TOKEN_SIZE,
  SPL_MINT_SIZE,
  SPL_MULTISIG_SIZE,
  NONCE_ACCOUNT_SIZE,
  STAKE_STATE_SIZE,
  type SplTokenAccount,
  type SplMint,
  type SplMultisig,
  type NonceAccount,
  type StakeState,
  decodeSplTokenAccount,
  decodeSplMint,
  decodeSplMultisig,
  decodeNonceAccount,
  decodeStakeState,
} from './layouts.js';
