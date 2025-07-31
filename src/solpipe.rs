use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::VecDeque;

#[cfg(target_os = "wasi")]
use crate::primitive::wasmimport::HostImport;
use crate::{
    primitive::{
        common::{match_discriminator, PUBKEY_LEN, U32_LEN, U64_LEN},
        guest::GuestFilter,
        header::AccountHeader,
        tree::{FilterEdge, WEIGHT_DIRECT, WEIGHT_IS_OUTGOING, WEIGHT_PROGRAM, WEIGHT_SYMLINK},
    },
    DISCRIMINATOR_SIZE,
};
pub struct Solpipe {
    d_controller: [u8; 8],
    d_controller_api: [u8; 8],
    d_pipeline: [u8; 8],
    d_period_ring: [u8; 8],
    d_payout: [u8; 8],
    d_agent: [u8; 8],
    d_bidlist: [u8; 8],
    d_refunds: [u8; 8],
    pub program_id: Pubkey,
}

impl GuestFilter for Solpipe {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        if data.len() < DISCRIMINATOR_SIZE {
            return list;
        }
        let id = header.pubkey;
        let pubkey_len = std::mem::size_of::<Pubkey>();
        let subbuf = &data[DISCRIMINATOR_SIZE..];

        #[cfg(target_os = "wasi")]
        HostImport::log(format!("_edge - 1 - pubkey {};", id));
        if match_discriminator(&self.d_controller, data) {
            let program = FilterEdge {
                slot: header.slot,
                weight: WEIGHT_PROGRAM,
                from: self.program_id,
                to: id,
            };
            let mut i = 1; // start after bump
            let mut length = pubkey_len;
            let admin_pk = Pubkey::try_from(&subbuf[i..(i + length)]).unwrap();
            let admin = FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: id,
                to: admin_pk,
            };
            i += length;
            i += 2 * pubkey_len + 2 * 8; // skip until pc_vault
            let pc_vault_pk = Pubkey::try_from(&subbuf[i..(i + length)]).unwrap();
            //let pcvault = FilterEdge {
            //    slot: header.slot,
            //    weight: WEIGHT_SYMLINK | WEIGHT_IS_OUTGOING,
            //    from: id,
            //    to: pc_vault_pk,
            //};
            i += length;
            let pc_mint_pk = Pubkey::try_from(&subbuf[i..(i + length)]).unwrap();
            #[cfg(target_os = "wasi")]
            HostImport::log(format!(
                "edge - 2 - pubkey {}; controller; admin {}; pcmint {}; pcvault {};",
                id, admin_pk, pc_mint_pk, pc_vault_pk
            ));
            list.push_back(program);
            list.push_back(admin);
        } else if match_discriminator(&self.d_controller_api, data) {
            let i = 1; // start after bump
            let controller_pk = Pubkey::try_from(&subbuf[i..(i + pubkey_len)]).unwrap();
            #[cfg(target_os = "wasi")]
            HostImport::log(format!(
                "edge - 2 - controller_api {}; controller {}",
                id, controller_pk
            ));
            let controller = FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: controller_pk,
                to: id,
            };
            list.push_back(controller);
        } else if match_discriminator(&self.d_pipeline, data) {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("edge - 2 - pubkey {}; pipeline", id));
            let mut i = 0;
            let length = pubkey_len;
            let controller = FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: Pubkey::try_from(&subbuf[i..(i + length)]).unwrap(),
                to: id,
            };
            i += length;
            i += length + 2;
            let admin = FilterEdge {
                slot: header.slot,
                weight: WEIGHT_SYMLINK,
                from: id,
                to: Pubkey::try_from(&subbuf[i..(i + length)]).unwrap(),
            };
            // the pipeline account owns the vault account, so we let the token program add the
            // graph edge
            list.push_back(controller);
            list.push_back(admin);
        } else if match_discriminator(&self.d_payout, data) {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("edge - 2 - pubkey {}; payout", id));
            let i = 2 + pubkey_len;
            let pipeline = Pubkey::try_from(&subbuf[i..(i + pubkey_len)]).unwrap();
            list.push_back(FilterEdge {
                slot: header.slot,
                from: pipeline,
                to: id,
                weight: WEIGHT_DIRECT,
            });
        } else if match_discriminator(&self.d_period_ring, data) {
            let i = 0;
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("edge - 2 - pubkey {}; period_ring", id));
            let pipeline = Pubkey::try_from(&subbuf[i..(i + pubkey_len)]).unwrap();
            list.push_back(FilterEdge {
                slot: header.slot,
                from: pipeline,
                to: id,
                weight: WEIGHT_DIRECT,
            });
        } else if match_discriminator(&self.d_refunds, data) {
            let i = 0;
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("edge - 2 - pubkey {}; refunds", id));
            let pipeline = Pubkey::try_from(&subbuf[i..(i + pubkey_len)]).unwrap();
            list.push_back(FilterEdge {
                slot: header.slot,
                from: pipeline,
                to: id,
                weight: WEIGHT_DIRECT,
            });
            if let Some(claim_count) = Refunds::count(subbuf) {
                for i in 0..claim_count {
                    let claim = Refunds::parse(subbuf, i);
                    if 0 < claim.balance {
                        list.push_back(FilterEdge {
                            slot: header.slot,
                            from: id,
                            to: claim.bidder,
                            weight: WEIGHT_SYMLINK,
                        });
                    }
                }
            }
        } else if match_discriminator(&self.d_bidlist, data) {
            // map from bidder (agent) to payout;
            // parse this for now because the logic is more complicated than before;
            // TODO: do a zerocopy parse.
            let bidlist = match BidList::parse(subbuf) {
                Some(x) => x,
                None => return VecDeque::new(),
            };
            let payout = bidlist.payout;
            #[cfg(target_os = "wasi")]
            HostImport::log(format!(
                "edge - 2 - pubkey {}; bidlist; payout {}",
                id, payout
            ));

            list.push_back(FilterEdge {
                slot: header.slot,
                from: payout,
                to: id,
                weight: WEIGHT_DIRECT,
            });
            for bid in bidlist.book.iter() {
                // map to agent accounts
                if !bid.is_blank {
                    list.push_back(FilterEdge {
                        slot: header.slot,
                        from: Pubkey::from(bid.bidder.to_bytes()),
                        to: id,
                        weight: WEIGHT_SYMLINK,
                    });
                }
            }
        } else if match_discriminator(&self.d_agent, data) {
            // 1;1+32;i=1+2*32+4*8
            let mut i = 1;
            let controller = FilterEdge {
                slot: header.slot,
                from: Pubkey::try_from(&subbuf[i..(i + pubkey_len)]).unwrap(),
                to: id,
                weight: WEIGHT_DIRECT,
            };
            i += pubkey_len;
            let authorizer = FilterEdge {
                slot: header.slot,
                from: id,
                to: Pubkey::try_from(&subbuf[i..(i + pubkey_len)]).unwrap(),
                weight: WEIGHT_DIRECT,
            };
            i += pubkey_len + 4 * 8;
            // this may or may not get replaced with a token owner graph edge
            let vault = FilterEdge {
                slot: header.slot,
                from: id,
                to: Pubkey::try_from(&subbuf[i..(i + pubkey_len)]).unwrap(),
                weight: WEIGHT_DIRECT,
            };
            list.push_back(controller);
            list.push_back(authorizer);
            list.push_back(vault);
        }
        list
    }
}
impl Solpipe {
    pub fn new(program_id: &Pubkey) -> Self {
        let d_controller = controller_discriminator();
        let d_controller_api = controllerapi_discriminator();
        let d_pipeline = pipeline_discriminator();
        let d_payout = payout_discriminator();
        let d_agent = agent_discriminator();
        let d_bidlist = bidlist_discriminator();
        let d_period_ring = periodring_discriminator();
        let d_refunds = refunds_discriminator();
        Self {
            program_id: *program_id,
            d_controller,
            d_controller_api,
            d_pipeline,
            d_payout,
            d_agent,
            d_bidlist,
            d_period_ring,
            d_refunds,
        }
    }
}

pub fn agent_discriminator() -> [u8; 8] {
    [47, 166, 112, 147, 155, 197, 86, 7]
}
pub fn bidlist_discriminator() -> [u8; 8] {
    [233, 127, 13, 29, 123, 209, 192, 79]
}
pub fn refunds_discriminator() -> [u8; 8] {
    [169, 83, 174, 99, 135, 161, 12, 150]
}
pub fn controller_discriminator() -> [u8; 8] {
    [184, 79, 171, 0, 183, 43, 113, 110]
}
pub fn controllerapi_discriminator() -> [u8; 8] {
    [224, 136, 168, 42, 53, 0, 84, 163]
}
pub fn payout_discriminator() -> [u8; 8] {
    [69, 45, 245, 131, 218, 101, 158, 228]
}
pub fn periodring_discriminator() -> [u8; 8] {
    [61, 191, 59, 143, 226, 235, 104, 26]
}
pub fn pipeline_discriminator() -> [u8; 8] {
    [30, 82, 16, 218, 196, 77, 115, 224]
}
pub fn protocol_discriminator() -> [u8; 8] {
    [45, 39, 101, 43, 115, 72, 131, 40]
}
pub fn bidreceipt_discriminator() -> [u8; 8] {
    [186, 150, 141, 135, 59, 122, 39, 99]
}

pub struct Bid {
    pub is_blank: bool,
    // the owner of the bid; (must be owner of token account to which refunds are sent)
    pub bidder: Pubkey,
    // user deposits pc_mint token, deposit account goes up; the user can set deposit=0
    pub deposit: u64,
}

pub struct BidList {
    pub bidding_finished: bool,

    pub payout: Pubkey,

    pub book: Vec<Bid>,

    // total deposits are put here; the numerators are stored in Bids
    pub total_deposits: u64,
}

impl BidList {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 1 + 32 + 4 {
            #[cfg(target_os = "wasi")]
            HostImport::log(String::from("bidlist parse - 1"));
            return None;
        }
        let mut i = 0;
        let bidding_finished = 0 < data[i];
        i += 1;
        let mut payout_data = [0u8; 32];
        payout_data.copy_from_slice(&data[i..i + 32]);
        i += 32;
        let payout = Pubkey::from(payout_data);
        let size = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        i += 4;
        let mut book = Vec::new();
        for _k in 0..size {
            if data.len() < i + 1 + 32 + 8 {
                #[cfg(target_os = "wasi")]
                HostImport::log(format!("bidlist parse - 2 - i {}", i));
                return None;
            }
            let is_blank = 0 < data[i];
            i += 1;

            let mut bidder_data = [0u8; 32];
            bidder_data.copy_from_slice(&data[i..i + 32]);
            let bidder = Pubkey::from(bidder_data);
            i += 32;
            let deposit = u64::from_le_bytes([
                data[i],
                data[i + 1],
                data[i + 2],
                data[i + 3],
                data[i + 4],
                data[i + 5],
                data[i + 6],
                data[i + 7],
            ]);
            i += 8;
            book.push(Bid {
                is_blank,
                bidder,
                deposit,
            });
        }
        if data.len() < i + 8 {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("bidlist parse - 3 - i {}", i));
            return None;
        }
        let total_deposits = u64::from_le_bytes([
            data[i],
            data[i + 1],
            data[i + 2],
            data[i + 3],
            data[i + 4],
            data[i + 5],
            data[i + 6],
            data[i + 7],
        ]);
        Some(Self {
            payout,
            bidding_finished,
            book,
            total_deposits,
        })
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct Refunds {
    pub pipeline: Pubkey,
    pub refunds: Vec<Claim>,
}
const CLAIM_SIZE: usize = PUBKEY_LEN + U64_LEN;
const CLAIM_HEADER_SIZE: usize = PUBKEY_LEN + U32_LEN;
impl Refunds {
    pub fn count(data: &[u8]) -> Option<usize> {
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("edge - Refunds::count - 1 - data {}", data.len()));
        if data.len() < CLAIM_HEADER_SIZE {
            return None;
        }
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("edge - Refunds::count - 2 - data {}", data.len()));
        let nsubbuf = &data[PUBKEY_LEN..(PUBKEY_LEN + U32_LEN)];
        let x: [u8; U32_LEN] = nsubbuf.try_into().unwrap();
        let n = u32::from_le_bytes(x) as usize;
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("edge - Refunds::count - 3 - data {}", data.len()));
        if data.len() < CLAIM_HEADER_SIZE + n * CLAIM_SIZE {
            return None;
        }
        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "edge - Refunds::count - 4 - data {}; count {}",
            data.len(),
            n
        ));
        Some(n)
    }
    // Get the claims; length are not checked
    pub fn parse(data: &[u8], claim_i: usize) -> Claim {
        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "edge - Refunds::parse - 1 - data {}; claim_i {}",
            data.len(),
            claim_i
        ));
        let start = CLAIM_HEADER_SIZE + claim_i * CLAIM_SIZE;
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("edge - Refunds::parse - 2 - data {}", data.len()));
        let finish = start + CLAIM_SIZE;
        let subbuf = &data[start..finish];
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("edge - Refunds::parse - 3 - data {}", data.len()));
        let mut i = 0;
        let bidder = Pubkey::try_from(&subbuf[i..(i + PUBKEY_LEN)]).unwrap();
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("edge - Refunds::parse - 4 - data {}", data.len()));
        i += PUBKEY_LEN;
        let xsubbuf = &subbuf[i..(i + U64_LEN)];
        let x: [u8; U64_LEN] = xsubbuf.try_into().unwrap();
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("edge - Refunds::parse - 5 - data {}", data.len()));
        let balance = u64::from_le_bytes(x);
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("edge - Refunds::parse - 6 - data {}", data.len()));
        Claim { bidder, balance }
    }
}
#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct Claim {
    pub bidder: Pubkey,
    pub balance: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct PeriodRing {
    pub pipeline: Pubkey,
    pub ring: Vec<PeriodWithPayout>,
    // where does the ring buffer start
    pub start: u16,
    // from self.start, how long is the ring
    pub length: u16,
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct PeriodWithPayout {
    pub period: Period,
    pub payout: Pubkey,
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct Period {
    pub is_blank: bool,
    pub bandwidth_allotment: u16,
    pub withhold: u16, // specify how much bandwidth is going to be withheld (unit=1/1000)
    pub start: u64,    // when will this stage start?
    pub length: u64,   // how many slots will this stage last?
}
