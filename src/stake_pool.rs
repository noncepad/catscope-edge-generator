use std::collections::VecDeque;

use solana_sdk::pubkey::Pubkey;

#[cfg(target_os = "wasi")]
use crate::primitive::wasmimport::HostImport;

use crate::primitive::{
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT, WEIGHT_PROGRAM},
};

/// SPL Stake Pool program
/// Mainnet: SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy
pub struct StakePool {
    pub program_id: Pubkey,
}

impl StakePool {
    pub fn new(program_id: &Pubkey) -> Self {
        Self {
            program_id: *program_id,
        }
    }
}

/// Account type discriminator (first byte)
const ACCOUNT_TYPE_UNINITIALIZED: u8 = 0;
const ACCOUNT_TYPE_STAKE_POOL: u8 = 1;
const ACCOUNT_TYPE_VALIDATOR_LIST: u8 = 2;

impl GuestFilter for StakePool {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut edges = VecDeque::new();
        let id = header.pubkey;

        if data.is_empty() {
            return edges;
        }

        let account_type = data[0];

        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "stake_pool_edge: pubkey={} type={}",
            id, account_type
        ));

        match account_type {
            ACCOUNT_TYPE_STAKE_POOL => {
                self.handle_stake_pool(header, data, &mut edges);
            }
            ACCOUNT_TYPE_VALIDATOR_LIST => {
                self.handle_validator_list(header, data, &mut edges);
            }
            _ => {}
        }

        edges
    }
}

impl StakePool {
    // ------------------------------------------------------------
    // StakePool account parsing
    // ------------------------------------------------------------
    fn handle_stake_pool(
        &self,
        header: &AccountHeader,
        data: &[u8],
        edges: &mut VecDeque<FilterEdge>,
    ) {
        let id = header.pubkey;

        // Program → StakePool
        edges.push_back(FilterEdge {
            slot: header.slot,
            from: self.program_id,
            to: id,
            weight: WEIGHT_PROGRAM,
        });

        // Layout (from spl-stake-pool state.rs):
        //
        // 0     u8   account_type
        // 1     Pubkey manager
        // 33    Pubkey staker
        // 65    Pubkey stake_deposit_authority
        // 97    u8   stake_withdraw_bump
        // 98    Pubkey validator_list
        // 130   Pubkey reserve_stake
        // 162   Pubkey pool_mint
        // 194   Pubkey manager_fee_account
        // 226   Pubkey token_program_id
        //
        // Everything after this is numeric / config data.

        let mut off = 1;

        let mut read_pubkey = |off: usize| -> Option<Pubkey> {
            if off + 32 <= data.len() {
                Pubkey::try_from(&data[off..off + 32]).ok()
            } else {
                None
            }
        };

        // manager
        if let Some(pk) = read_pubkey(off) {
            edges.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: pk,
                weight: WEIGHT_DIRECT,
            });
        }
        off += 32;

        // staker
        if let Some(pk) = read_pubkey(off) {
            edges.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: pk,
                weight: WEIGHT_DIRECT,
            });
        }
        off += 32;

        // stake_deposit_authority
        if let Some(pk) = read_pubkey(off) {
            edges.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: pk,
                weight: WEIGHT_DIRECT,
            });
        }
        off += 32;

        // bump seed
        off += 1;

        // validator_list
        if let Some(pk) = read_pubkey(off) {
            edges.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: pk,
                weight: WEIGHT_DIRECT,
            });
        }
        off += 32;

        // reserve_stake
        if let Some(pk) = read_pubkey(off) {
            edges.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: pk,
                weight: WEIGHT_DIRECT,
            });
        }
        off += 32;

        // pool_mint
        if let Some(pk) = read_pubkey(off) {
            edges.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: pk,
                weight: WEIGHT_DIRECT,
            });
        }
        off += 32;

        // manager_fee_account
        if let Some(pk) = read_pubkey(off) {
            edges.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: pk,
                weight: WEIGHT_DIRECT,
            });
        }
        off += 32;

        // token_program_id
        if let Some(pk) = read_pubkey(off) {
            edges.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: pk,
                weight: WEIGHT_DIRECT,
            });
        }
    }

    // ------------------------------------------------------------
    // ValidatorList parsing
    // ------------------------------------------------------------
    fn handle_validator_list(
        &self,
        header: &AccountHeader,
        data: &[u8],
        edges: &mut VecDeque<FilterEdge>,
    ) {
        let id = header.pubkey;

        // Program → ValidatorList
        edges.push_back(FilterEdge {
            slot: header.slot,
            from: self.program_id,
            to: id,
            weight: WEIGHT_PROGRAM,
        });

        // Layout:
        // [u8 account_type]
        // [ValidatorListHeader]
        // [u32 vec_len]
        // [ValidatorStakeInfo * vec_len]
        //
        // ValidatorStakeInfo layout (relevant part):
        //   Pubkey vote_account_address (first 32 bytes)

        let mut offset = 1;

        // Skip ValidatorListHeader.
        // In SPL this is fixed-size (32 bytes).
        // Safe to skip 32 here.
        offset += 32;

        if offset + 4 > data.len() {
            return;
        }

        let len = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        offset += 4;

        // Each entry is fixed-size; vote pubkey is always first field.
        // Total entry size in SPL is currently 72 bytes.
        const ENTRY_SIZE: usize = 72;

        for _ in 0..len {
            if offset + 32 > data.len() {
                break;
            }

            if let Ok(vote) = Pubkey::try_from(&data[offset..offset + 32]) {
                edges.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to: vote,
                    weight: WEIGHT_DIRECT,
                });
            }

            offset += ENTRY_SIZE;
        }
    }
}
