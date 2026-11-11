//! Read-only smoke test against the real session's Secret Service.
//! Prints keyring names, lock state and item counts only, never item
//! labels or secrets.
//!
//!     cargo run --example smoke

use moneypenny_lib::keyring::{Keyring, KeyringError};

#[tokio::main]
async fn main() -> Result<(), KeyringError> {
    let keyring = Keyring::connect().await?;
    for c in keyring.collections().await? {
        let listed = match keyring.items(&c.id).await {
            Ok(items) => format!("{} items readable", items.len()),
            Err(e) => format!("items: {e}"),
        };
        println!(
            "{:<12} locked={:<5} default={:<5} session={:<5} count={:<4} {listed}",
            c.label, c.locked, c.is_default, c.is_session, c.item_count
        );
    }
    Ok(())
}
