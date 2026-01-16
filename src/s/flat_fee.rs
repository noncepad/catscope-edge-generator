// flat_fee.rs — Sanctum Flat Fee Pricing graph filter
//
// Models the persistent pricing configuration owned by the
// flat-fee pricing program.
//
//   FlatFeePricingProgram
//     ├──▶ ProgramState PDA (global)
//     └──▶ FeeAccount PDA (per LST)
//

use std::collections::VecDeque;
use solana_sdk::pubkey::Pubkey;

use crate::primitive::{
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT},
};

/// FlatFeePricing represents the pricing configuration subtree
/// rooted at the flat-fee pricing program executable.
pub struct FlatFeePricing {
    /// Flat-fee pricing program ID (executable account)
    pub program_id: Pubkey,
}

impl GuestFilter for FlatFeePricing {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, _data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;

        if id == self.program_id {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: self.program_id,
            });
            return list;
        }


        if header.owner == self.program_id {
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
        }

        list
    }
}

impl FlatFeePricing {
    pub fn new(program_id: &Pubkey) -> Self {
        Self {
            program_id: *program_id,
        }
    }
}
