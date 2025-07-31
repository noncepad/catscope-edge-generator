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
pub struct Orca {
    d_whirlpool: [u8; 8],
    d_whirlpoolconfig: [u8; 8],
    d_tickarray: [u8; 8],
    pub program_id: Pubkey,
}
impl GuestFilter for Orca {
    fn program_id_list(&self) -> Vec<Pubkey> {
        vec![self.program_id]
    }

    fn edge(&self, header: &AccountHeader, data: &[u8]) -> VecDeque<FilterEdge> {
        let mut list = VecDeque::new();
        let id = header.pubkey;
        // all discriminators are the same length
        let prefix = self.d_whirlpool.len();
        let mut i;
        let pubkey_len = std::mem::size_of::<Pubkey>();
        #[cfg(target_os = "wasi")]
        HostImport::log(format!(
            "orca_edge - 1 - pubkey {}; data len {}",
            id,
            data.len()
        ));
        if match_discriminator(&self.d_whirlpoolconfig, data) {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("orca_edge - 2 - whirlpoolconfig - pubkey {};", id));
            // program
            list.push_back(FilterEdge {
                slot: header.slot,
                weight: WEIGHT_DIRECT,
                from: self.program_id,
                to: id,
            });
            // fee authority
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
            // Collect Protocol Fees Authority
            {
                i = 40;
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
            // Reward Emissions Super Authority
            {
                i = 72;
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
        } else if match_discriminator(&self.d_whirlpool, data) {
            #[cfg(target_os = "wasi")]
            HostImport::log(format!("orca_edge - 2 - whirlpool - pubkey {};", id));
            // WhirlpoolsConfig
            {
                i = 8;
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
            // skip mint edges; they are useless as the mint information is available in the vault
            // accounts
            // token vault A
            {
                i = 133;
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
            // token vault B
            {
                i = 213;
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
            // rewards -> TBD
        } else if match_discriminator(&self.d_tickarray, data) {
            // whirlpool
            {
                i = data.len() - pubkey_len;
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
        #[cfg(target_os = "wasi")]
        HostImport::log(format!("orca_edge - 4 - pubkey {};", id));
        list
    }
}

impl Orca {
    pub fn new(program_id: &Pubkey) -> Self {
        let d_whirlpool = whirlpool_discriminator();
        let d_whirlpoolconfig = whirlpoolconfig_discriminator();
        let d_tickarray = tickarray_discriminator();
        Self {
            program_id: *program_id,
            d_whirlpool,
            d_tickarray,
            d_whirlpoolconfig,
        }
    }
}
//  9d 14 31 e0 d9 57 c1 fe
pub fn whirlpoolconfig_discriminator() -> [u8; 8] {
    [157, 20, 49, 224, 217, 87, 193, 254]
}

// 3f 95 d1 0c e1 80 63 09
pub fn whirlpool_discriminator() -> [u8; 8] {
    [63, 149, 209, 12, 225, 128, 99, 9]
}
// 45 61 bd be 6e 07 42 bb
pub fn tickarray_discriminator() -> [u8; 8] {
    [69, 97, 189, 190, 110, 7, 66, 187]
}

// unit tests
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::primitive::tree::Weight;

    use super::*; // import functions from the parent module
    use std::sync::Once;

    static INIT_LOGGER: Once = Once::new();

    fn init_logger() {
        INIT_LOGGER.call_once(|| {
            env_logger::init();
        });
    }

    use log::{info, warn};
    // dump with:  solana account --output-file=./whirlpool1.bin DtYKbQELgMZ3ihFUrCcCs9gy4djcUuhwgR7UpxVpP2Tg; xxd -p -c 99999999 ./whirlpool1.bin
    #[test]
    fn test_whirlpool() {
        init_logger();
        let program_id = Pubkey::try_from("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap();
        let data = std::fs::read("tests/data/whirlpool1.bin").unwrap();
        info!("data length {}", data.len());
        let header = AccountHeader {
            pubkey: Pubkey::try_from("DtYKbQELgMZ3ihFUrCcCs9gy4djcUuhwgR7UpxVpP2Tg").unwrap(),
            lamports: 488832,
            data_size: data.len() as u32,
            node_id: 100432,
            owner: program_id,
            rent_epoch: 1,
            slot: 1,
            executable: false,
        };
        let filter = Orca::new(&program_id);
        let mut list = filter.edge(&header, &data);
        assert_eq!(list.len(), 3, "whirlpool must have 3 edges");
        let mut m_edge: HashMap<Pubkey, Weight> = HashMap::default();
        while let Some(edge) = list.pop_front() {
            if edge.from == header.pubkey {
                // from
                m_edge.insert(edge.to, edge.weight);
            } else {
                // to
                m_edge.insert(edge.from, edge.weight);
            }
        }
        let token_vault_a =
            Pubkey::try_from("3AfqjdDWMof5p2gEH4MRPZQyhDC36spx8GJk5LZJQRnP").unwrap();
        assert_eq!(*m_edge.get(&token_vault_a).unwrap(), WEIGHT_DIRECT);
        let token_vault_b =
            Pubkey::try_from("CdRr1RX5uFdJes33NiFiiG5TZNd6gvXWts9u9xjiCVRq").unwrap();
        assert_eq!(*m_edge.get(&token_vault_b).unwrap(), WEIGHT_DIRECT);
    }
    #[test]
    fn test_tickarray() {
        init_logger();
        let program_id = Pubkey::try_from("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap();
        let list_pubkey = [
            Pubkey::try_from("AAgXUtyECrW4uXyRzzrwgZrnutJ6YnftSSN1DHPehtgw").unwrap(),
            Pubkey::try_from("FmU2Tg7vvqPhcZELTqUZCnNA6QqjZwGhDwZnNKWQurqf").unwrap(),
        ];
        let whirlpool = Pubkey::try_from("DtYKbQELgMZ3ihFUrCcCs9gy4djcUuhwgR7UpxVpP2Tg").unwrap();
        for i in 1..3 {
            let data = std::fs::read(format!("tests/data/tickarray{}.bin", i)).unwrap();
            info!("data length {}", data.len());
            let header = AccountHeader {
                pubkey: list_pubkey[i - 1],
                lamports: 488832,
                data_size: data.len() as u32,
                node_id: i as u64,
                owner: program_id,
                rent_epoch: 1,
                slot: 1,
                executable: false,
            };
            let filter = Orca::new(&program_id);
            let mut list = filter.edge(&header, &data);
            assert_eq!(list.len(), 1, "tick array must have 1 edge");
            let mut m_edge: HashMap<Pubkey, Weight> = HashMap::default();
            while let Some(edge) = list.pop_front() {
                if edge.from == header.pubkey {
                    // from
                    m_edge.insert(edge.to, edge.weight);
                } else {
                    // to
                    m_edge.insert(edge.from, edge.weight);
                }
            }
            info!("m_edge {:?}", m_edge);
            assert_eq!(
                *m_edge.get(&whirlpool).unwrap(),
                WEIGHT_DIRECT,
                "need {} but got {:?} for whirlpool",
                whirlpool,
                m_edge
            );
        }
    }
    #[test]
    fn test_whirlpoolconfig() {
        init_logger();
        let program_id = Pubkey::try_from("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap();
        let data = std::fs::read("tests/data/whirlpoolconfig1.bin").unwrap();
        assert_eq!(data.len(), 108);
        info!("data length {}", data.len());
        let header = AccountHeader {
            pubkey: Pubkey::try_from("2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ").unwrap(),
            lamports: 488832,
            data_size: data.len() as u32,
            node_id: 100432,
            owner: program_id,
            rent_epoch: 1,
            slot: 1,
            executable: false,
        };
        let filter = Orca::new(&program_id);
        let mut list = filter.edge(&header, &data);
        assert_eq!(list.len(), 4, "whirlpool config must have 3 edges");
        let mut m_edge: HashMap<Pubkey, Weight> = HashMap::default();
        while let Some(edge) = list.pop_front() {
            if edge.from == header.pubkey {
                // from
                m_edge.insert(edge.to, edge.weight);
            } else {
                // to
                m_edge.insert(edge.from, edge.weight);
            }
        }
        info!("list {:?}", m_edge);
        let collect_fee_authority =
            Pubkey::try_from("CRQd5wvbf6FKVmjHC7on8w4pzFPzudij2BKXRcMCu7aK").unwrap();
        assert_eq!(*m_edge.get(&collect_fee_authority).unwrap(), WEIGHT_DIRECT);
        let fee_authority =
            Pubkey::try_from("6BLTtBS9miUZruZtR9reTzp6ctGc4kVY4xrcxQwurYtw").unwrap();
        assert_eq!(*m_edge.get(&fee_authority).unwrap(), WEIGHT_DIRECT);
        let reward = Pubkey::try_from("DjDsi34mSB66p2nhBL6YvhbcLtZbkGfNybFeLDjJqxJW").unwrap();
        assert_eq!(*m_edge.get(&reward).unwrap(), WEIGHT_DIRECT);
    }
}
