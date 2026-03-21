use std::collections::HashMap;

use solana_sdk::pubkey::Pubkey;

use super::{
    err::CatscopeWasmError,
    header::{AccountHeader, AccountId},
    wasmstore::{GuestBlob, Store},
};

/// Import functions from the host through this struct.
/// This struct also provides a Store.
#[derive(Default)]
pub struct HostImport {
    store: Store,
    m_pubkey: HashMap<Pubkey, AccountId>,
    pub m_tx: HashMap<u64, TransactionState>,
}

pub struct TransactionState {
    status: u8,
    slot: u64,
}
const MAX_PUBKEY_HASHMAP_SIZE: usize = 1000;

impl HostImport {
    pub fn store(&self) -> &Store {
        &self.store
    }
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
    pub fn init_args(&mut self) -> Result<Option<GuestBlob>, CatscopeWasmError> {
        let size = unsafe { hf_init_args_size() };
        let mut o_blob = self.store.allocate2(size as usize);
        if o_blob.is_none() {
            return Ok(None);
        }
        let blob = o_blob.take().unwrap();
        let ptr = blob.pointer();
        let result = unsafe { hf_init_args(ptr) };
        if result < 0 {
            Err(CatscopeWasmError::InsufficientBuffer)
        } else {
            Ok(Some(blob))
        }
    }
    /// Look up the AccountId representation of a Pubkey.
    pub fn pubkey_lookup(
        &mut self,
        list: &[Pubkey],
    ) -> Result<Option<GuestBlob>, CatscopeWasmError> {
        if list.is_empty() {
            return Ok(None);
        }
        if list.len() == 1 {
            if self.m_pubkey.contains_key(&list[0]) {
                return Ok(None);
            }
        }
        let node_len = std::mem::size_of::<AccountId>();
        let store = self.store_mut();
        let blob = store.allocate2(std::mem::size_of_val(list)).unwrap();
        let req_ptr = blob.pointer();
        let req_len = std::mem::size_of_val(list) as u32;
        let resp_blob = store.allocate2(node_len * list.len()).unwrap();
        let resp_callback_id = resp_blob.pointer();
        let result = unsafe { hf_pubkey_lookup(req_ptr, req_len, resp_callback_id) };
        if result < 0 {
            return Err(CatscopeWasmError::InsufficientBuffer);
        }
        let resp_slice =
            unsafe { std::slice::from_raw_parts(resp_callback_id as *const AccountId, list.len()) };
        // make sure we do not have too many keys and fill the memory up
        if MAX_PUBKEY_HASHMAP_SIZE < self.m_pubkey.len() {
            self.m_pubkey.clear();
        }
        for i in 0..list.len() {
            self.m_pubkey.insert(list[i], resp_slice[i]);
        }
        Ok(Some(resp_blob))
    }

    /// Fetch the header from the host.
    pub fn header(
        &mut self,
        node_id: AccountId,
        o_slot: Option<u64>,
    ) -> Result<Option<GuestBlob>, CatscopeWasmError> {
        let header_size = std::mem::size_of::<AccountHeader>();
        let store = self.store_mut();
        let blob = store.allocate2(header_size).unwrap();
        let header_ptr = blob.pointer();
        let s = o_slot.unwrap_or_default();
        let header_result = unsafe { hf_account_header(header_ptr, node_id, s) };
        if header_result < 0 {
            Err(CatscopeWasmError::InsufficientBuffer)
        } else if header_result == 0 {
            Ok(None)
        } else {
            Ok(Some(blob))
        }
    }

    /// Fetch the body of an account from the host.
    pub fn body(
        &mut self,
        header_blob: &GuestBlob,
    ) -> Result<Option<GuestBlob>, CatscopeWasmError> {
        let header: &AccountHeader = header_blob.payload();
        let size = header.data_size as usize;
        if size == 0 {
            return Ok(None);
        }
        let list = vec![header.pubkey];
        self.pubkey_lookup(&list)?;
        let node_id = *self.m_pubkey.get(&header.pubkey).unwrap();
        let store = self.store_mut();
        let body_blob = store.allocate2(size).unwrap();
        let req_callback_id = body_blob.pointer();
        let result = unsafe { hf_account_body(req_callback_id, node_id, header.slot) };
        if result < 0 {
            Err(CatscopeWasmError::InsufficientBuffer)
        } else if result == 0 {
            Ok(None)
        } else {
            Ok(Some(body_blob))
        }
    }

    pub fn tx_send(&mut self, tx: &[u8]) -> Result<u64, CatscopeWasmError> {
        if tx.is_empty() {
            return Err(CatscopeWasmError::InsufficientBuffer);
        }
        let mut blob = match self.store.allocate2(tx.len()) {
            Some(x) => x,
            None => return Err(CatscopeWasmError::InsufficientBuffer),
        };
        {
            let slice = blob.slice_mut();
            slice.copy_from_slice(tx);
        }
        let req_ptr = blob.pointer();
        let req_len = tx.len() as u32;
        let result = unsafe { hf_tx_send(req_ptr, req_len) };
        if result < 0 {
            Err(CatscopeWasmError::NotImplemented)
        } else {
            self.m_tx
                .insert(req_ptr, TransactionState { status: 0, slot: 0 });
            Ok(req_ptr)
        }
    }

    /// Create an extern function and export this to the host. The host will call this function.
    pub fn tx_response(&mut self, callback_id: u64, slot: u64, status: u8) {
        if let Some(ts) = self.m_tx.get_mut(&callback_id) {
            ts.slot = slot;
            ts.status = status;
        }
    }
    pub fn log(str: String) {
        let ptr: *const u8 = str.as_ptr();
        unsafe { hf_simple_log(ptr as u64, str.len() as u32) };
    }
}
pub trait CatscopeBot {
    fn on_slot(&mut self, slot: u64, status: u8) -> std::io::Result<()>;
}

pub struct Bot<B: CatscopeBot> {
    hook: B,
    pub store: Store,
}
impl<B: CatscopeBot> Bot<B> {
    pub fn new(hook: B) -> Result<Self, CatscopeWasmError> {
        let store = Store::default();
        Ok(Self { hook, store })
    }
}

// Imported functions
#[cfg(not(target_os = "linux"))]
#[link(wasm_import_module = "")]
extern "C" {
    fn hf_simple_log(ptr: u64, size: u32);
    fn hf_init_args_size() -> u32;
    fn hf_init_args(req_callback_id: u64) -> i32;
    fn hf_pubkey_lookup(req_ptr: u64, req_len: u32, resp_callback_id: u64) -> i32;
    fn hf_account_header(req_callback_id: u64, node_id: u64, o_slot: u64) -> i32;
    fn hf_account_body(req_callback_id: u64, node_id: u64, o_slot: u64) -> i32;
    fn hf_tx_send(req_ptr: u64, req_len: u32) -> i32;
}

#[cfg(target_os = "linux")]
unsafe fn hf_simple_log(_ptr: u64, _size: u32) {}
#[cfg(target_os = "linux")]
unsafe fn hf_init_args_size() -> u32 {
    0
}
#[cfg(target_os = "linux")]
unsafe fn hf_init_args(_req_callback_id: u64) -> i32 {
    0
}
#[cfg(target_os = "linux")]
unsafe fn hf_pubkey_lookup(_req_ptr: u64, _req_len: u32, _resp_callback_id: u64) -> i32 {
    0
}
#[cfg(target_os = "linux")]
unsafe fn hf_account_header(_req_callback_id: u64, _node_id: u64, _o_slot: u64) -> i32 {
    0
}
#[cfg(target_os = "linux")]
unsafe fn hf_account_body(_req_callback_id: u64, _node_id: u64, _o_slot: u64) -> i32 {
    0
}
#[cfg(target_os = "linux")]
unsafe fn hf_tx_send(_req_ptr: u64, _req_len: u32) -> i32 {
    0
}
