//! Integration tests for keyring changes, against a private oo7-daemon.
//! Tests that need a password prompt answer it with a fake prompter and run
//! headless (see `common::run_headless`).

mod common;

use std::collections::BTreeMap;
use std::time::Duration;

use common::{TEST_PASSWORD, TestDaemon, run_headless};
use moneypenny_lib::keyring::{KeyringError, SecretValue};
use oo7::Secret;
use zeroize::Zeroizing;

macro_rules! daemon {
    () => {
        match TestDaemon::start().await {
            Some(d) => d,
            None => return,
        }
    };
}

/// Skips to a headless child process when needed, then starts a daemon.
macro_rules! headless_daemon {
    ($name:literal) => {{
        if !run_headless($name) {
            return;
        }
        daemon!()
    }};
}

async fn seed_item(d: &TestDaemon) {
    d.service()
        .await
        .default_collection()
        .await
        .unwrap()
        .create_item("seed", &[("seed", "1")], Secret::text("seed"), false, None)
        .await
        .unwrap();
}

async fn login_id(d: &TestDaemon) -> String {
    d.service()
        .await
        .default_collection()
        .await
        .unwrap()
        .path()
        .to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn lock_then_unlock_through_prompt() {
    let d = headless_daemon!("lock_then_unlock_through_prompt");
    let (_cnx, prompter) = d.prompter().await;
    let keyring = d.keyring().await;
    let login = login_id(&d).await;

    keyring.lock(&login).await.unwrap();
    assert_eq!(keyring.items(&login).await, Err(KeyringError::Locked));

    prompter.reply(Some(TEST_PASSWORD));
    keyring.unlock(&login).await.unwrap();
    assert_eq!(prompter.prompted(), ["Login"]);
    assert!(keyring.items(&login).await.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelled_unlock_is_dismissed() {
    let d = headless_daemon!("cancelled_unlock_is_dismissed");
    let (_cnx, prompter) = d.prompter().await;
    let keyring = d.keyring().await;
    let login = login_id(&d).await;

    keyring.lock(&login).await.unwrap();
    prompter.reply(None);
    assert_eq!(keyring.unlock(&login).await, Err(KeyringError::Dismissed));
    assert_eq!(keyring.items(&login).await, Err(KeyringError::Locked));
}

#[tokio::test(flavor = "multi_thread")]
async fn creates_and_deletes_a_keyring() {
    let d = headless_daemon!("creates_and_deletes_a_keyring");
    let (_cnx, prompter) = d.prompter().await;
    let keyring = d.keyring().await;

    prompter.reply(Some("work-password"));
    let id = keyring.create_collection("  Work  ").await.unwrap();
    let collections = keyring.collections().await.unwrap();
    let work = collections
        .iter()
        .find(|c| c.id == id)
        .expect("new keyring listed");
    assert_eq!(work.label, "Work", "label is trimmed");
    assert!(!work.locked);
    assert!(!work.is_default);

    keyring.delete_collection(&id).await.unwrap();
    let collections = keyring.collections().await.unwrap();
    assert!(collections.iter().all(|c| c.id != id), "keyring deleted");
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelled_keyring_creation_creates_nothing() {
    let d = headless_daemon!("cancelled_keyring_creation_creates_nothing");
    let (_cnx, prompter) = d.prompter().await;
    let keyring = d.keyring().await;
    let before = keyring.collections().await.unwrap().len();

    prompter.reply(None);
    assert_eq!(
        keyring.create_collection("Nope").await,
        Err(KeyringError::Dismissed)
    );
    assert_eq!(keyring.collections().await.unwrap().len(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_empty_names() {
    let d = daemon!();
    let keyring = d.keyring().await;
    let login = login_id(&d).await;

    assert!(matches!(
        keyring.create_collection("   ").await,
        Err(KeyringError::Invalid(_))
    ));
    assert!(matches!(
        keyring
            .create_item(&login, "", BTreeMap::new(), Zeroizing::new("s".into()))
            .await,
        Err(KeyringError::Invalid(_))
    ));
    let attrs = BTreeMap::from([(" ".to_owned(), "v".to_owned())]);
    assert!(matches!(
        keyring
            .create_item(&login, "x", attrs, Zeroizing::new("s".into()))
            .await,
        Err(KeyringError::Invalid(_))
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn changes_password_through_prompt() {
    let d = headless_daemon!("changes_password_through_prompt");
    let (_cnx, prompter) = d.prompter().await;
    let keyring = d.keyring().await;
    let login = login_id(&d).await;
    // oo7 can only check a password against an encrypted item, so the
    // keyring needs one for password checks to mean anything.
    seed_item(&d).await;

    // Locked first: unlocks with the old password, then asks for the new one.
    keyring.lock(&login).await.unwrap();
    prompter
        .reply(Some(TEST_PASSWORD))
        .reply(Some("brand-new-password"));
    keyring.change_password(&login).await.unwrap();
    assert_eq!(prompter.prompted().len(), 2);

    // The new password works...
    keyring.lock(&login).await.unwrap();
    prompter.reply(Some("brand-new-password"));
    keyring.unlock(&login).await.unwrap();
    assert_eq!(keyring.items(&login).await.unwrap().len(), 1);

    // ...and the old one doesn't. oo7-daemon's CLI prompter never completes
    // the prompt after a wrong password (the GNOME prompter retries), so
    // this must be the last unlock in the test.
    keyring.lock(&login).await.unwrap();
    prompter.reply(Some(TEST_PASSWORD));
    let attempt = tokio::time::timeout(Duration::from_secs(2), keyring.unlock(&login)).await;
    assert!(
        !matches!(attempt, Ok(Ok(()))),
        "old password must not unlock"
    );
    assert_eq!(keyring.items(&login).await, Err(KeyringError::Locked));
}

#[tokio::test(flavor = "multi_thread")]
async fn creates_edits_and_deletes_items() {
    let d = daemon!();
    let keyring = d.keyring().await;
    let login = login_id(&d).await;

    let attrs = BTreeMap::from([("service".to_owned(), "example.com".to_owned())]);
    let created = keyring
        .create_item(
            &login,
            "Example",
            attrs.clone(),
            Zeroizing::new("first".into()),
        )
        .await
        .unwrap();
    assert_eq!(created.label, "Example");
    assert_eq!(
        created.schema.as_deref(),
        Some("org.gnome.keyring.Note"),
        "default schema added"
    );
    assert_eq!(
        created.attributes.get("service").map(String::as_str),
        Some("example.com")
    );

    // Same attributes again must not replace the first item.
    let second = keyring
        .create_item(&login, "Example 2", attrs, Zeroizing::new("second".into()))
        .await
        .unwrap();
    assert_ne!(second.id, created.id);
    assert_eq!(keyring.items(&login).await.unwrap().len(), 2);

    // Edit label, attributes and secret.
    let new_attrs = BTreeMap::from([
        ("xdg:schema".to_owned(), "org.gnome.keyring.Note".to_owned()),
        ("user".to_owned(), "bob".to_owned()),
    ]);
    let updated = keyring
        .update_item(
            &login,
            &created.id,
            moneypenny_lib::keyring::ItemUpdate {
                label: Some("Renamed".into()),
                attributes: Some(new_attrs),
                secret: Some(Zeroizing::new("changed".into())),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.label, "Renamed");
    assert_eq!(
        updated.attributes.get("user").map(String::as_str),
        Some("bob")
    );
    assert!(
        !updated.attributes.contains_key("service"),
        "attributes replaced"
    );
    match keyring.secret(&login, &created.id).await.unwrap() {
        SecretValue::Text { value } => assert_eq!(value.as_str(), "changed"),
        other => panic!("expected text, got {other:?}"),
    }

    // A label-only update leaves the secret alone.
    keyring
        .update_item(
            &login,
            &created.id,
            moneypenny_lib::keyring::ItemUpdate {
                label: Some("Renamed again".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    match keyring.secret(&login, &created.id).await.unwrap() {
        SecretValue::Text { value } => assert_eq!(value.as_str(), "changed"),
        other => panic!("expected text, got {other:?}"),
    }

    keyring.delete_item(&login, &created.id).await.unwrap();
    let remaining = keyring.items(&login).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, second.id);
    assert_eq!(
        keyring.delete_item(&login, &created.id).await,
        Err(KeyringError::NotFound)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn items_in_locked_keyrings_cannot_be_changed() {
    let d = daemon!();
    let keyring = d.keyring().await;
    let service = d.service().await;
    let login = service.default_collection().await.unwrap();
    let item = login
        .create_item("x", &[("k", "v")], Secret::text("s"), false, None)
        .await
        .unwrap();
    let (c, i) = (login.path().to_string(), item.path().to_string());
    login.lock(None).await.unwrap();

    assert_eq!(
        keyring
            .create_item(&c, "y", BTreeMap::new(), Zeroizing::new("s".into()))
            .await,
        Err(KeyringError::Locked)
    );
    assert_eq!(
        keyring
            .update_item(
                &c,
                &i,
                moneypenny_lib::keyring::ItemUpdate {
                    label: Some("z".into()),
                    ..Default::default()
                }
            )
            .await,
        Err(KeyringError::Locked)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_attribute_names_that_collide_after_trimming() {
    let d = daemon!();
    let keyring = d.keyring().await;
    let login = login_id(&d).await;
    let attrs = BTreeMap::from([
        ("user".to_owned(), "a".to_owned()),
        (" user ".to_owned(), "b".to_owned()),
    ]);
    assert!(matches!(
        keyring
            .create_item(&login, "x", attrs, Zeroizing::new("s".into()))
            .await,
        Err(KeyringError::Invalid(_))
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_fails_instead_of_hanging_when_the_daemon_exits() {
    let mut d = headless_daemon!("prompt_fails_instead_of_hanging_when_the_daemon_exits");
    let (_cnx, prompter) = d.prompter().await;
    let keyring = d.keyring().await;
    let login = login_id(&d).await;
    seed_item(&d).await;
    keyring.lock(&login).await.unwrap();

    // A wrong password leaves oo7-daemon's CLI prompt open indefinitely.
    prompter.reply(Some("wrong password"));
    let unlock = tokio::spawn(async move { keyring.unlock(&login).await });
    tokio::time::sleep(Duration::from_millis(500)).await;
    d.stop_daemon();

    let result = tokio::time::timeout(Duration::from_secs(5), unlock)
        .await
        .expect("unlock returned after the daemon exited")
        .unwrap();
    assert!(
        matches!(result, Err(KeyringError::Unavailable(_))),
        "got {result:?}"
    );
}
