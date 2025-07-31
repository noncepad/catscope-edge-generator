use std::collections::VecDeque;

use solana_sdk::{pubkey::Pubkey, system_program::ID as system_id};

#[cfg(target_os = "wasi")]
use crate::primitive::wasmimport::HostImport;
use crate::primitive::{
    common::match_discriminator,
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT},
};

pub struct Meteora {
    d_lb_pair: [u8; 8],
    d_bin_array: [u8; 8],
    d_bin_array_ext: [u8; 8],
    d_position: [u8; 8],
    d_position_v2: [u8; 8],
    pub program_id: Pubkey,
}

impl GuestFilter for Meteora {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        let pubkey_len = std::mem::size_of::<Pubkey>();
        let mut i;

        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "meteora_edge - 1 - pubkey {}; data len {}",
            id,
            data.len()
        ));
        if match_discriminator(&self.d_lb_pair, data) {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("meteora_edge - 2 - lb_pair - pubkey {};", id));
            
            // program → lb_pair
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });

            // token_0_mint
            {
                i = 88;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }

            // token_1_mint
            {
                i = 120;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // reserve_x
            {
                i = 152;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // reserve_y
            {
                i = 184;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // oracle
            {
                i = 488;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // pre_activation_swap_address
            {
                i = 688;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // base_key
            {
                i = 720;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // creator
            {
                i = 784;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            } else if match_discriminator(&self.d_bin_array, data) {
            {
                i = 24;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey });
                }
            } else if match_discriminator(&self.d_bin_array_ext, data) {
            {
                i = 8;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            } else if match_discriminator(&self.d_position, data) {
            // lb_pair
            {
                i = 8;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // owner
            {
                i = 40;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
        } else if match_discriminator(&self.d_position_v2, data) {
            // lb_pair
            {
                i = 8;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // owner
            {
                i = 40;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // operator
            {
                i = 248;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
            // fee_owner
            {
                i = 281;
                let pubkey = Pubkey::try_from(&data[i..(i + pubkey_len)]).unwrap();
                if pubkey != system_id {
                    list.push_back(FilterEdge { 
                        slot: header.slot, 
                        weight: WEIGHT_DIRECT, 
                        from: id, 
                        to: pubkey 
                    });
                }
            }
        }

        #[cfg(target_os = "wasi")]
        HostImport::log(format!("meteora_edge - 4 - pubkey {};", id));

        list
    }
}

impl Meteora {
    pub fn new(program_id: &Pubkey) -> Self {
        let d_lb_pair = [33, 11, 49, 98, 181, 101, 177, 13];
        let d_bin_array = [92, 142, 92, 220, 5, 148, 70, 181];
        let d_bin_array_ext = [80, 111, 124, 113, 55, 237, 18, 5];
        let d_position = [170, 188, 143, 228, 122, 64, 247, 208];
        let d_position_v2 = [117, 176, 212, 199, 245, 180, 133, 182];

        Self {
            program_id: *program_id,
            d_lb_pair,
            d_bin_array,
            d_bin_array_ext,
            d_position,
            d_position_v2,
        }
    }
}

