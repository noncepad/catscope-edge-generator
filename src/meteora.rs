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

pub struct Meteora {
    d_lb_pair: [u8; 8],
    d_position: [u8; 8],
    d_position_v2: [u8; 8],
    d_bin_array: [u8; 8],
    pub program_id: Pubkey,
}

impl GuestFilter for Meteora {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        let prefix = self.d_lb_pair.len();
        let mut i;
        let pubkey_len = std::mem::size_of::<Pubkey>();

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "meteora_edge - 1 - pubkey {}; data len {}",
            id,
            data.len()
        ));

        if match_discriminator(&self.d_lb_pair, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("meteora_edge - 2 - lb_pair - pubkey {};", id));

            // skip mint edges; mint is reachable transitively via reserve token accounts
            // LbPair layout (after 8-byte discriminator):
            //   StaticParameters(32) + VariableParameters(32) + non-pubkey fields(16)
            //   + token_x_mint(32) + token_y_mint(32) = 152 → reserve_x
            //   + reserve_x(32) = 184 → reserve_y
            //   + reserve_y(32) + protocol_fee(16) + _padding_1(32) + reward_infos(288) = 552 → oracle
            {
                i = 152;
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
            // reserve_y at offset 184
            {
                i = 184;
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
            // oracle at offset 552
            {
                i = 552;
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
        } else if match_discriminator(&self.d_position, data) || match_discriminator(&self.d_position_v2, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("meteora_edge - 2 - position - pubkey {};", id));

            // lb_pair at offset 8
            {
                i = prefix;
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
            // owner at offset 8 + 32 = 40
            {
                i = prefix + pubkey_len;
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
        } else if match_discriminator(&self.d_bin_array, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("meteora_edge - 2 - bin_array - pubkey {};", id));

            // BinArray layout: disc(8) + index(i64=8) + version(u8=1) + _padding_1([u8;7]=7)
            // lb_pair at offset 24
            {
                i = 24;
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

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!("meteora_edge - 4 - pubkey {};", id));
        list
    }
}

impl Meteora {
    pub fn new(program_id: &Pubkey) -> Self {
        let d_lb_pair = lb_pair_discriminator();
        let d_position = position_discriminator();
        let d_position_v2 = position_v2_discriminator();
        let d_bin_array = bin_array_discriminator();
        Self {
            program_id: *program_id,
            d_lb_pair,
            d_position,
            d_position_v2,
            d_bin_array,
        }
    }
}

// Meteora DLMM discriminators
pub fn lb_pair_discriminator() -> [u8; 8] {
    [33, 11, 49, 98, 181, 101, 177, 13]
}

pub fn position_discriminator() -> [u8; 8] {
    [170, 188, 143, 228, 122, 64, 248, 208]
}

pub fn position_v2_discriminator() -> [u8; 8] {
    [117, 180, 212, 199, 245, 180, 133, 182]
}

pub fn bin_array_discriminator() -> [u8; 8] {
    [92, 142, 92, 220, 5, 148, 70, 91]
}
