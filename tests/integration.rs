// Integration tests (native) for the `hanzi-cat` crate.
// These tests avoid wasm-specific functionality and exercise pure Rust logic so
// they can run under `cargo test` on the host.

// Assert legacy powerups always return false (purchase_powerup currently disabled)
#[test]
fn purchase_powerup_returns_false() {
    assert!(!hanzi_cat::purchase_powerup("shield"));
}

// Basic dataset sanity check: ensure the SINGLE_HANZI dataset is non-empty.
#[test]
fn single_hanzi_dataset_nonempty() {
    assert!(!hanzi_cat::SINGLE_HANZI.is_empty());
}

