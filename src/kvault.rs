use std::collections::VecDeque;

use solana_sdk::{pubkey::Pubkey, system_program::ID as system_id};

#[cfg(any(target_os = "wasi", target_os = "linux"))]
use crate::primitive::wasmimport::HostImport;
use crate::primitive::{
    common::match_discriminator,
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT},
};

const VAULT_ALLOCATION_SIZE: usize = 2160;
const ALLOCATION_ARRAY_OFFSET: usize = 312;
const NUM_ALLOCATIONS: usize = 25;

pub struct Kvault {
    d_global_config: [u8; 8],
    d_vault_state: [u8; 8],
    d_reserve_whitelist_entry: [u8; 8],
    pub program_id: Pubkey,
}

impl GuestFilter for Kvault {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        let pubkey_len = std::mem::size_of::<Pubkey>();

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "kvault_edge - pubkey {}; data len {}",
            id,
            data.len()
        ));

        if match_discriminator(&self.d_global_config, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("kvault_edge - global_config - pubkey {};", id));

            // GlobalConfig layout after discriminator (8 bytes):
            // globalAdmin: Pubkey   (32 bytes, offset 8)
            // pendingAdmin: Pubkey  (32 bytes, offset 40)
            // withdrawalPenaltyLamports: u64  (8 bytes, offset 72)
            // withdrawalPenaltyBps: u64       (8 bytes, offset 80)
            // padding: [u8; 944]              (944 bytes, offset 88)

            // program → global_config
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
            // global_config → globalAdmin
            {
                let i = 8;
                if data.len() >= i + pubkey_len {
                    let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                    if pubkey != system_id {
                        list.push_back(FilterEdge {
                            slot: header.slot,
                            weight: WEIGHT_DIRECT,
                            from: pubkey,
                            to: id,
                        });
                    }
                }
            }
        } else if match_discriminator(&self.d_vault_state, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("kvault_edge - vault_state - pubkey {};", id));

            // VaultState layout after discriminator (8 bytes):
            // vaultAdminAuthority: Pubkey      (32 bytes, offset 8)   — skip (shared admin)
            // baseVaultAuthority: Pubkey        (32 bytes, offset 40)  — skip (PDA, SolToken covers via SPLTOKEN_OWNER)
            // baseVaultAuthorityBump: u64        (8 bytes, offset 72)
            // tokenMint: Pubkey                 (32 bytes, offset 80)  — skip (USDC/SOL shared singleton)
            // tokenMintDecimals: u64             (8 bytes, offset 112)
            // tokenVault: Pubkey                (32 bytes, offset 120)
            // tokenProgram: Pubkey              (32 bytes, offset 152) — skip
            // sharesMint: Pubkey                (32 bytes, offset 184)
            // sharesMintDecimals: u64            (8 bytes, offset 216)
            // tokenAvailable: u64                (8 bytes, offset 224)
            // sharesIssued: u64                  (8 bytes, offset 232)
            // availableCrankFunds: u64           (8 bytes, offset 240)
            // unallocatedWeight: u64             (8 bytes, offset 248)
            // performanceFeeBps: u64             (8 bytes, offset 256)
            // managementFeeBps: u64              (8 bytes, offset 264)
            // lastFeeChargeTimestamp: u64        (8 bytes, offset 272)
            // prevAumSf: u128                   (16 bytes, offset 280)
            // pendingFeesSf: u128               (16 bytes, offset 296)
            // vaultAllocationStrategy: [VaultAllocation; 25]  (offset 312)
            //   Each VaultAllocation (2160 bytes):
            //     reserve: Pubkey     (32 bytes, offset +0)  — skip (Klend external, handled by kamino.rs)
            //     ctokenVault: Pubkey (32 bytes, offset +32)

            // program → vault_state
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
            // vault_state → tokenVault
            {
                let i = 120;
                if data.len() >= i + pubkey_len {
                    let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                    if pubkey != system_id {
                        list.push_back(FilterEdge {
                            slot: header.slot,
                            weight: WEIGHT_DIRECT,
                            from: id,
                            to: pubkey,
                        });
                    }
                }
            }
            // vault_state → sharesMint
            if false {
                let i = 184;
                if data.len() >= i + pubkey_len {
                    let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                    if pubkey != system_id {
                        list.push_back(FilterEdge {
                            slot: header.slot,
                            weight: WEIGHT_DIRECT,
                            from: id,
                            to: pubkey,
                        });
                    }
                }
            }
            // vault_state → ctokenVault[i] for each allocation slot
            for n in 0..NUM_ALLOCATIONS {
                let i = ALLOCATION_ARRAY_OFFSET + n * VAULT_ALLOCATION_SIZE + 32;
                if data.len() >= i + pubkey_len {
                    let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                    if pubkey != system_id {
                        list.push_back(FilterEdge {
                            slot: header.slot,
                            weight: WEIGHT_DIRECT,
                            from: id,
                            to: pubkey,
                        });
                    }
                }
            }
        } else if match_discriminator(&self.d_reserve_whitelist_entry, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!(
                "kvault_edge - reserve_whitelist_entry - pubkey {};",
                id
            ));

            // ReserveWhitelistEntry layout after discriminator (8 bytes):
            // tokenMint: Pubkey           (32 bytes, offset 8)  — skip (shared token)
            // reserve: Pubkey             (32 bytes, offset 40)
            // whitelistAddAllocation: u8  (1 byte,   offset 72)
            // whitelistInvest: u8         (1 byte,   offset 73)
            // padding: [u8; 62]           (62 bytes, offset 74)

            // program → reserve_whitelist_entry
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
        }

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!("kvault_edge - done - pubkey {};", id));
        list
    }
}

impl Kvault {
    pub fn new(program_id: &Pubkey) -> Self {
        Self {
            program_id: *program_id,
            d_global_config: global_config_discriminator(),
            d_vault_state: vault_state_discriminator(),
            d_reserve_whitelist_entry: reserve_whitelist_entry_discriminator(),
        }
    }
}

/// sha256("account:GlobalConfig")[..8]
pub fn global_config_discriminator() -> [u8; 8] {
    [149, 8, 156, 202, 160, 252, 176, 217]
}

/// sha256("account:VaultState")[..8]
pub fn vault_state_discriminator() -> [u8; 8] {
    [228, 196, 82, 165, 98, 210, 235, 152]
}

/// sha256("account:ReserveWhitelistEntry")[..8]
pub fn reserve_whitelist_entry_discriminator() -> [u8; 8] {
    [135, 130, 156, 210, 58, 58, 91, 170]
}
