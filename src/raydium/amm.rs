#[cfg(any(target_os = "wasi", target_os = "linux"))]
use crate::primitive::wasmimport::HostImport;
use crate::{
    primitive::{
        guest::GuestFilter,
        header::AccountHeader,
        tree::{FilterEdge, WEIGHT_DIRECT},
    },
    pubkey_not_blank,
};
use solana_sdk::pubkey::Pubkey;
use std::collections::VecDeque;

/// Source: https://github.com/raydium-io/raydium-amm/blob/master/program/src/state.rs and https://docs.raydium.io/products/amm-v4/accounts.
/// Not an Anchor program
pub struct RaydiumAmm {
    len_amminfo: usize,
    pub program_id: Pubkey,
}
impl RaydiumAmm {
    pub fn new(program_id: &Pubkey) -> Self {
        Self {
            len_amminfo: std::mem::size_of::<AmmInfo>(),
            program_id: *program_id,
        }
    }
}

impl GuestFilter for RaydiumAmm {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "raydium_amm_edge - pubkey {}; data len {}",
            id,
            data.len()
        ));
        let a = if self.len_amminfo == data.len() {
            let ptr = data.as_ptr() as *const AmmInfo;
            unsafe { &*ptr }
        } else {
            return list;
        };
        // program → pool
        list.push_back(FilterEdge {
            slot: header.slot,
            weight: WEIGHT_DIRECT,
            from: self.program_id,
            to: id,
        });

        // coin vault
        if let Some(pk) = pubkey_not_blank(&a.coin_vault) {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: *pk,
            });
        }
        if let Some(pk) = pubkey_not_blank(&a.pc_vault) {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: *pk,
            });
        }
        if let Some(pk) = pubkey_not_blank(&a.open_orders) {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: *pk,
            });
        }
        if let Some(pk) = pubkey_not_blank(&a.market) {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: *pk,
            });
        }
        if let Some(pk) = pubkey_not_blank(&a.target_orders) {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: *pk,
            });
        }
        if let Some(pk) = pubkey_not_blank(&a.withdraw_queue) {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: *pk,
            });
        }
        if let Some(pk) = pubkey_not_blank(&a.owner) {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: *pk,
                to: id,
            });
        }

        list
    }
}

#[repr(C, packed)]
struct AmmInfo {
    status: u64, // bitmask: swap/deposit/withdraw/crank enabled
    nonce: u64,  // bump used to derive amm_authority
    order_num: u64,
    depth: u64,
    coin_decimals: u64,
    pc_decimals: u64,
    state: u64, // internal state machine
    reset_flag: u64,
    min_size: u64,
    vol_max_cut_ratio: u64,
    amount_wave: u64,
    coin_lot_size: u64, // mirrors OpenBook market
    pc_lot_size: u64,
    min_price_multiplier: u64,
    max_price_multiplier: u64,
    sys_decimal_value: u64,

    fees: Fees, // trade/protocol/fund fee rates
    state_data: StateData,

    // Pool-owned accounts:
    coin_vault: Pubkey,
    pc_vault: Pubkey,
    coin_vault_mint: Pubkey,
    pc_vault_mint: Pubkey,
    lp_mint: Pubkey,
    open_orders: Pubkey,    // pool's OpenOrders on OpenBook
    market: Pubkey,         // OpenBook market
    market_program: Pubkey, // OpenBook program ID
    target_orders: Pubkey,
    withdraw_queue: Pubkey,
    lp_vault: Pubkey, // = pool_temp_lp
    owner: Pubkey,    // admin (multisig)
    lp_reserve: u64,
    padding: [u64; 3],
}
#[repr(C, packed)]
struct Fees {
    min_separate_numerator: u64,   // 5
    min_separate_denominator: u64, // 10_000
    trade_fee_numerator: u64,      // 25  → used by OpenBook integration
    trade_fee_denominator: u64,    // 10_000
    pnl_numerator: u64,            // 12  → protocol's share OF the swap fee
    pnl_denominator: u64,          // 100 → so 12/100 = 12% of fee, = 0.03% of volume
    swap_fee_numerator: u64,       // 25  → 0.25% gross swap fee
    swap_fee_denominator: u64,     // 10_000
}
#[repr(C, packed)]
struct StateData {
    need_take_pnl_coin: u64,
    need_take_pnl_pc: u64,
    total_pnl_pc: u64,
    total_pnl_coin: u64,
    pool_open_time: u64,
    punish_pc_amount: u64,
    punish_coin_amount: u64,
    orderbook_to_init_time: u64,
    swap_coin_in_amount: u128,
    swap_pc_out_amount: u128,
    swap_acc_pc_fee: u64,
    swap_pc_in_amount: u128,
    swap_coin_out_amount: u128,
    swap_acc_coin_fee: u64,
}
