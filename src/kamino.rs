use std::collections::VecDeque;

use solana_sdk::{pubkey::Pubkey, system_program};

#[cfg(any(target_os = "wasi", target_os = "linux"))]
use crate::primitive::wasmimport::HostImport;
use crate::{
    primitive::{
        common::match_discriminator,
        guest::GuestFilter,
        header::AccountHeader,
        tree::{FilterEdge, WEIGHT_DIRECT},
    },
    pubkey_not_blank,
};

/// Sources: https://github.com/Kamino-Finance/klend/blob/master/programs/klend/src/state/lending_market.rs
/// Anchor program.
pub struct Kamino {
    len_lending_market: usize,
    len_reserve: usize,
    program_id: Pubkey,
}
impl Kamino {
    pub fn new(program_id: &Pubkey) -> Self {
        Self {
            program_id: *program_id,
            len_lending_market: std::mem::size_of::<LendingMarket>(),
            len_reserve: std::mem::size_of::<Reserve>(),
        }
    }
}

impl GuestFilter for Kamino {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, anchor_data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        if anchor_data.len() < 8 {
            return list;
        }
        let data = &anchor_data[8..];
        if self.len_lending_market == data.len() {
            let ptr = data.as_ptr() as *const LendingMarket;
            let a = unsafe { &*ptr };
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
            if let Some(pk) = pubkey_not_blank(&a.lending_market_owner) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pk,
                    to: id,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.emergency_council) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pk,
                    to: id,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.permissioning_authority) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pk,
                    to: id,
                });
            }
        } else if self.len_reserve == data.len() {
            let ptr = data.as_ptr() as *const Reserve;
            let a = unsafe { &*ptr };
            if let Some(pk) = pubkey_not_blank(&a.lending_market) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pk,
                    to: id,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.farm_debt) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pk,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.farm_collateral) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pk,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.liquidity.supply_vault) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pk,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.liquidity.fee_vault) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pk,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.collateral.supply_vault) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pk,
                });
            }
        }
        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "kamino_edge - pubkey {}; data len {}",
            id,
            anchor_data.len()
        ));

        list
    }
}

#[repr(C)]
struct LendingMarket {
    version: u64,

    bump_seed: u64,

    lending_market_owner: Pubkey,

    lending_market_owner_cached: Pubkey,

    quote_currency: [u8; 32],

    referral_fee_bps: u16,

    emergency_mode: u8,

    autodeleverage_enabled: u8,

    borrow_disabled: u8,

    price_refresh_trigger_to_max_age_pct: u8,

    liquidation_max_debt_close_factor_pct: u8,

    insolvency_risk_unhealthy_ltv_pct: u8,

    min_full_liquidation_value_threshold: u64,

    max_liquidatable_debt_market_value_at_once: u64,

    reserved0: [u8; 8],

    global_allowed_borrow_value: u64,

    emergency_council: Pubkey,

    reserved1: [u8; 8],

    elevation_groups: [ElevationGroup; 32],
    elevation_group_padding: [u64; 90],

    min_net_value_in_obligation_sf: u128,

    min_value_skip_liquidation_ltv_checks: u64,

    name: [u8; 32],

    min_value_skip_liquidation_bf_checks: u64,

    individual_autodeleverage_margin_call_period_secs: u64,

    min_initial_deposit_amount: u64,

    obligation_order_execution_enabled: u8,

    immutable: u8,

    obligation_order_creation_enabled: u8,

    price_triggered_liquidation_disabled: u8,

    mature_reserve_debt_liquidation_enabled: u8,

    obligation_borrow_debt_term_liquidation_enabled: u8,

    borrow_order_creation_enabled: u8,

    borrow_order_execution_enabled: u8,

    proposer_authority: Pubkey,

    min_borrow_order_fill_value: u64,

    withdraw_ticket_issuance_enabled: u8,

    withdraw_ticket_redemption_enabled: u8,

    obligation_borrow_rollover_configuration_enabled: u8,

    obligation_borrow_migration_to_fixed_execution_enabled: u8,

    withdraw_ticket_cancellation_enabled: u8,

    padding2: [u8; 1],

    reserve_rewards_max_apr_bps: u16,

    min_withdraw_queued_liquidity_value: u64,

    fixed_term_rollover_window_duration_seconds: u64,

    open_term_rollover_window_duration_seconds: u64,

    min_partial_rollover_value: u64,

    term_based_full_liquidation_duration_secs: u64,

    permissioning_authority: Pubkey,

    permissioned_ops: u64,

    padding1: [u64; 153],
}

#[repr(C)]
struct ElevationGroup {
    max_liquidation_bonus_bps: u16,
    id: u8,
    ltv_pct: u8,
    liquidation_threshold_pct: u8,
    allow_new_loans: u8,
    max_reserves_as_collateral: u8,

    padding_0: u8,

    debt_reserve: Pubkey,
    padding_1: [u64; 4],
}

#[repr(C)]
struct LastUpdate {
    /// Last slot when updated.
    slot: u64,
    /// True (1) when marked stale, false (0) when slot updated.
    stale: u8,
    /// Price status flags (bitfield).
    price_status: u8,
    placeholder: [u8; 6],
}

#[repr(transparent)]
struct PodU128([u8; 16]);
#[repr(C)]
struct ReserveLiquidity {
    mint_pubkey: Pubkey,
    supply_vault: Pubkey,
    fee_vault: Pubkey,
    total_available_amount: u64,
    /// Total borrowed (scaled fraction).
    borrowed_amount_sf: PodU128,
    /// Market price in quote currency (scaled fraction).
    market_price_sf: PodU128,
    market_price_last_updated_ts: u64,
    mint_decimals: u64,
    deposit_limit_crossed_timestamp: u64,
    borrow_limit_crossed_timestamp: u64,
    /// Cumulative borrow rate (big scaled fraction).
    cumulative_borrow_rate_bsf: BigFractionBytes,
    /// Protocol fees (scaled fraction).
    accumulated_protocol_fees_sf: PodU128,
    /// Referrer fees (scaled fraction).
    accumulated_referrer_fees_sf: PodU128,
    /// Pending referrer fees (scaled fraction).
    pending_referrer_fees_sf: PodU128,
    /// Referral rate (scaled fraction).
    absolute_referral_rate_sf: PodU128,
    token_program: Pubkey,
    /// Reserve rewards budget remaining for distribution
    rewards_amount_available: u64,
    padding2: [u64; 50],
    padding3: [PodU128; 32],
}

#[repr(C)]
struct BigFractionBytes {
    value: [u64; 4],
    padding: [u64; 2],
}

#[repr(C)]
struct Reserve {
    version: u64,
    last_update: LastUpdate,
    lending_market: Pubkey,
    farm_collateral: Pubkey,
    farm_debt: Pubkey,
    liquidity: ReserveLiquidity,
    reserve_liquidity_padding: [u64; 150],
    collateral: ReserveCollateral,
    reserve_collateral_padding: [u64; 150],
    config: ReserveConfig,
    config_padding: [u64; 112],
    borrowed_amount_outside_elevation_group: u64,
    borrowed_amounts_against_this_reserve_in_elevation_groups: [u64; 32],
    withdraw_queue: WithdrawQueue,
    padding: [u64; 204],
}
#[repr(C)]
struct ReserveCollateral {
    mint_pubkey: Pubkey,
    mint_total_supply: u64,
    supply_vault: Pubkey,
    padding1: [PodU128; 32],
    padding2: [PodU128; 32],
}
#[repr(C)]
struct ReserveConfig {
    /// 0 = Active, 1 = Obsolete, 2 = Hidden.
    status: u8,
    padding_deprecated_asset_tier: u8,
    host_fixed_interest_rate_bps: u16,
    min_deleveraging_bonus_bps: u16,
    block_ctoken_usage: u8,
    early_repay_remaining_interest_pct: u8,
    /// Whether the reserve is in emergency mode.
    emergency_mode: u8,
    reserved_1: [u8; 4],
    protocol_order_execution_fee_pct: u8,
    protocol_take_rate_pct: u8,
    protocol_liquidation_fee_pct: u8,
    loan_to_value_pct: u8,
    liquidation_threshold_pct: u8,
    min_liquidation_bonus_bps: u16,
    max_liquidation_bonus_bps: u16,
    bad_debt_liquidation_bonus_bps: u16,
    deleveraging_margin_call_period_secs: u64,
    deleveraging_threshold_decrease_bps_per_day: u64,
    fees: ReserveFees,
    borrow_rate_curve: BorrowRateCurve,
    borrow_factor_pct: u64,
    deposit_limit: u64,
    borrow_limit: u64,
    token_info: TokenInfo,
    deposit_withdrawal_cap: WithdrawalCaps,
    debt_withdrawal_cap: WithdrawalCaps,
    elevation_groups: [u8; 20],
    disable_usage_as_coll_outside_emode: u8,
    utilization_limit_block_borrowing_above_pct: u8,
    autodeleverage_enabled: u8,
    proposer_authority_locked: u8,
    borrow_limit_outside_elevation_group: u64,
    borrow_limit_against_this_collateral_in_elevation_group: [u64; 32],
    deleveraging_bonus_increase_bps_per_day: u64,
    debt_maturity_timestamp: u64,
    debt_term_seconds: u64,
    /// Rewards token amount distributed per slot to depositors
    rewards_amount_per_slot: u64,
    permissioned_ops: u64,
}
#[repr(C)]
struct TokenInfo {
    name: [u8; 32],
    heuristic: PriceHeuristic,
    max_twap_divergence_bps: u64,
    max_age_price_seconds: u64,
    max_age_twap_seconds: u64,
    scope_configuration: ScopeConfiguration,
    switchboard_configuration: SwitchboardConfiguration,
    pyth_configuration: PythConfiguration,
    block_price_usage: u8,
    reserved: [u8; 7],
    _padding: [u64; 19],
}
#[repr(C)]
struct PriceHeuristic {
    lower: u64,
    upper: u64,
    exp: u64,
}
#[repr(C)]
struct ScopeConfiguration {
    price_feed: Pubkey,
    price_chain: [u16; 4],
    twap_chain: [u16; 4],
}
#[repr(C)]
struct SwitchboardConfiguration {
    price_aggregator: Pubkey,
    twap_aggregator: Pubkey,
}
#[repr(C)]
struct PythConfiguration {
    price: Pubkey,
}
#[repr(C)]
struct BorrowRateCurve {
    points: [CurvePoint; 11],
}
#[repr(C)]
struct CurvePoint {
    utilization_rate_bps: u32,
    borrow_rate_bps: u32,
}
#[repr(C)]
struct WithdrawalCaps {
    config_capacity: i64,
    current_total: i64,
    last_interval_start_timestamp: u64,
    config_interval_length_seconds: u64,
}
#[repr(C)]
struct ReserveFees {
    /// Borrow origination fee (scaled fraction).
    origination_fee_sf: u64,
    /// Flash loan fee (u64::MAX = disabled).
    flash_loan_fee_sf: u64,
    padding: [u8; 8],
}
#[repr(C)]
struct WithdrawQueue {
    queued_collateral_amount: u64,
    next_issued_ticket_sequence_number: u64,
    next_withdrawable_ticket_sequence_number: u64,
}
