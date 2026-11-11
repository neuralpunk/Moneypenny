//! Integration tests against a private oo7-daemon (see tests/common).

mod common;

use common::{TestBus, TestDaemon};
use moneypenny_lib::keyring::{Keyring, KeyringError, SecretValue};
use oo7::Secret;

macro_rules! daemon {
    () => {
        match TestDaemon::start().await {
            Some(d) => d,
            None => return,
        }
    };
}

#[tokio::test(flavor = "multi_thread")]
async fn lists_default_and_session_collections() {
    let d = daemon!();
    let service = d.service().await;
    let login = service.default_collection().await.unwrap();
    login
        .create_item("One", &[("app", "test")], Secret::text("1"), true, None)
        .await
        .unwrap();

    let collections = d.keyring().await.collections().await.unwrap();
    assert_eq!(collections.len(), 2, "{collections:?}");

    let first = &collections[0];
    assert!(first.is_default, "default keyring sorts first");
    assert_eq!(first.label, "Login");
    assert!(!first.locked);
    assert_eq!(first.item_count, 1);
    assert_eq!(first.id, login.path().as_str());

    let last = &collections[1];
    assert!(last.is_session, "session keyring sorts last");
    assert!(!last.is_default);
}

#[tokio::test(flavor = "multi_thread")]
async fn lists_items_sorted_with_attributes() {
    let d = daemon!();
    let login = d.service().await.default_collection().await.unwrap();
    for (label, user) in [("zeta", "z"), ("Alpha", "a"), ("beta", "b")] {
        login
            .create_item(
                label,
                &[("xdg:schema", "org.example.Password"), ("user", user)],
                Secret::text(format!("pw-{user}")),
                true,
                None,
            )
            .await
            .unwrap();
    }

    let keyring = d.keyring().await;
    let items = keyring.items(login.path().as_str()).await.unwrap();
    let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, ["Alpha", "beta", "zeta"], "case-insensitive order");

    let alpha = &items[0];
    assert_eq!(alpha.collection_id, login.path().as_str());
    assert_eq!(alpha.schema.as_deref(), Some("org.example.Password"));
    assert_eq!(alpha.attributes.get("user").map(String::as_str), Some("a"));
    assert!(alpha.created > 1_700_000_000, "created = {}", alpha.created);
    assert!(alpha.modified >= alpha.created);
    assert!(!alpha.locked);
}

#[tokio::test(flavor = "multi_thread")]
async fn reads_text_and_binary_secrets() {
    let d = daemon!();
    let login = d.service().await.default_collection().await.unwrap();
    let text = login
        .create_item("text", &[("k", "t")], Secret::text("hunter2"), true, None)
        .await
        .unwrap();
    let binary = login
        .create_item(
            "bin",
            &[("k", "b")],
            Secret::blob([0u8, 1, 2, 255]),
            true,
            None,
        )
        .await
        .unwrap();

    let keyring = d.keyring().await;
    let c = login.path().as_str();
    match keyring.secret(c, text.path().as_str()).await.unwrap() {
        SecretValue::Text { value } => assert_eq!(value.as_str(), "hunter2"),
        other => panic!("expected text, got {other:?}"),
    }
    assert_eq!(
        keyring.secret(c, binary.path().as_str()).await.unwrap(),
        SecretValue::Binary { size: 4 }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn locked_collection_reports_locked() {
    let d = daemon!();
    let login = d.service().await.default_collection().await.unwrap();
    let item = login
        .create_item("hidden", &[("k", "v")], Secret::text("s"), true, None)
        .await
        .unwrap();
    login.lock(None).await.unwrap();

    let keyring = d.keyring().await;
    let collections = keyring.collections().await.unwrap();
    let info = collections.iter().find(|c| c.is_default).unwrap();
    assert!(info.locked);
    assert_eq!(info.item_count, 1, "count stays visible while locked");
    assert_eq!(info.label, "Login", "keyring label stays visible");

    let c = login.path().as_str();
    assert_eq!(keyring.items(c).await, Err(KeyringError::Locked));
    assert_eq!(
        keyring.secret(c, item.path().as_str()).await,
        Err(KeyringError::Locked)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_ids_are_not_found() {
    let d = daemon!();
    let keyring = d.keyring().await;
    let login = d.service().await.default_collection().await.unwrap();
    let c = login.path().as_str();

    assert_eq!(
        keyring
            .items("/org/freedesktop/secrets/collection/nope")
            .await,
        Err(KeyringError::NotFound)
    );
    assert_eq!(
        keyring.items("not a path").await,
        Err(KeyringError::NotFound)
    );
    assert_eq!(
        keyring.secret(c, &format!("{c}/999")).await,
        Err(KeyringError::NotFound)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_service_is_unavailable() {
    let bus = TestBus::start();
    let err = Keyring::with_connection(&bus.connect().await)
        .await
        .expect_err("no Secret Service on this bus");
    assert!(
        matches!(err, KeyringError::Unavailable(_)),
        "expected Unavailable, got {err:?}"
    );
}
