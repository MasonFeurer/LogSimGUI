set RUSTFLAGS=--cfg=web_sys_unstable_apis
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/log_sim_gui_web.wasm --out-dir site --no-modules --no-typescript
