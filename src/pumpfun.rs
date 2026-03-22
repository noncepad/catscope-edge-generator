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

pub struct Pumpfun {
    d_bonding_curve: [u8; 8],
    d_global: [u8; 8],
    pub program_id: Pubkey,
}

impl GuestFilter for Pumpfun {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        let prefix = self.d_bonding_curve.len();
        let mut i;
        let pubkey_len = std::mem::size_of::<Pubkey>();

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "pumpfun_edge - 1 - pubkey {}; data len {}",
            id,
            data.len()
        ));

        if match_discriminator(&self.d_bonding_curve, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("pumpfun_edge - 2 - bonding_curve - pubkey {};", id));

            // BondingCurve layout after discriminator (8 bytes):
            // virtual_token_reserves: u64 (8 bytes, offset 8)
            // virtual_sol_reserves: u64 (8 bytes, offset 16)
            // real_token_reserves: u64 (8 bytes, offset 24)
            // real_sol_reserves: u64 (8 bytes, offset 32)
            // token_total_supply: u64 (8 bytes, offset 40)
            // complete: bool (1 byte, offset 48)
            // creator: pubkey (32 bytes, offset 49)

            // creator at offset 49
            {
                i = prefix + 8 * 5 + 1; // 8 bytes discriminator + 5 u64s (40 bytes) + 1 bool (1 byte) = 49
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
        } else if match_discriminator(&self.d_global, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("pumpfun_edge - 2 - global - pubkey {};", id));

            // Global state - connect program to global state
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
        }

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!("pumpfun_edge - 4 - pubkey {};", id));
        list
    }
}

impl Pumpfun {
    pub fn new(program_id: &Pubkey) -> Self {
        let d_bonding_curve = bonding_curve_discriminator();
        let d_global = global_discriminator();
        Self {
            program_id: *program_id,
            d_bonding_curve,
            d_global,
        }
    }
}

// Pump.fun discriminators
pub fn bonding_curve_discriminator() -> [u8; 8] {
    [23, 183, 248, 55, 96, 216, 172, 96]
}

pub fn global_discriminator() -> [u8; 8] {
    [167, 232, 232, 177, 200, 108, 114, 127]
}
