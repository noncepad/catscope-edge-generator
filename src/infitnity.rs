// infinity.rs — Sanctum “Infinity / S Controller” graph filter
//
//
// Visual map (what this file tries to build):
//
//                           ┌──────────────────────────┐
//                           │  Sanctum Controller Prog │
//                           │  (5ocnV1qiCgaQ...)        │
//                           └───────────┬──────────────┘
//                                       │
//                                       ▼
//                          ┌───────────────────────────┐
//                          │        PoolState           │   ← GLOBAL PDA
//                          │  (derived from seed "state") 
//                          │
//                          │  ───────── (NOT graph edges) ─────────
//                          │  total_sol_value: u64
//                          │  trading_protocol_fee_bps: u16
//                          │  lp_protocol_fee_bps: u16
//                          │  version: u8
//                          │  is_disabled: u8
//                          │  is_rebalancing: u8
//                          │
//                          │  ───────── Pubkeys (GRAPH EDGES) ─────────
//                          │  admin: Pubkey  ─────────────────────────▶
//                          │  rebalance_authority: Pubkey ───────────▶
//                          │  protocol_fee_beneficiary: Pubkey ─────▶
//                          │  pricing_program: Pubkey ─────────────▶
//                          │  lp_token_mint: Pubkey ───────────────▶
//                          │
//                          └───────────┬────────────────┘
//                                      │
//                                      │ (discovered via SPL Token accounts:
//                                      │  token_account.authority == PoolState PDA)
//        ┌─────────────────────────────┼─────────────────────────────┐
//        │                             │                             │
//        ▼                             ▼                             ▼
// ┌────────────────┐        ┌────────────────┐            ┌────────────────┐
// │ Pool Reserve   │        │ Pool Reserve   │            │ Pool Reserve   │
// │ Vault (LST A)  │        │ Vault (LST B)  │   ...      │ Vault (LST N)  │
// │ SPL Token Acct │        │ SPL Token Acct │            │ SPL Token Acct │
// │ authority=     │        │ authority=     │            │ authority=     │
// │ PoolState PDA  │        │ PoolState PDA  │            │ PoolState PDA  │
// └───────┬────────┘        └───────┬────────┘            └───────┬────────┘
//         │                         │                             │
//         │ (token_account.mint)
//         ▼                         ▼                             ▼
//    LST Mint A               LST Mint B                     LST Mint N
//         ▲                         ▲                             ▲
//         │                         │                             │
//         │            (enumeration source of “which mints exist”)
//         └─────────────── LST List (GLOBAL PDA) ──────────────────┘
//                           │
//                           │ (contains list of LST mints)
//                           ▼
//                     (for each LST mint)
//                           │
//                           │ derive / locate calculator using the mint
//                           │ (exact mechanism TBD: PDA vs stored mapping)
//                           ▼
//                    Calculator (per LST mint)
//

use std::collections::VecDeque;

use solana_sdk::pubkey::Pubkey;

#[cfg(target_os = "wasi")]
use crate::primitive::wasmimport::HostImport;

use crate::primitive::{
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT},
};

use spl_token::state::Account as TokenAccount;

pub struct Infinity {
    pub program_id: Pubkey,

    // hardcoded roots
    pub controller: Pubkey,
    pub pool_state: Pubkey,
    pub lst_list: Pubkey,
}

impl GuestFilter for Infinity {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        let pubkey_len = std::mem::size_of::<Pubkey>();

        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "infinity_edge - pubkey {}; data len {}",
            id,
            data.len()
        ));

        // =====================================================
        // Controller (root)
        // =====================================================
        if id == self.controller {
            // program → controller
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });

            // controller → pool_state
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: self.pool_state,
            });

            // controller → lst_list
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: self.lst_list,
            });

            return list;
        }

        // =====================================================
        // PoolState (GLOBAL)
        // =====================================================
        if id == self.pool_state {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("infinity_edge - pool_state {}", id));

            // skip numeric fields:
            // u64 + u16 + u16 + 4 * u8
            let mut i = 8 + 2 + 2 + 4;

            // admin
            {
                let pk = Pubkey::try_from(&data[i..i + pubkey_len]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
                i += pubkey_len;
            }

            // rebalance_authority
            {
                let pk = Pubkey::try_from(&data[i..i + pubkey_len]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
                i += pubkey_len;
            }

            // protocol_fee_beneficiary
            {
                let pk = Pubkey::try_from(&data[i..i + pubkey_len]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
                i += pubkey_len;
            }

            // pricing_program
            {
                let pk = Pubkey::try_from(&data[i..i + pubkey_len]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
                i += pubkey_len;
            }

            // lp_token_mint
            {
                let pk = Pubkey::try_from(&data[i..i + pubkey_len]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pk,
                });
            }

            return list;
        }

        // =====================================================
        // LST List (GLOBAL)
        // =====================================================

        // Observed LST list entry layout (from manual hex inspection):
        //
        //   [ flags / padding / metadata ] 16 bytes
        //   [ Pubkey mint ]                32 bytes
        //   [ Pubkey calculator ]          32 bytes
        //
        // Total per-entry size = 80 bytes

        let pubkey_len = std::mem::size_of::<Pubkey>(); // 32
        let entry_size = 16 + pubkey_len * 2;           // 80 bytes

        let mut i = 0;

        while i + entry_size <= data.len() {
            // skip flags / padding
            i += 16;

            let mint = Pubkey::try_from(&data[i..i + pubkey_len]).unwrap();
            i += pubkey_len;

            let calculator = Pubkey::try_from(&data[i..i + pubkey_len]).unwrap();
            i += pubkey_len;

            // lst_list → mint
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: mint,
            });

            // mint → calculator
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: mint,
                to: calculator,
            });    
        
            return list;
            
        }



        // =====================================================
        // Pool Reserves (SPL Token Accounts) 
        // =====================================================
        if header.owner == spl_token::id() {
            if let Ok(token) = TokenAccount::unpack(data) {
                if token.owner == self.pool_state {
                    // pool_state → reserve vault
                    list.push_back(FilterEdge {
                        slot: header.slot,
                        weight: WEIGHT_DIRECT,
                        from: self.pool_state,
                        to: id,
                    });

                    // reserve vault → mint
                    list.push_back(FilterEdge {
                        slot: header.slot,
                        weight: WEIGHT_DIRECT,
                        from: id,
                        to: token.mint,
                    });
                }
            }
        }

        list
    }
}

impl Infinity {
    pub fn new(
        program_id: &Pubkey,
        controller: &Pubkey,
        pool_state: &Pubkey,
        lst_list: &Pubkey,
    ) -> Self {
        Self {
            program_id: *program_id,
            controller: *controller,
            pool_state: *pool_state,
            lst_list: *lst_list,
        }
    }
}
