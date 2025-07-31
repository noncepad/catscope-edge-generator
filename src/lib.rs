use self::orca::Orca;
use self::raydium::Raydium;
#[cfg(target_os = "wasi")]
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
use solpipe::Solpipe;
use std::collections::VecDeque;

//pub mod all;
pub mod orca;
pub mod primitive;
pub mod raydium;
pub mod safejar;
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
    let program_list = match parse_program_list(args.slice()) {
        Ok(x) => x,
        Err(_) => return 0,
    };
    for (i, program_id) in program_list.iter().enumerate() {
        match i {
            0 => {
                list.push_back(Box::new(Safejar::new(program_id)));
            }
            1 => {
                list.push_back(Box::new(Solpipe::new(program_id)));
            }
            2 => {
                list.push_back(Box::new(Orca::new(program_id)));
            }
            3 => {
                list.push_back(Box::new(Raydium::new(program_id)));
            }
            _ => {}
        }
    }

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
