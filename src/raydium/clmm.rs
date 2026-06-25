use std::collections::VecDeque;

use solana_sdk::pubkey::Pubkey;

#[cfg(any(target_os = "wasi", target_os = "linux"))]
use crate::primitive::wasmimport::HostImport;
use crate::{
    primitive::{
        guest::GuestFilter,
        header::AccountHeader,
        tree::{FilterEdge, WEIGHT_DIRECT, WEIGHT_PROGRAM},
    },
    pubkey_not_blank,
};

/// Sources: https://docs.raydium.io/products/clmm/accounts and https://github.com/raydium-io/raydium-clmm/blob/master/programs/amm/src/states/pool.rs
/// Anchor program
pub struct RaydiumCLMM {
    len_poolstate: usize,
    len_ammconfig: usize,
    pub program_id: Pubkey,
}
impl RaydiumCLMM {
    pub fn new(program_id: &Pubkey) -> Self {
        Self {
            program_id: *program_id,
            len_poolstate: std::mem::size_of::<PoolState>(),
            len_ammconfig: std::mem::size_of::<AmmConfig>(),
        }
    }
}
impl GuestFilter for RaydiumCLMM {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, anchor_data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        if anchor_data.len() < 8 {
            return list;
        }
        let data = &anchor_data[8..];
        let id = header.pubkey;
        if self.len_poolstate == data.len() {
            let ptr = data.as_ptr() as *const PoolState;
            let a = unsafe { &*ptr };

            if let Some(pubkey) = pubkey_not_blank(&a.amm_config) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pubkey,
                    to: id,
                });
            }
            if let Some(pubkey) = pubkey_not_blank(&a.owner) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pubkey,
                    to: id,
                });
            }
            if let Some(pubkey) = pubkey_not_blank(&a.token_vault_0) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pubkey,
                });
            }
            if let Some(pubkey) = pubkey_not_blank(&a.token_vault_1) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pubkey,
                });
            }
            if let Some(pubkey) = pubkey_not_blank(&a.observation_key) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pubkey,
                });
            }
        } else if self.len_ammconfig == data.len() {
            let ptr = data.as_ptr() as *const AmmConfig;
            let a = unsafe { &*ptr };
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_PROGRAM,
                from: self.program_id,
                to: id,
            });
            if let Some(pubkey) = pubkey_not_blank(&a.owner) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pubkey,
                    to: id,
                });
            }
            if let Some(pubkey) = pubkey_not_blank(&a.fund_owner) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pubkey,
                    to: id,
                });
            }
        }

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!("raydium_edge - 4 - pubkey {id};"));
        list
    }
}

// programs/amm/src/states/pool.rs
#[repr(C, packed)]
struct PoolState {
    bump: [u8; 1],
    amm_config: Pubkey, // fee tier binding
    owner: Pubkey,      // admin (multisig)
    token_mint_0: Pubkey,
    token_mint_1: Pubkey,
    token_vault_0: Pubkey,
    token_vault_1: Pubkey,
    observation_key: Pubkey,

    mint_decimals_0: u8,
    mint_decimals_1: u8,
    tick_spacing: u16, // inherited from amm_config at init

    liquidity: u128,      // total active (in-range) liquidity
    sqrt_price_x64: u128, // Q64.64 of sqrt(price)
    tick_current: i32,    // current tick index

    padding3: u16,
    padding4: u16,

    // Global fee growth per unit of liquidity, Q64.64.
    fee_growth_global_0_x64: u128,
    fee_growth_global_1_x64: u128,

    // Accrued-but-not-swept protocol fees (per mint).
    protocol_fees_token_0: u64,
    protocol_fees_token_1: u64,

    // Reserved padding for future upgrades.
    padding5: [u128; 4],

    // Status bitmask. Bits 0-5: open-position, decrease-liquidity,
    // collect-fee, collect-reward, swap, limit-order. A set bit disables
    // the corresponding operation.
    status: u8,

    // Fee-collection mode (CollectFeeOn).
    //   0 = FromInput (deduct fee from the swap input — Uniswap-V3 default)
    //   1 = Token0Only (always deduct fee from token0 vault)
    //   2 = Token1Only (always deduct fee from token1 vault)
    fee_on: u8,
    padding: [u8; 6],

    // Live reward streams (up to REWARD_NUM = 3).
    reward_infos: [RewardInfo; 3],

    // Inline bitmap tracking initialized tick-arrays in the primary range.
    tick_array_bitmap: [u64; 16],

    // Reserved padding for future upgrades.
    padding6: [u64; 4],

    fund_fees_token_0: u64,
    fund_fees_token_1: u64,

    open_time: u64, // currently disabled by the program
    recent_epoch: u64,

    // Per-pool dynamic-fee state. Zero-valued unless the pool was
    // created with `enable_dynamic_fee = true` via create_customizable_pool.
    dynamic_fee_info: DynamicFeeInfo,

    // Reserved for future upgrades.
    padding1: [u64; 14],
    padding2: [u64; 32],
}

#[repr(C, packed)]
struct AmmConfig {
    bump: u8,
    index: u16, // uses "amm_config"+u16 seed

    owner: Pubkey,          // admin
    protocol_fee_rate: u32, // fraction of trade fee to protocol, denom 1e6
    trade_fee_rate: u32,    // trade fee in 1e6ths of volume
    tick_spacing: u16,      // default spacing for pools using this config
    fund_fee_rate: u32,     // fraction of trade fee to fund, denom 1e6
    padding_u32: u32,

    fund_owner: Pubkey,
    padding: [u64; 3],
}
#[repr(C, packed)]
struct DynamicFeeInfo {
    filter_period: u16,
    decay_period: u16,
    reduction_factor: u16,
    dynamic_fee_control: u32,
    max_volatility_accumulator: u32,
    tick_spacing_index_reference: i32, // tick-spacing-units; reference for next swap
    volatility_reference: u32,         // running floor for the accumulator
    volatility_accumulator: u32,       // current cumulative volatility (capped)
    last_update_timestamp: u64,
    padding: [u8; 46],
}

#[repr(C, packed)]
struct RewardInfo {
    /// Reward state
    reward_state: u8,
    /// Reward open time
    open_time: u64,
    /// Reward end time
    end_time: u64,
    /// Reward last update time
    last_update_time: u64,
    /// Q64.64 number indicates how many tokens per second are earned per unit of liquidity.
    emissions_per_second_x64: u128,
    /// The total amount of reward emitted
    reward_total_emitted: u64,
    /// The total amount of claimed reward
    reward_claimed: u64,
    /// Reward token mint.
    token_mint: Pubkey,
    /// Reward vault token account.
    token_vault: Pubkey,
    /// The owner that has permission to set reward param
    authority: Pubkey,
    /// Q64.64 number that tracks the total tokens earned per unit of liquidity since the reward
    /// emissions were turned on.
    reward_growth_global_x64: u128,
}
