# Catscope Edge Generator

## Install Dependencies

```bash
curl https://sh.rustup.rs -sSf | sh
rustup toolchain install 1.84.0
rustup override set 1.84.0
rustup target add wasm32-wasip1
cargo install cargo-deb@2.7.0
```

* do not use the latest `cargo-deb` version as it does not support packaging `wasm32-wasip1` targets into a Debian package for `x86_64`.

## Compile wasm32-wasip1

```bash
cargo build --target wasm32-wasip1
```

## Compile Debian package

```bash
cargo build --target wasm32-wasip1 --release
cargo deb --no-build
```

The resulting `.deb` package will be in `./target/debian`.

The wasm will be installed in `/usr/share/catscope/catscope_edge_generator.wasm`.

## Update the Geyser config to reflect the wasm file path

```json
{
   "libpath":"/usr/lib/libsolana_geyser_plugin_catscope.so",
   "filter": "/usr/share/catscope/solpipe_filter.wasm",
   "filter_count": 128,
   "filter_args":"/etc/catscope/program.txt",
   "worker_count": 20,
   "worker_count_after_startup": 18,
   "bot":{
      "core_count": 2
   },
   "log":{
      "level":"warn"
   },
   "net":{
      "listen_url":"127.0.0.1:50001",
      "ring_size": 1024,
      "buffer_size": 1048576,
      "max_write": 1024,
      "store_dir":"/var/share/catscope"
    },
    "shooter":{
      "broadcast_path":"/validator",
      "version":1,
      "buffer_size":100000
    }
}
```

Set `/etc/catscope/program.txt` to look like:

```
TRSY7YgS3tcDoi6ZgTp2MmPJpXHyCVrGaFhL7HLdQc9,CBAidZ5BjA1BYi9WF6Ca1AaWakF2MPxkVgp7oo5tDyW3,whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc,CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C
```

* list base58 encoded program ids in the order of Safejar, Solpipe, Orca, and Raydium
* these are the program ids for Mainnet

## Run Tests

```bash
RUST_LOG=debug RUST_BACKTRACE=full CARGO_BUILD_JOBS=2 cargo test -- --nocapture
```

