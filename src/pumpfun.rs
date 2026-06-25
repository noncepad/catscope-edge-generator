use std::collections::VecDeque;

use solana_sdk::{pubkey, pubkey::Pubkey};

#[cfg(any(target_os = "wasi", target_os = "linux"))]
use crate::primitive::wasmimport::HostImport;
use crate::primitive::{
    common::match_discriminator,
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT},
};

pub const PUMPFUN_PROGRAM_ID: Pubkey =
    pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

pub struct Pumpfun {
    d_global: [u8; 8],
    d_bonding_curve: [u8; 8],
    pub program_id: Pubkey,
    global: Pubkey,
}

impl GuestFilter for Pumpfun {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!(
            "pumpfun_edge - pubkey {}; data len {}",
            id,
            data.len()
        ));

        if match_discriminator(&self.d_global, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("pumpfun_edge - global - pubkey {};", id));

            // Global layout after discriminator (8 bytes):
            // initial_virtual_token_reserves: u64  (8 bytes, offset 8)
            // initial_virtual_sol_reserves: u64    (8 bytes, offset 16)
            // initial_real_token_reserves: u64     (8 bytes, offset 24)
            // token_total_supply: u64              (8 bytes, offset 32)
            // fee_basis_points: u64                (8 bytes, offset 40)
            // No pubkey fields — no outgoing edges needed.

            // program → global
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
        } else if match_discriminator(&self.d_bonding_curve, data) {
            #[cfg(any(target_os = "wasi", target_os = "linux"))]
            HostImport::log(format!("pumpfun_edge - bonding_curve - pubkey {};", id));

            // BondingCurve layout after discriminator (8 bytes):
            // virtual_token_reserves: u64  (8 bytes, offset 8)
            // virtual_quote_reserves: u64  (8 bytes, offset 16)
            // real_token_reserves: u64     (8 bytes, offset 24)
            // real_quote_reserves: u64     (8 bytes, offset 32)
            // token_total_supply: u64      (8 bytes, offset 40)
            // complete: bool               (1 byte,  offset 48)
            // creator: Pubkey              (32 bytes, offset 49) — skip (external wallet, one per deployer)
            // is_mayhem_mode: bool         (1 byte,  offset 81)
            // is_cashback_coin: bool       (1 byte,  offset 82)
            // quote_mint: Pubkey           (32 bytes, offset 83) — skip (WSOL for most, shared singleton)
            //
            // Note: base_mint is NOT stored in BondingCurve data — the curve is a PDA derived
            // FROM the mint. The associated_bonding_curve token vault is discovered automatically
            // by SolToken via WEIGHT_SPLTOKEN_OWNER (bonding curve PDA owns it).

            // global → bonding_curve (Global PDA is the structural parent of all curves)
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.global,
                to: id,
            });
        }

        #[cfg(any(target_os = "wasi", target_os = "linux"))]
        HostImport::log(format!("pumpfun_edge - done - pubkey {};", id));
        list
    }
}

impl Pumpfun {
    pub fn new(program_id: &Pubkey) -> Self {
        let (global, _) = Pubkey::find_program_address(&[b"global"], program_id);
        Self {
            program_id: *program_id,
            global,
            d_global: global_discriminator(),
            d_bonding_curve: bonding_curve_discriminator(),
        }
    }
}

/// sha256("account:Global")[..8]
pub fn global_discriminator() -> [u8; 8] {
    [167, 232, 232, 177, 200, 108, 114, 127]
}

/// sha256("account:BondingCurve")[..8]
pub fn bonding_curve_discriminator() -> [u8; 8] {
    [23, 183, 248, 55, 96, 216, 172, 96]
}
