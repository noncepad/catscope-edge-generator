// Feed every account from an extracted Solana mainnet snapshot through CatscopeFilter.
//
// Environment variables:
//
//   DOWNLOAD_SNAPSHOT=1
//     Discovers a validator that is serving a snapshot by:
//       1. Calling getClusterNodes on https://api.mainnet-beta.solana.com
//       2. Probing each node's tpuQuic endpoint concurrently with a QUIC handshake
//       3. Checking HTTP HEAD on /snapshot.tar.lz4 for every QUIC-alive node
//     The first URL that answers 200 is downloaded with curl and extracted with tar.
//
//   SNAPSHOT_DIR=<path>   (optional when DOWNLOAD_SNAPSHOT=1)
//     Where to extract (or look for a pre-extracted) snapshot.
//     Defaults to /tmp/catscope-snapshot-test.
//     If accounts/ already exists there, the download is skipped.
//
//   SNAPSHOT_PROGRAMS=<ids>
//     Comma-separated program IDs in filter-slot order:
//       0 = Safejar     1 = Solpipe    2 = Orca (Whirlpool)
//       3 = RaydiumCLMM 4 = RaydiumCPMM 5 = RaydiumAmm
//       6 = Meteora DLMM 7 = Kamino    8 = Sanctum (INF)
//
// Examples:
//   # Auto-discover, download, and test:
//   DOWNLOAD_SNAPSHOT=1 SNAPSHOT_DIR=/data/snapshot \
//   SNAPSHOT_PROGRAMS="..." cargo test snapshot -- --nocapture
//
//   # Re-test a previously extracted snapshot (no download):
//   SNAPSHOT_DIR=/data/snapshot cargo test snapshot -- --nocapture
//
// AppendVec binary format (agave 2.x, no inline account hash):
//
//   offset  size  field
//   ──────  ────  ─────────────────────────────
//    0        8   write_version_obsolete (u64 LE, always 0 in 2.x)
//    8        8   data_len (u64 LE)
//   16       32   pubkey
//   48        8   lamports (u64 LE)
//   56        8   rent_epoch (u64 LE)
//   64       32   owner
//   96        1   executable (0 or 1)
//   97        7   padding
//  104   data_len account data
//
//   next record starts at ((104 + data_len) + 7) & !7

use std::{
    collections::VecDeque,
    fs::{self, File},
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};

use memmap2::Mmap;
use solana_sdk::pubkey::Pubkey;

use catscope_edge_generator::{
    kamino::Kamino,
    meteora::Meteora,
    orca::Orca,
    primitive::{
        filter::CatscopeFilter,
        guest::GuestFilter,
        header::AccountHeader,
        soltoken::SolToken,
        tree::parse_program_list,
        wasmimport::HostImport,
    },
    raydium::{amm::RaydiumAmm, clmm::RaydiumCLMM, cpmm::RaydiumCPMM},
    safejar::Safejar,
    sanctum::Sanctum,
    solpipe::Solpipe,
};

// ── AppendVec parser ──────────────────────────────────────────────────────────

const OFF_DATA_LEN: usize = 8;
const OFF_PUBKEY: usize = 16;
const OFF_LAMPORTS: usize = 48;
const OFF_RENT_EPOCH: usize = 56;
const OFF_OWNER: usize = 64;
const OFF_EXECUTABLE: usize = 96;
const OFF_DATA: usize = 104; // StoredMeta(48) + AccountMeta(56, repr(C) padded)

struct AppendVecIter<'a> {
    buf: &'a [u8],
    off: usize,
}

struct RawAccount<'a> {
    pubkey: Pubkey,
    lamports: u64,
    rent_epoch: u64,
    owner: Pubkey,
    executable: bool,
    data: &'a [u8],
}

impl<'a> Iterator for AppendVecIter<'a> {
    type Item = RawAccount<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let off = self.off;
        if off + OFF_DATA > self.buf.len() {
            return None;
        }
        let data_len = u64::from_le_bytes(
            self.buf[off + OFF_DATA_LEN..off + OFF_DATA_LEN + 8]
                .try_into()
                .ok()?,
        ) as usize;
        // Trailing zero padding: zero pubkey + data_len == 0 → stop.
        if data_len == 0
            && self.buf[off + OFF_PUBKEY..off + OFF_PUBKEY + 32]
                .iter()
                .all(|&b| b == 0)
        {
            return None;
        }
        if off + OFF_DATA + data_len > self.buf.len() {
            return None;
        }
        let pubkey =
            Pubkey::try_from(&self.buf[off + OFF_PUBKEY..off + OFF_PUBKEY + 32]).ok()?;
        let lamports = u64::from_le_bytes(
            self.buf[off + OFF_LAMPORTS..off + OFF_LAMPORTS + 8]
                .try_into()
                .ok()?,
        );
        let rent_epoch = u64::from_le_bytes(
            self.buf[off + OFF_RENT_EPOCH..off + OFF_RENT_EPOCH + 8]
                .try_into()
                .ok()?,
        );
        let owner =
            Pubkey::try_from(&self.buf[off + OFF_OWNER..off + OFF_OWNER + 32]).ok()?;
        let executable = self.buf[off + OFF_EXECUTABLE] != 0;
        let data = &self.buf[off + OFF_DATA..off + OFF_DATA + data_len];
        self.off = off + ((OFF_DATA + data_len + 7) & !7);
        Some(RawAccount { pubkey, lamports, rent_epoch, owner, executable, data })
    }
}

// ── QUIC snapshot discovery ───────────────────────────────────────────────────

// rustls cert verifier that accepts any certificate (validators use self-signed certs).
#[derive(Debug)]
struct SkipCertVerification;

impl rustls::client::danger::ServerCertVerifier for SkipCertVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _msg: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _msg: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

fn make_quic_endpoint() -> quinn::Endpoint {
    // Install the ring crypto provider once; ignore error if already installed.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut tls = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipCertVerification))
        .with_no_client_auth();
    // Match the ALPN Solana validators expect on TPU QUIC.
    tls.alpn_protocols = vec![b"solana-tpu".to_vec()];

    let quic_tls =
        quinn::crypto::rustls::QuicClientConfig::try_from(tls).expect("invalid TLS config");

    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(
        Duration::from_secs(5).try_into().expect("timeout fits"),
    ));

    let mut client_cfg = quinn::ClientConfig::new(Arc::new(quic_tls));
    client_cfg.transport_config(Arc::new(transport));

    let mut endpoint =
        quinn::Endpoint::client("0.0.0.0:0".parse().unwrap()).expect("QUIC endpoint");
    endpoint.set_default_client_config(client_cfg);
    endpoint
}

// Returns true if a QUIC handshake with `addr` completes within the transport timeout.
async fn probe_quic(endpoint: &quinn::Endpoint, addr: SocketAddr) -> bool {
    let connecting = match endpoint.connect(addr, "validator") {
        Ok(c) => c,
        Err(_) => return false,
    };
    tokio::time::timeout(Duration::from_secs(5), connecting)
        .await
        .is_ok_and(|r| r.is_ok())
}

struct Candidate {
    tpu_quic: SocketAddr,
    rpc: String, // "ip:port" for HTTP snapshot download
}

// Call getClusterNodes on mainnet-beta and return structured candidates.
fn fetch_cluster_nodes() -> Vec<Candidate> {
    let output = Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "30",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"jsonrpc":"2.0","id":1,"method":"getClusterNodes"}"#,
            "https://api.mainnet-beta.solana.com",
        ])
        .output()
        .expect("curl: getClusterNodes failed");

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("getClusterNodes: invalid JSON");

    json["result"]
        .as_array()
        .expect("getClusterNodes: missing result array")
        .iter()
        .filter_map(|n| {
            let tpu_quic: SocketAddr = n["tpuQuic"].as_str()?.parse().ok()?;
            let rpc = n["rpc"].as_str()?.to_string();
            Some(Candidate { tpu_quic, rpc })
        })
        .collect()
}

// Probe all candidates concurrently and return the first HTTP snapshot URL that
// answers 200.
fn find_snapshot_url() -> String {
    let candidates = fetch_cluster_nodes();
    println!(
        "cluster: {} nodes with tpuQuic + rpc",
        candidates.len()
    );

    let endpoint = make_quic_endpoint();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    // Probe all candidates concurrently; collect the rpc strings of alive nodes.
    let alive_rpcs: Vec<String> = rt.block_on(async {
        let mut set = tokio::task::JoinSet::new();
        for c in &candidates {
            let ep = endpoint.clone();
            let addr = c.tpu_quic;
            let rpc = c.rpc.clone();
            set.spawn(async move {
                if probe_quic(&ep, addr).await {
                    Some(rpc)
                } else {
                    None
                }
            });
        }
        let mut alive = Vec::new();
        while let Some(res) = set.join_next().await {
            if let Ok(Some(rpc)) = res {
                alive.push(rpc);
            }
        }
        alive
    });

    endpoint.close(0u32.into(), b"done");
    println!("{} validators answered QUIC", alive_rpcs.len());

    // For each alive node check whether it serves a snapshot via HTTP.
    for rpc in &alive_rpcs {
        let url = format!("http://{}/snapshot.tar.lz4", rpc);
        let http_code = Command::new("curl")
            .args([
                "-s", "-o", "/dev/null", "-w", "%{http_code}",
                "--max-time", "5", "-I",
            ])
            .arg(&url)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        if http_code.trim() == "200" {
            println!("snapshot available at {url}");
            return url;
        }
    }

    panic!("no validator found serving a snapshot after probing {} alive nodes", alive_rpcs.len());
}

// ── Download / extract ────────────────────────────────────────────────────────

fn download_and_extract(url: &str, dest_dir: &Path) {
    let accounts_dir = dest_dir.join("accounts");
    if accounts_dir.exists() {
        println!("accounts/ already present in {} — skipping download", dest_dir.display());
        return;
    }

    fs::create_dir_all(dest_dir)
        .unwrap_or_else(|e| panic!("cannot create {}: {e}", dest_dir.display()));

    let filename = url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("snapshot.tar.lz4");
    let archive = dest_dir.join(filename);

    println!("Downloading {} -> {}", url, archive.display());
    let status = Command::new("curl")
        .args(["-L", "--progress-bar", "-C", "-", "-o"])
        .arg(&archive)
        .arg(url)
        .status()
        .expect("curl: download failed");
    assert!(status.success(), "curl exited with {status}");

    println!("Extracting {} -> {}", archive.display(), dest_dir.display());
    let status = Command::new("tar")
        .arg("xf")
        .arg(&archive)
        .arg("-C")
        .arg(dest_dir)
        .status()
        .expect("tar: extraction failed");
    assert!(status.success(), "tar exited with {status}");

    println!("Extraction complete");
}

// ── Filter construction ───────────────────────────────────────────────────────

fn build_filter(programs_arg: &str) -> CatscopeFilter {
    let mut list: VecDeque<Box<dyn GuestFilter + 'static>> = VecDeque::new();
    list.push_back(Box::new(SolToken::default()));
    if !programs_arg.is_empty() {
        let program_list =
            parse_program_list(programs_arg.as_bytes()).expect("invalid SNAPSHOT_PROGRAMS");
        for (i, pid) in program_list.iter().enumerate() {
            match i {
                0 => list.push_back(Box::new(Safejar::new(pid))),
                1 => list.push_back(Box::new(Solpipe::new(pid))),
                2 => list.push_back(Box::new(Orca::new(pid))),
                3 => list.push_back(Box::new(RaydiumCLMM::new(pid))),
                4 => list.push_back(Box::new(RaydiumCPMM::new(pid))),
                5 => list.push_back(Box::new(RaydiumAmm::new(pid))),
                6 => list.push_back(Box::new(Meteora::new(pid))),
                7 => list.push_back(Box::new(Kamino::new(pid))),
                8 => list.push_back(Box::new(Sanctum::new(pid))),
                _ => {}
            }
        }
    }
    CatscopeFilter::new(list, HostImport::default())
}

// ── Test ─────────────────────────────────────────────────────────────────────

#[test]
fn snapshot() {
    let download = std::env::var("DOWNLOAD_SNAPSHOT").ok();
    let dir_override = std::env::var("SNAPSHOT_DIR").ok().map(PathBuf::from);

    let snapshot_dir = match download.as_deref() {
        Some("1") => {
            let dir = dir_override
                .unwrap_or_else(|| std::env::temp_dir().join("catscope-snapshot-test"));
            let url = find_snapshot_url();
            download_and_extract(&url, &dir);
            dir
        }
        Some(_) => panic!("DOWNLOAD_SNAPSHOT must be \"1\" (got something else)"),
        None => match dir_override {
            Some(d) => d,
            None => {
                println!("neither DOWNLOAD_SNAPSHOT nor SNAPSHOT_DIR set — skipping");
                return;
            }
        },
    };

    let programs_arg = std::env::var("SNAPSHOT_PROGRAMS").unwrap_or_default();
    let filter = build_filter(&programs_arg);

    let accounts_dir = snapshot_dir.join("accounts");
    let mut entries: Vec<PathBuf> = fs::read_dir(&accounts_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", accounts_dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s.chars().all(|c| c.is_ascii_digit() || c == '.') && s.contains('.')
        })
        .map(|e| e.path())
        .collect();
    entries.sort();
    println!("found {} AppendVec files", entries.len());

    let mut total_accounts: u64 = 0;
    let mut total_edges: u64 = 0;
    let mut file_errors: u64 = 0;
    let mut panics: u64 = 0;

    for path in &entries {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("open {:?}: {e}", path);
                file_errors += 1;
                continue;
            }
        };
        let mmap = match unsafe { Mmap::map(&file) } {
            Ok(m) => m,
            Err(e) => {
                eprintln!("mmap {:?}: {e}", path);
                file_errors += 1;
                continue;
            }
        };

        for account in (AppendVecIter { buf: &mmap, off: 0 }) {
            let header = AccountHeader {
                pubkey: account.pubkey,
                lamports: account.lamports,
                data_size: account.data.len() as u32,
                account_id: 0,
                owner: account.owner,
                rent_epoch: account.rent_epoch,
                slot: 0,
                executable: account.executable,
            };
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                filter.edge(&header, account.data).len() as u64
            })) {
                Ok(n) => total_edges += n,
                Err(_) => {
                    eprintln!(
                        "PANIC: pubkey={} owner={} data_len={}",
                        account.pubkey, account.owner, account.data.len()
                    );
                    panics += 1;
                }
            }
            total_accounts += 1;
            if total_accounts % 1_000_000 == 0 {
                println!("  {total_accounts} accounts, {total_edges} edges");
            }
        }
    }

    println!(
        "done: {total_accounts} accounts | {total_edges} edges | \
         {file_errors} file errors | {panics} panics"
    );
    assert_eq!(panics, 0, "CatscopeFilter panicked on {panics} accounts");
}
