use std::collections::VecDeque;

use solana_sdk::{pubkey, pubkey::Pubkey, system_program::ID as system_id};

pub const PUMPSWAP_PROGRAM_ID: Pubkey =
    pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");

#[cfg(any(target_os = "wasi", target_os = "linux"))]
use crate::primitive::wasmimport::HostImport;
use crate::primitive::{
    common::match_discriminator,
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT},
};

pub struct Pumpswap {
    d_pool: [u8; 8],
    d_global_config: [u8; 8],
    d_global_volume_accumulator: [u8; 8],
    pub program_id: Pubkey,
    global_config: Pubkey,
}

impl GuestFilter for Pumpswap {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        let pubkey_len = std::mem::size_of::<Pubkey>();
        let mut i;

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "pumpswap_edge - 1 - pubkey {}; data len {}",
            id,
            data.len()
        ));

        if match_discriminator(&self.d_pool, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("pumpswap_edge - pool - pubkey {};", id));

            // Pool layout after discriminator (8 bytes):
            // pool_bump: u8          (1 byte,  offset 8)
            // index: u16             (2 bytes, offset 9)
            // creator: Pubkey        (32 bytes, offset 11)
            // base_mint: Pubkey      (32 bytes, offset 43)  — skip, available from vault
            // quote_mint: Pubkey     (32 bytes, offset 75)  — skip, WSOL shared across all pools
            // lp_mint: Pubkey        (32 bytes, offset 107)
            // pool_base_token_account: Pubkey  (32 bytes, offset 139)
            // pool_quote_token_account: Pubkey (32 bytes, offset 171)
            // lp_supply: u64         (8 bytes, offset 203)
            // coin_creator: Pubkey   (32 bytes, offset 211) — skip, shared per creator across pools

            // global_config → pool (config is the structural parent of all pools)
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.global_config,
                to: id,
            });
            // pool → lp_mint
            {
                i = 107;
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
            // pool → pool_base_token_account
            {
                i = 139;
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
            // pool → pool_quote_token_account
            {
                i = 171;
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
        } else if match_discriminator(&self.d_global_config, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("pumpswap_edge - global_config - pubkey {};", id));

            // GlobalConfig layout after discriminator (8 bytes):
            // admin: Pubkey                        (32 bytes, offset 8)
            // lp_fee_basis_points: u64             (8 bytes,  offset 40)
            // protocol_fee_basis_points: u64       (8 bytes,  offset 48)
            // disable_flags: u8                    (1 byte,   offset 56)
            // protocol_fee_recipients: [Pubkey; 8] (256 bytes, offset 57)
            // coin_creator_fee_basis_points: u64   (8 bytes,  offset 313)
            // admin_set_coin_creator_authority: Pubkey (32 bytes, offset 321)

            // program → global_config
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
            // global_config → admin
            {
                i = 8;
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
        } else if match_discriminator(&self.d_global_volume_accumulator, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!(
                "pumpswap_edge - global_volume_accumulator - pubkey {};",
                id
            ));

            // GlobalVolumeAccumulator layout after discriminator (8 bytes):
            // start_time: i64          (8 bytes,  offset 8)
            // end_time: i64            (8 bytes,  offset 16)
            // seconds_in_a_day: i64   (8 bytes,  offset 24)
            // mint: Pubkey             (32 bytes, offset 32)
            // total_token_supply: [u64; 30] (240 bytes, offset 64)
            // sol_volumes: [u64; 30]   (240 bytes, offset 304)

            // program → global_volume_accumulator
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
            // global_volume_accumulator → incentive mint
            {
                i = 32;
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

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!("pumpswap_edge - done - pubkey {};", id));
        list
    }
}

impl Pumpswap {
    pub fn new(program_id: &Pubkey) -> Self {
        let (global_config, _) =
            Pubkey::find_program_address(&[b"global_config"], program_id);
        Self {
            program_id: *program_id,
            global_config,
            d_pool: pool_discriminator(),
            d_global_config: global_config_discriminator(),
            d_global_volume_accumulator: global_volume_accumulator_discriminator(),
        }
    }
}

pub fn pool_discriminator() -> [u8; 8] {
    [241, 154, 109, 4, 17, 177, 109, 188]
}

pub fn global_config_discriminator() -> [u8; 8] {
    [149, 8, 156, 202, 160, 252, 176, 217]
}

pub fn global_volume_accumulator_discriminator() -> [u8; 8] {
    [202, 42, 246, 43, 142, 190, 30, 255]
}

