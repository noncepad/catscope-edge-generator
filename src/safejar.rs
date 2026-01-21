use std::collections::VecDeque;

use solana_sdk::pubkey::Pubkey;

#[cfg(target_os = "wasi")]
use crate::primitive::wasmimport::HostImport;
use crate::primitive::{
    common::match_discriminator,
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT, WEIGHT_PROGRAM},
};
pub struct Safejar {
    d_controller: [u8; 8],
    d_delegation: [u8; 8],
    pub program_id: Pubkey,
}

impl GuestFilter for Safejar {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        // all discriminators are the same length
        let prefix = self.d_controller.len();
        let pubkey_len = std::mem::size_of::<Pubkey>();
        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "safejar_edge - _1 - pubkey {}; data len {}",
            id,
            data.len()
        ));
        if match_discriminator(&self.d_controller, data) {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("safejar_edge - 2 - controller - pubkey {};", id));
            // program to controller
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_PROGRAM,
                from: self.program_id,
                to: id,
            });
            // controller to owner
            list.push_back(FilterEdge {
                slot: header.slot,
                from: id,
                to: Pubkey::try_from(&data[(prefix + 1)..(prefix + 1 + pubkey_len)]).unwrap(),
                weight: WEIGHT_DIRECT,
            });
        } else if match_discriminator(&self.d_delegation, data) {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("safejar_edge - 3 - delegation - pubkey {};", id));
            // controller to delegation
            list.push_back(FilterEdge {
                slot: header.slot,
                from: Pubkey::try_from(&data[(prefix + 1)..(prefix + 1 + pubkey_len)]).unwrap(),
                to: id,
                weight: WEIGHT_DIRECT,
            });
        }
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("safejar_edge - 4 - pubkey {};", id));
        list
    }
}
impl Safejar {
    pub fn new(program_id: &Pubkey) -> Self {
        let d_controller = controller_discriminator();
        let d_delegation = delegation_discriminator();
        Self {
            program_id: *program_id,
            d_controller,
            d_delegation,
        }
    }
}

pub fn controller_discriminator() -> [u8; 8] {
    [184, 79, 171, 0, 183, 43, 113, 110]
}

pub fn delegation_discriminator() -> [u8; 8] {
    [237, 90, 140, 159, 124, 255, 243, 80]
}

pub fn ruleaccumulator_discriminator() -> [u8; 8] {
    [127, 132, 189, 170, 68, 38, 206, 135]
}

pub fn spendrequest_discriminator() -> [u8; 8] {
    [71, 251, 215, 71, 98, 153, 90, 25]
}
