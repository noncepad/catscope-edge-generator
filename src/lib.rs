use crate::kvault::Kvault;
use crate::meteora::Meteora;
use crate::orca::Orca;
use crate::pumpfun::Pumpfun;
use crate::pumpswap::Pumpswap;
use crate::raydium::amm::RaydiumAmm;
use crate::raydium::cpmm::RaydiumCPMM;
use crate::sanctum::Sanctum;
use crate::{kamino::Kamino, raydium::clmm::RaydiumCLMM};
#[cfg(any(target_os = "wasi", target_os = "linux"))]
use primitive::{
    filter::{ptr_to_filter, CatscopeFilter},
    wasmimport::HostImport,
    wasmstore::{AccountOnGuest, FilterEdgeWithNextPointer},
};
use primitive::{
    guest::GuestFilter,
    soltoken::SolToken,
    tree::{parse_program_list, ProgramList},
};
use safejar::Safejar;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::system_program;
use solpipe::Solpipe;
use std::collections::VecDeque;

//pub mod all;
pub mod kamino;
pub mod kvault;
pub mod meteora;
pub mod orca;
pub mod primitive;
pub mod pumpfun;
pub mod pumpswap;
pub mod raydium;
pub mod safejar;
pub mod sanctum;
pub mod solpipe;
pub(crate) const DISCRIMINATOR_SIZE: usize = 8;
/// A place holder for starting the web assembly.
/// # Safety
#[cfg(target_os = "wasi")]
#[no_mangle] // Prevent Rust from changing the function name
pub extern "C" fn _start() -> i32 {
    0
}

/// The code below defines a basic filter (edge generator) for
/// system, token, Safejar, and Solpipe accounts.
/// # Safety
#[cfg(target_os = "wasi")]
#[no_mangle] // Prevent Rust from changing the function name
pub unsafe extern "C" fn init() -> u64 {
    let mut hi = HostImport::default();

    let mut list: VecDeque<Box<dyn GuestFilter + 'static>> = VecDeque::new();
    list.push_back(Box::new(SolToken::default()));

    let args = match hi.init_args() {
        Ok(x) => match x {
            Some(y) => y,
            None => return 0,
        },
        Err(_) => return 0,
    };
    let _program_list = match parse_program_list(args.slice()) {
        Ok(x) => x,
        Err(_) => vec![],
    };
    let prog_safejar = Pubkey::from_str_const("jarMRc5v8QZLn8AawsmwN426mG9HHVaAtiZwa7rMi9J");
    list.push_back(Box::new(Safejar::new(&prog_safejar)));
    let prog_solpipe = Pubkey::from_str_const("CBAidZ5BjA1BYi9WF6Ca1AaWakF2MPxkVgp7oo5tDyW3");
    list.push_back(Box::new(Solpipe::new(&prog_solpipe)));
    let prog_orca = Pubkey::from_str_const("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
    list.push_back(Box::new(Orca::new(&prog_orca)));
    let prog_raydium_clmm = Pubkey::from_str_const("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
    list.push_back(Box::new(RaydiumCLMM::new(&prog_raydium_clmm)));
    let prog_raydium_cpmm = Pubkey::from_str_const("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
    list.push_back(Box::new(RaydiumCPMM::new(&prog_raydium_cpmm)));
    let prog_raydium_amm = Pubkey::from_str_const("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
    list.push_back(Box::new(RaydiumAmm::new(&prog_raydium_amm)));
    let prog_meteora = Pubkey::from_str_const("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
    list.push_back(Box::new(Meteora::new(&prog_meteora)));
    let prog_klend = Pubkey::from_str_const("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
    list.push_back(Box::new(Kamino::new(&prog_klend)));
    let prog_kvault = Pubkey::from_str_const("KvauGMspG5k6rtzrqqn7WNn3oZdyKqLKwK2XWQ8FLjd");
    list.push_back(Box::new(Kvault::new(&prog_kvault)));
    let prog_sanctum = Pubkey::from_str_const("5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx");
    list.push_back(Box::new(Sanctum::new(&prog_sanctum)));
    let prog_pumpfun = Pubkey::from_str_const("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
    list.push_back(Box::new(Pumpfun::new(&prog_pumpfun)));
    let prog_pumpswap = Pubkey::from_str_const("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
    list.push_back(Box::new(Pumpswap::new(&prog_pumpswap)));

    let filter = Box::new(CatscopeFilter::new(list, hi));
    Box::into_raw(filter) as u64
}

/// Deallocate a blob.
/// # Returns
/// Returns the memory offset to this byte slice.
/// # Safety
#[cfg(target_os = "wasi")]
#[no_mangle] // Prevent Rust from changing the function name
pub unsafe extern "C" fn allocate(cat_ptr: u64, data_size: u32) -> u64 {
    let filter: &mut CatscopeFilter = ptr_to_filter(cat_ptr).unwrap();
    let r = filter.store_mut().allocate(data_size as usize).unwrap();
    r.pointer()
}

/// Deallocate a blob.
/// # Safety
#[cfg(target_os = "wasi")]
#[no_mangle]
pub unsafe extern "C" fn deallocate(cat_ptr: u64, ptr: u64) -> u64 {
    let filter: &mut CatscopeFilter = ptr_to_filter(cat_ptr).unwrap();
    filter.store_mut().deallocate(ptr);
    0
}

/// Close the filtering object.
/// # Safety
#[cfg(target_os = "wasi")]
#[no_mangle] // Prevent Rust from changing the function name
pub unsafe extern "C" fn close(cat_ptr: u64) -> u64 {
    unsafe {
        // Reconstruct the Box<T> from the raw pointer and immediately drop it
        drop(Box::from_raw(cat_ptr as *mut CatscopeFilter));
    }
    0
}

/// A holder for responses.
/// # Safety
#[cfg(target_os = "wasi")]
#[no_mangle] // Prevent Rust from changing the function name
pub unsafe extern "C" fn response(
    _x1: i64,
    _x2: i64,
    _x3: i32,
    _x4: i64,
    _x5: i64,
    _x6: i64,
    _x7: i64,
    _x8: i64,
    _x9: i64,
    _x10: i64,
    _x11: i64,
    _x12: i64,
    _x13: i64,
) -> i64 {
    0
}

/// List programs that need to be tracked.
/// # Safety
///
#[cfg(target_os = "wasi")]
#[no_mangle]
pub unsafe extern "C" fn program_list(cat_ptr: u64) -> u64 {
    let filter: &mut CatscopeFilter = ptr_to_filter(cat_ptr).unwrap();
    let list = filter.program_id_list();
    let store = filter.store_mut();
    let mut out = store.allocate_struct::<ProgramList>().unwrap();
    {
        let p = out.payload_mut::<ProgramList>();
        for (i, pk) in list.iter().enumerate() {
            p.list[i] = *pk;
        }
        p.count = list.len() as u16;
    }
    out.pointer()
}

/// Produce edges from reading an account.
/// # Safety
///
#[cfg(target_os = "wasi")]
#[no_mangle] // Prevent Rust from changing the function name
pub unsafe extern "C" fn edge(cat_ptr: u64, ptr: u64, size: u32) -> u64 {
    let filter: &mut CatscopeFilter = ptr_to_filter(cat_ptr).unwrap();
    let blob;
    {
        let store = filter.store();
        blob = store.recover_blob(ptr as usize, size as usize).unwrap();
    }

    let a: AccountOnGuest = match blob.try_into() {
        Ok(x) => x,
        Err(_) => return 0,
    };
    let h = a.header();
    /*    HostImport::log(format!(
        "edge - 1 - header {} {} {}",
        h.pubkey, h.owner, h.lamports
    ));*/
    HostImport::log(format!(
        "outside - 2 - pubkey {}; owner {}",
        h.pubkey, h.owner,
    ));
    let data = a.data();
    let mut list = filter.edge(h, data);
    if list.is_empty() {
        return 0;
    }
    let mut last_ptr = 0;
    while let Some(edge) = list.pop_back() {
        let store = filter.store_mut();
        let mut out = store
            .allocate_struct::<FilterEdgeWithNextPointer>()
            .unwrap();
        let h = out.payload_mut::<FilterEdgeWithNextPointer>();
        h.edge = edge;
        h.next_pointer = last_ptr;
        last_ptr = out.pointer();
    }
    last_ptr
}

#[inline]
pub(crate) fn pubkey_not_blank(pubkey: &Pubkey) -> Option<&Pubkey> {
    if pubkey.eq(&system_program::ID) {
        None
    } else {
        Some(pubkey)
    }
}

#[inline]
pub(crate) fn read_pubkey(data: &[u8]) -> Option<&Pubkey> {
    if data.len() != 32 {
        return None;
    }
    let ptr = data.as_ptr() as *const Pubkey;
    let pubkey = unsafe { &*ptr };
    if pubkey.eq(&system_program::ID) {
        return None;
    }

    Some(pubkey)
}
