use std::collections::VecDeque;

use solana_sdk::{
    bpf_loader::ID as bpf_loader_id, pubkey::Pubkey, system_program::ID as system_id,
};
use spl_token::{state::Mint, ID as token_id};

#[cfg(any(target_os = "wasi", target_os = "linux"))]
use super::wasmimport::HostImport;

use super::{
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_PROGRAM, WEIGHT_SPLTOKEN_MINT, WEIGHT_SPLTOKEN_OWNER},
};
#[repr(C, align(8))]
pub struct SolToken {
    program_id: [Pubkey; 3],
}
impl Default for SolToken {
    fn default() -> Self {
        Self {
            program_id: [system_id, token_id, bpf_loader_id],
        }
    }
}
impl GuestFilter for SolToken {
    fn program_id_list(&self) -> Vec<Pubkey> {
        self.program_id.to_vec()
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let pubkey_len = std::mem::size_of::<Pubkey>();
        if header.owner.eq(&system_id) {
            // there is nothing to do;
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("system account - 1__+ -  {}", header.pubkey));
            // DO NOT set edge for system account
        } else if header.owner.eq(&token_id) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!(
                "token - 1 -  id {}; program {}; account data {}",
                header.pubkey,
                header.owner,
                data.len()
            ));
            if 165 <= data.len() && data.len() <= 176 {
                // both edges are incoming, not outgoing.
                let mint = Pubkey::try_from(&data[0..pubkey_len]).unwrap();
                let owner = Pubkey::try_from(&data[pubkey_len..2 * pubkey_len]).unwrap();
                #[cfg(any(target_os = "wasi", target_os = "linux"))]
                HostImport::log(format!(
                    "token edge - 1 -  id {}; owner {}; mint {};",
                    header.pubkey, owner, mint
                ));
                // mint edge; mint->token;
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: mint,
                    to: header.pubkey,
                    weight: WEIGHT_SPLTOKEN_MINT,
                });
                // owner edge; owner->token;
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: owner,
                    to: header.pubkey,
                    weight: WEIGHT_SPLTOKEN_OWNER,
                });
            } else if 82 <= data.len() || data.len() <= std::mem::size_of::<Mint>() {
                #[cfg(any(target_os = "wasi", target_os = "linux"))]
                HostImport::log(format!("mint edge - 1 - mint {}", header.pubkey));
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: token_id,
                    to: header.pubkey,
                    weight: WEIGHT_PROGRAM,
                });
            }
        }
        list
    }
}
