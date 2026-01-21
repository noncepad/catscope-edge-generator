use std::cmp::Ordering;

use solana_sdk::pubkey::Pubkey;

pub type AccountId = u64;

/// Use this to store accounts.
#[repr(C, align(8))]
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct AccountHeader {
    /// account public key - (the ecdsa 32B public key)
    pub pubkey: Pubkey,

    /// lamports in the account
    pub lamports: u64,

    /// account ID (the `u64` assigned to the `pubkey` by Catscope)
    pub account_id: AccountId,

    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: Pubkey,

    ///  the epoch at which this account will next owe rent
    pub rent_epoch: u64,

    // the slot at which the transaction was registered
    pub slot: u64,

    // the size of the account data
    pub data_size: u32,

    //  This account's data contains a loaded program (and is now read-only)
    pub executable: bool,
}

impl AccountHeader {
    pub fn reset(&mut self) {
        self.pubkey = Pubkey::default();
        self.lamports = 0;
        self.data_size = 0;
        self.owner = Pubkey::default();
        self.executable = false;
        self.rent_epoch = 0;
        self.account_id = 0;
    }
    pub fn cmp(&self, b: &Self) -> Ordering {
        if self.slot < b.slot {
            Ordering::Less
        } else if self.slot == b.slot {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}
