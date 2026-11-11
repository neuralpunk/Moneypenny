//! Live-update tests: changes made by another client are reported.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use common::TestDaemon;
use moneypenny_lib::watch::{Change, watch_changes};
use oo7::Secret;

/// Starts the watcher on the test bus and returns a change counter.
async fn watch(d: &TestDaemon) -> Arc<AtomicUsize> {
    let count = Arc::new(AtomicUsize::new(0));
    let cnx = d.bus.connect().await;
    let c = Arc::clone(&count);
    tokio::spawn(async move {
        let _ = watch_changes(&cnx, move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        })
        .await;
    });
    tokio::time::sleep(Duration::from_millis(200)).await; // let match rules register
    count
}

async fn wait_for(count: &AtomicUsize, at_least: usize) -> usize {
    for _ in 0..60 {
        let n = count.load(Ordering::SeqCst);
        if n >= at_least {
            return n;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    count.load(Ordering::SeqCst)
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_item_changes_from_other_clients() {
    let Some(d) = TestDaemon::start().await else {
        return;
    };
    let count = watch(&d).await;
    let login = d.service().await.default_collection().await.unwrap();

    login
        .create_item("new", &[("k", "v")], Secret::text("s"), false, None)
        .await
        .unwrap();
    assert!(wait_for(&count, 1).await >= 1, "item creation reported");

    let before = count.load(Ordering::SeqCst);
    login.lock(None).await.unwrap();
    assert!(
        wait_for(&count, before + 1).await > before,
        "locking reported"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn coalesces_bursts() {
    let Some(d) = TestDaemon::start().await else {
        return;
    };
    let count = watch(&d).await;
    let login = d.service().await.default_collection().await.unwrap();

    for i in 0..10 {
        login
            .create_item(
                &format!("item {i}"),
                &[("i", i.to_string().as_str())],
                Secret::text("s"),
                false,
                None,
            )
            .await
            .unwrap();
    }
    let n = wait_for(&count, 1).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    let n = n.max(count.load(Ordering::SeqCst));
    assert!(
        (1..10).contains(&n),
        "10 quick changes became {n} refreshes"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn reports_the_daemon_going_away() {
    let Some(mut d) = TestDaemon::start().await else {
        return;
    };
    let saw_daemon = Arc::new(AtomicBool::new(false));
    let cnx = d.bus.connect().await;
    let flag = Arc::clone(&saw_daemon);
    tokio::spawn(async move {
        let _ = watch_changes(&cnx, move |change: Change| {
            flag.fetch_or(change.daemon_changed, Ordering::SeqCst);
        })
        .await;
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    d.stop_daemon();
    for _ in 0..60 {
        if saw_daemon.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("daemon exit not reported as a daemon change");
}
