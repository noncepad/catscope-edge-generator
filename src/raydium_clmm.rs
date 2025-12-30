use std::collections::VecDeque;

use solana_sdk::{pubkey::Pubkey, system_program};

#[cfg(target_os = "wasi")]
use crate::primitive::wasmimport::HostImport;
use crate::primitive::{
    common::match_discriminator,
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT, WEIGHT_SYMLINK},
};
pub struct Raydium {
    d_amm_config: [u8; 8],
    d_observation_state: [u8; 8],
    d_operation_state: [u8; 8],
    d_personal_position_state: [u8; 8],
    d_pool_state: [u8; 8],
    d_protocol_position_state: [u8; 8],
    d_support_mint_associated: [u8; 8],
    d_tick_array_bitmap_extension: [u8; 8],
    d_tick_array_state: [u8; 8],
    pub program_id: Pubkey,
}
impl GuestFilter for Raydium {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        // all discriminators are the same length
        let prefix = self.d_amm_config.len();
        let mut i = prefix;
        let pubkey_len = std::mem::size_of::<Pubkey>();
        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "raydium_edge - 1 - pubkey {}; data len {}",
            id,
            data.len()
        ));

        if match_discriminator(&self.d_amm_config, data) {
            // protocol owner
            {
                i += 3;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_SYMLINK,
                    from: pubkey,
                    to: id,
                });
            }
            // fund owner
            {
                i += pubkey_len + 18;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pubkey,
                });
            }
        } else if match_discriminator(&self.d_protocol_position_state, data) {
            // pool
            {
                i += 1;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: pubkey,
                    to: id,
                });
            }
        } else if match_discriminator(&self.d_support_mint_associated, data) {
            // skipping mint edges
        } else if match_discriminator(&self.d_tick_array_state, data) {
            // pool
            {
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: pubkey,
                    to: id,
                });
            }
        } else if match_discriminator(&self.d_personal_position_state, data) {
            // nft mint
            {
                i += 1;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_SYMLINK,
                    from: pubkey,
                    to: id,
                });
            }
            // pool
            {
                i += pubkey_len;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: pubkey,
                    to: id,
                });
            }
        } else if match_discriminator(&self.d_observation_state, data) {
            // pool
            {
                i += 11;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: pubkey,
                    to: id,
                });
            }
        } else if match_discriminator(&self.d_tick_array_bitmap_extension, data) {
            // pool id
            {
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: pubkey,
                    to: id,
                });
            }
        } else if match_discriminator(&self.d_pool_state, data) {
            // amm config
            {
                i += 1;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: pubkey,
                    to: id,
                });
            }
            // owner
            {
                i += pubkey_len;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pubkey,
                });
            }
            // token vault 0
            {
                i += pubkey_len + 2 * pubkey_len;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pubkey,
                });
            }
            // token vault 1
            {
                i += pubkey_len;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pubkey,
                });
            }
            // observation account
            {
                i += pubkey_len;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                list.push_back(FilterEdge {
                    slot: header.slot,
                    weight: WEIGHT_DIRECT,
                    from: id,
                    to: pubkey,
                });
            }
        } else if match_discriminator(&self.d_operation_state, data) {
            i += 1;
            // operation owners
            for _k in 0..10 {
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_program::ID {
                    list.push_back(FilterEdge {
                        slot: header.slot,
                        weight: WEIGHT_DIRECT,
                        from: id,
                        to: pubkey,
                    });
                }
                i += pubkey_len;
            }
            // whitelist mints
            for _k in 0..100 {
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_program::ID {
                    list.push_back(FilterEdge {
                        slot: header.slot,
                        weight: WEIGHT_DIRECT,
                        from: id,
                        to: pubkey,
                    });
                }
                i += pubkey_len;
            }
        }

        #[cfg(target_os = "wasi")]
        HostImport::log(format!("raydium_edge - 4 - pubkey {};", id));
        list
    }
}

impl Raydium {
    pub fn new(program_id: &Pubkey) -> Self {
        let d_amm_config = discriminator_amm_config();
        let d_observation_state = discriminator_observation_state();
        let d_operation_state = discriminator_operation_state();
        let d_personal_position_state = discriminator_personal_position_state();
        let d_pool_state = discriminator_pool_state();
        let d_protocol_position_state = discriminator_protocol_position_state();
        let d_support_mint_associated = discriminator_support_mint_associated();
        let d_tick_array_bitmap_extension = discriminator_tick_array_bitmap_extension();
        let d_tick_array_state = discriminator_tick_array_state();
        Self {
            program_id: *program_id,
            d_amm_config,
            d_observation_state,
            d_operation_state,
            d_personal_position_state,
            d_pool_state,
            d_protocol_position_state,
            d_support_mint_associated,
            d_tick_array_bitmap_extension,
            d_tick_array_state,
        }
    }
}

pub fn discriminator_amm_config() -> [u8; 8] {
    [218, 244, 33, 104, 203, 203, 43, 111]
}
pub fn discriminator_observation_state() -> [u8; 8] {
    [122, 174, 197, 53, 129, 9, 165, 132]
}
pub fn discriminator_operation_state() -> [u8; 8] {
    [19, 236, 58, 237, 81, 222, 183, 252]
}
pub fn discriminator_personal_position_state() -> [u8; 8] {
    [70, 111, 150, 126, 230, 15, 25, 117]
}
pub fn discriminator_pool_state() -> [u8; 8] {
    [247, 237, 227, 245, 215, 195, 222, 70]
}
pub fn discriminator_protocol_position_state() -> [u8; 8] {
    [100, 226, 145, 99, 146, 218, 160, 106]
}
pub fn discriminator_support_mint_associated() -> [u8; 8] {
    [134, 40, 183, 79, 12, 112, 162, 53]
}
pub fn discriminator_tick_array_bitmap_extension() -> [u8; 8] {
    [60, 150, 36, 219, 97, 128, 139, 153]
}
pub fn discriminator_tick_array_state() -> [u8; 8] {
    [192, 155, 85, 205, 49, 249, 129, 42]
}
