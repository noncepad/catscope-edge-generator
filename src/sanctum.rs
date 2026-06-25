//! Edge filter for the Sanctum S Controller (INF / Infinity) program.
//!
//! IDL: https://github.com/igneous-labs/inf-1.5/blob/master/idl/inf_controller.json
//!
//! # Account layout — PoolStateV1 (176 bytes, version byte == 1)
//!
//! ```text
//! offset  size  field
//! ──────  ────  ─────────────────────────────────────────
//!  0       8    totalSolValue: u64
//!  8       2    tradingProtocolFeeBps: u16
//! 10       2    lpProtocolFeeBps: u16
//! 12       1    version: u8
//! 13       1    isDisabled: u8
//! 14       1    isRebalancing: u8
//! 15       1    padding: u8
//! 16      32    admin: Pubkey
//! 48      32    rebalanceAuthority: Pubkey
//! 80      32    protocolFeeBeneficiary: Pubkey
//! 112     32    pricingProgram: Pubkey
//! 144     32    lpTokenMint: Pubkey
//! ```
//!
//! # Account layout — PoolStateV2 (240 bytes, version byte == 2)
//!
//! ```text
//! offset  size  field
//! ──────  ────  ─────────────────────────────────────────
//!  0       8    totalSolValue: u64
//!  8       4    protocolFeeNanos: u32
//! 12       1    version: u8
//! 13       1    isDisabled: u8
//! 14       1    isRebalancing: u8
//! 15       1    padding: u8
//! 16      32    admin: Pubkey              (same as V1)
//! 48      32    rebalanceAuthority: Pubkey (same as V1)
//! 80      32    protocolFeeBeneficiary: Pubkey (same as V1)
//! 112     32    pricingProgram: Pubkey     (same as V1)
//! 144     32    lpTokenMint: Pubkey        (same as V1)
//! 176     32    rpsAuthority: Pubkey       (V2 only)
//! 208      8    rps: u64
//! 216      8    withheldLamports: u64
//! 224      8    protocolFeeLamports: u64
//! 232      8    lastReleaseSlot: u64
//! ```
//!
//! # Account layout — LstState (80 bytes per entry, no discriminator/length prefix)
//!
//! ```text
//! offset  size  field
//! ──────  ────  ─────────────────────────────────────────
//!  0       1    isInputDisabled: u8
//!  1       1    poolReservesBump: u8
//!  2       1    protocolFeeAccumulatorBump: u8
//!  3       5    padding: [u8; 5]
//!  8       8    solValue: u64
//! 16      32    mint: Pubkey
//! 48      32    solValueCalculator: Pubkey
//! ```
//!
//! # PDAs
//!
//! ```text
//! pool_state_pda     = PDA([b"state"],          program_id)
//! lst_state_list_pda = PDA([b"lst-state-list"],  program_id)
//! ```

use std::collections::VecDeque;

use solana_sdk::{pubkey::Pubkey, system_program::ID as system_id};

#[cfg(any(target_os = "wasi", target_os = "linux"))]
use crate::primitive::wasmimport::HostImport;
use crate::primitive::{
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT},
};

/// Known mainnet program ID for the Sanctum S Controller (INF).
pub const S_CONTROLLER_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx");

const POOL_STATE_V1_SIZE: usize = 176;
const POOL_STATE_V2_SIZE: usize = 240;
const LST_STATE_SIZE: usize = 80;

// PoolState field offsets — identical in V1 and V2 through byte 176.
const OFF_ADMIN: usize = 16;
const OFF_REBALANCE_AUTHORITY: usize = 48;
const OFF_PROTOCOL_FEE_BENEFICIARY: usize = 80;
const OFF_PRICING_PROGRAM: usize = 112;
const OFF_LP_TOKEN_MINT: usize = 144;
const OFF_RPS_AUTHORITY: usize = 176; // V2 only

// LstState field offsets (within each 80-byte entry).
const OFF_LS_MINT: usize = 16;
const OFF_LS_SOL_VALUE_CALCULATOR: usize = 48;

pub struct Sanctum {
    pub program_id: Pubkey,
    /// PDA([b"state"], program_id)
    pool_state_pda: Pubkey,
    /// PDA([b"lst-state-list"], program_id)
    lst_state_list_pda: Pubkey,
}

impl GuestFilter for Sanctum {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        let pk_len = std::mem::size_of::<Pubkey>();
        let read_pk = |off: usize| -> Option<Pubkey> {
            let pk = Pubkey::try_from(&data[off..off + pk_len]).unwrap();
            if pk == system_id { None } else { Some(pk) }
        };

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "sanctum_edge - pubkey {}; data len {}",
            id,
            data.len()
        ));

        if id == self.pool_state_pda
            && (data.len() == POOL_STATE_V1_SIZE || data.len() == POOL_STATE_V2_SIZE)
        {
            let is_v2 = data.len() == POOL_STATE_V2_SIZE;

            // program → pool_state
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
            // pool_state → admin
            if let Some(pk) = read_pk(OFF_ADMIN) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
            }
            // pool_state → lp_token_mint (the INF token)
            if let Some(pk) = read_pk(OFF_LP_TOKEN_MINT) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
            }
            // pool_state → pricing_program
            if let Some(pk) = read_pk(OFF_PRICING_PROGRAM) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
            }
            // pool_state → protocol_fee_beneficiary
            if let Some(pk) = read_pk(OFF_PROTOCOL_FEE_BENEFICIARY) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
            }
            // pool_state → rebalance_authority
            if let Some(pk) = read_pk(OFF_REBALANCE_AUTHORITY) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
            }
            // pool_state → rps_authority (V2 only)
            if is_v2 {
                if let Some(pk) = read_pk(OFF_RPS_AUTHORITY) {
                    list.push_back(FilterEdge {
                        slot: header.slot,
                        weight: WEIGHT_DIRECT,
                        from: id,
                        to: pk,
                    });
                }
            }
        } else if id == self.lst_state_list_pda
            && !data.is_empty()
            && data.len() % LST_STATE_SIZE == 0
        {
            // pool_state → lst_state_list
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.pool_state_pda,
                to: id,
            });

            let n = data.len() / LST_STATE_SIZE;
            for i in 0..n {
                let base = i * LST_STATE_SIZE;
                // lst_state_list → mint
                if data.len() >= base + OFF_LS_MINT + pk_len {
                    let pk = Pubkey::try_from(
                        &data[base + OFF_LS_MINT..base + OFF_LS_MINT + pk_len],
                    )
                    .unwrap();
                    if pk != system_id {
                        list.push_back(FilterEdge {
                            slot: header.slot,
                            weight: WEIGHT_DIRECT,
                            from: id,
                            to: pk,
                        });
                    }
                }
                // lst_state_list → sol_value_calculator
                if data.len() >= base + OFF_LS_SOL_VALUE_CALCULATOR + pk_len {
                    let pk = Pubkey::try_from(
                        &data[base + OFF_LS_SOL_VALUE_CALCULATOR
                            ..base + OFF_LS_SOL_VALUE_CALCULATOR + pk_len],
                    )
                    .unwrap();
                    if pk != system_id {
                        list.push_back(FilterEdge {
                            slot: header.slot,
                            weight: WEIGHT_DIRECT,
                            from: id,
                            to: pk,
                        });
                    }
                }
            }
        }

        list
    }
}

impl Sanctum {
    pub fn new(program_id: &Pubkey) -> Self {
        let pool_state_pda = Pubkey::find_program_address(&[b"state"], program_id).0;
        let lst_state_list_pda =
            Pubkey::find_program_address(&[b"lst-state-list"], program_id).0;
        Self {
            program_id: *program_id,
            pool_state_pda,
            lst_state_list_pda,
        }
    }
}
