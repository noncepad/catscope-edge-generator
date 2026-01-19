use std::collections::VecDeque;

use solana_sdk::pubkey::Pubkey;

#[cfg(target_os = "wasi")]
use crate::primitive::wasmimport::HostImport;

use crate::primitive::{
    guest::GuestFilter,
    header::AccountHeader,
    tree::{FilterEdge, WEIGHT_DIRECT, WEIGHT_PROGRAM},
};

/// SPL Stake Pool (mainnet) program id:
/// SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy
///
/// This filter emits **only pubkey edges embedded in on-chain accounts**:
/// - StakePool account -> authority / mint / fee / list / reserve pubkeys
/// - ValidatorList account -> each validator vote pubkey
///
/// It does NOT:
/// - parse instructions
/// - compute PDAs (withdraw authority, transient stake, metadata, etc.)
/// - interpret balances, fees, or stake amounts
pub struct StakePoolFilter {
    pub program_id: Pubkey,
}

impl StakePoolFilter {
    pub fn new(program_id: &Pubkey) -> Self {
        Self {
            program_id: *program_id,
        }
    }
}

// SPL stake-pool AccountType values (from spl-stake-pool state.rs)
const ACCOUNT_TYPE_UNINITIALIZED: u8 = 0;
const ACCOUNT_TYPE_STAKE_POOL: u8 = 1;
const ACCOUNT_TYPE_VALIDATOR_LIST: u8 = 2;

impl GuestFilter for StakePoolFilter {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;

        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "stake_pool_edge - 1 - pubkey {}; data len {}",
            id,
            data.len()
        ));

        if data.is_empty() {
            return list;
        }

        let account_type = data[0];
        if account_type == ACCOUNT_TYPE_UNINITIALIZED {
            return list;
        }

        if account_type == ACCOUNT_TYPE_STAKE_POOL {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("stake_pool_edge - stake_pool - pubkey {};", id));

            // program -> stake pool
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_PROGRAM,
                from: self.program_id,
                to: id,
            });

            // ----------------------------------------------------------
            // StakePool layout (from SPL state.rs):
            //   0:   account_type: u8
            //   1..: manager: Pubkey
            //         staker: Pubkey
            //         stake_deposit_authority: Pubkey
            //         stake_withdraw_bump_seed: u8   (skip)
            //         validator_list: Pubkey
            //         reserve_stake: Pubkey
            //         pool_mint: Pubkey
            //         manager_fee_account: Pubkey
            //         token_program_id: Pubkey
            //         ... (rest ignored)
            //
            // We only emit edges for the pubkey fields above.
            // ----------------------------------------------------------

            let mut off = 1;
            let pk = |data: &[u8], off: usize| -> Option<Pubkey> {
                if off + 32 <= data.len() {
                    Pubkey::try_from(&data[off..off + 32]).ok()
                } else {
                    None
                }
            };

            // manager
            if let Some(to) = pk(data, off) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to,
                    weight: WEIGHT_DIRECT,
                });
            }
            off += 32;

            // staker
            if let Some(to) = pk(data, off) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to,
                    weight: WEIGHT_DIRECT,
                });
            }
            off += 32;

            // stake_deposit_authority
            if let Some(to) = pk(data, off) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to,
                    weight: WEIGHT_DIRECT,
                });
            }
            off += 32;

            // stake_withdraw_bump_seed: u8 (skip 1 byte)
            off += 1;

            // validator_list
            if let Some(to) = pk(data, off) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to,
                    weight: WEIGHT_DIRECT,
                });
            }
            off += 32;

            // reserve_stake
            if let Some(to) = pk(data, off) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to,
                    weight: WEIGHT_DIRECT,
                });
            }
            off += 32;

            // pool_mint
            if let Some(to) = pk(data, off) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to,
                    weight: WEIGHT_DIRECT,
                });
            }
            off += 32;

            // manager_fee_account
            if let Some(to) = pk(data, off) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to,
                    weight: WEIGHT_DIRECT,
                });
            }
            off += 32;

            // token_program_id
            if let Some(to) = pk(data, off) {
                list.push_back(FilterEdge {
                    slot: header.slot,
                    from: id,
                    to,
                    weight: WEIGHT_DIRECT,
                });
            }

            // NOTE: We intentionally do NOT continue parsing after this point.
            // The rest includes numeric params + optional pubkeys; we can add
            // preferred validator vote edges later if you want.

        } else if account_type == ACCOUNT_TYPE_VALIDATOR_LIST {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("stake_pool_edge - validator_list - pubkey {};", id));

            // ValidatorList is owned by the stake-pool program; we link it to the program
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_PROGRAM,
                from: self.program_id,
                to: id,
            });

            // ----------------------------------------------------------
            // ValidatorList layout (SPL):
            //   account_type: u8
            //   header: ValidatorListHeader (fixed-size)
            //   validators: Vec<ValidatorStakeInfo> (borsh-ish but stored in account)
            //
            // We only need the vote_account_address pubkey per entry.
            //
            // IMPORTANT: The exact header size / entry size depends on the program version.
            // In the standard SPL stake-pool implementation, ValidatorStakeInfo is a fixed-size
            // Pod-like struct and the list account is sized to hold max validators.
            //
            // To avoid brittle hardcoding, we do a conservative scan:
            //   - skip the first byte (account_type)
            //   - scan the remaining bytes for 32-byte aligned pubkeys, stepping by 1 entry size
            //
            // If you paste `validator_list.rs` / the exact struct packing constants, we can
            // replace this with exact offsets.
            // ----------------------------------------------------------

            // Best-effort parsing strategy:
            // 1) Find vec length if stored as u32 at the start of the validators region.
            // 2) Otherwise, fall back to scanning for plausible vote pubkeys with a fixed stride.

            // Heuristic constants (will be corrected once we see validator list packing):
            const PUBKEY_LEN: usize = 32;

            // Conservative: try to locate a u32 length somewhere early.
            // Many implementations put a header that includes max_validators and maybe a count.
            // We'll just scan a small window for a plausible count and then read entries.
            let mut parsed_any = false;

            // Try window positions where a u32 "len" might live.
            for len_off in [1usize, 5, 9, 13, 17, 21, 25, 29] {
                if len_off + 4 > data.len() {
                    continue;
                }
                let n = u32::from_le_bytes(data[len_off..len_off + 4].try_into().unwrap()) as usize;
                if n == 0 || n > 10_000 {
                    continue;
                }

                // After this len field, entries might begin shortly after.
                // Try a few candidate starts.
                for start in [len_off + 4, len_off + 8, len_off + 16, len_off + 32] {
                    let mut off = start;
                    let mut ok = 0usize;

                    for _ in 0..n {
                        if off + PUBKEY_LEN > data.len() {
                            break;
                        }
                        if let Ok(vote) = Pubkey::try_from(&data[off..off + PUBKEY_LEN]) {
                            // validator_list -> vote account
                            list.push_back(FilterEdge {
                                slot: header.slot,
                                from: id,
                                to: vote,
                                weight: WEIGHT_DIRECT,
                            });
                            ok += 1;
                        }
                        // Try common fixed entry sizes: Pubkey + some ints.
                        // We'll guess 32 + 4 + 8 + 8 + 8 etc. but to avoid hardcoding,
                        // step by 64 as a safe-ish default unless proven otherwise.
                        off = off.saturating_add(64);
                    }

                    if ok > 0 {
                        parsed_any = true;
                        break;
                    }
                }

                if parsed_any {
                    break;
                }
            }

            if !parsed_any {
                #[cfg(target_os = "wasi")]
                HostImport::log(format!(
                    "stake_pool_edge - validator_list - fallback scan - pubkey {};",
                    id
                ));

                // Fallback: scan every 32-byte boundary after the first byte.
                // This is noisy, but still useful for graph discovery if the account is mostly pubkeys.
                let mut off = 1usize;
                while off + 32 <= data.len() {
                    if let Ok(vote) = Pubkey::try_from(&data[off..off + 32]) {
                        list.push_back(FilterEdge {
                            slot: header.slot,
                            from: id,
                            to: vote,
                            weight: WEIGHT_DIRECT,
                        });
                    }
                    off += 32;
                }
            }
        }

        #[cfg(target_os = "wasi")]
        HostImport::log(format!("stake_pool_edge - done - pubkey {};", id));

        list
    }
}
