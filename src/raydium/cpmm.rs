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

/// Sources: https://docs.raydium.io/products/cpmm/accounts and https://github.com/raydium-io/raydium-cp-swap/blob/master/programs/cp-swap/src/states/pool.rs
/// Anchor program
pub struct RaydiumCPMM {
    len_ammconfig: usize,
    len_poolstate: usize,
    pub program_id: Pubkey,
}
impl RaydiumCPMM {
    pub fn new(program_id: &Pubkey) -> Self {
        Self {
            len_ammconfig: std::mem::size_of::<AmmConfig>(),
            len_poolstate: std::mem::size_of::<PoolState>(),
            program_id: *program_id,
        }
    }
}
impl GuestFilter for RaydiumCPMM {
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
        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "raydium_amm_edge - pubkey {}; data len {}",
            id,
            data.len()
        ));
        if self.len_ammconfig == data.len() {
            let ptr = data.as_ptr() as *const AmmConfig;
            let a = unsafe { &*ptr };
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_PROGRAM,
                from: self.program_id,
                to: id,
            });
            if let Some(pk) = pubkey_not_blank(&a.fund_owner) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pk,
                    to: id,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.protocol_owner) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pk,
                    to: id,
                });
            }
        } else if self.len_poolstate == data.len() {
            let ptr = data.as_ptr() as *const PoolState;
            let a = unsafe { &*ptr };
            if let Some(pk) = pubkey_not_blank(&a.amm_config) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pk,
                    to: id,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.observation_key) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: *pk,
                    to: id,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.pool_creator) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pk,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.token_0_vault) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pk,
                });
            }
            if let Some(pk) = pubkey_not_blank(&a.token_1_vault) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: *pk,
                });
            }
        }

        list
    }
}

// programs/cp-swap/src/states/pool.rs
#[repr(C, packed)]
struct PoolState {
    amm_config: Pubkey,    // binds this pool to an AmmConfig
    pool_creator: Pubkey,  // who ran initialize
    token_0_vault: Pubkey, // == vault0 PDA
    token_1_vault: Pubkey, // == vault1 PDA

    lp_mint: Pubkey,
    token_0_mint: Pubkey,
    token_1_mint: Pubkey,

    token_0_program: Pubkey, // SPL Token or Token-2022 program
    token_1_program: Pubkey,

    observation_key: Pubkey, // == observation PDA
    auth_bump: u8,
    status: u8, // bitmask: deposit | withdraw | swap
    lp_mint_decimals: u8,
    mint_0_decimals: u8,
    mint_1_decimals: u8,

    lp_supply: u64, // mirrors lp_mint supply
    protocol_fees_token_0: u64,
    protocol_fees_token_1: u64,
    fund_fees_token_0: u64,
    fund_fees_token_1: u64,

    open_time: u64, // unix; swaps rejected before this
    recent_epoch: u64,

    // Creator-fee state (added after the original layout):
    creator_fee_on: u8, // 0=BothToken, 1=OnlyToken0, 2=OnlyToken1
    enable_creator_fee: bool,
    padding1: [u8; 6],
    creator_fees_token_0: u64,
    creator_fees_token_1: u64,

    padding: [u64; 28],
}

#[repr(C, packed)]
struct AmmConfig {
    bump: u8,
    disable_create_pool: bool,
    index: u16,             // matches the seed
    trade_fee_rate: u64,    // e.g., 2500 = 0.25%
    protocol_fee_rate: u64, // fraction of trade fee to protocol
    fund_fee_rate: u64,     // fraction of trade fee to fund
    create_pool_fee: u64,   // paid once at init (in SOL or token)
    protocol_owner: Pubkey, // can call CollectProtocolFee
    fund_owner: Pubkey,     // can call CollectFundFee
    creator_fee_rate: u64,  // optional pool-creator fee rate (1/1_000_000 of volume)
    padding: [u64; 15],
}
