//! A throwaway Secret Service for integration tests: a private dbus-daemon
//! plus an `oo7-daemon` whose data, runtime and PAM-socket paths all live in a
//! temporary directory. Nothing touches the user's real keyring or bus.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oo7::zbus;

pub const TEST_PASSWORD: &str = "moneypenny-test-password";
const SECRETS_NAME: &str = "org.freedesktop.secrets";

fn daemon_binary() -> PathBuf {
    std::env::var_os("OO7_DAEMON")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/libexec/oo7-daemon"))
}

/// Unix socket paths are limited to ~108 bytes, so prefer the short,
/// user-private runtime dir for the temp root.
fn temp_root() -> tempfile::TempDir {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    tempfile::Builder::new()
        .prefix("mp-test.")
        .tempdir_in(base)
        .expect("create temp dir")
}

/// Environment for child processes: cleared, then pointed at the temp dir.
fn isolated_env(cmd: &mut Command, root: &Path) {
    cmd.env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", root)
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_RUNTIME_DIR", root.join("rt"))
        .env("OO7_PAM_SOCKET", root.join("rt/pam.sock"))
        // No system bus, so the daemon can't ask logind about callers.
        .env(
            "DBUS_SYSTEM_BUS_ADDRESS",
            format!("unix:path={}", root.join("no-system-bus").display()),
        );
}

pub struct TestBus {
    root: tempfile::TempDir,
    bus: Child,
    pub address: String,
}

impl TestBus {
    pub fn start() -> Self {
        let root = temp_root();
        for dir in ["data", "config", "rt"] {
            std::fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        // No <servicedir>: nothing can be D-Bus-activated on this bus.
        let config = root.path().join("bus.conf");
        std::fs::write(
            &config,
            format!(
                r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:dir={}</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>"#,
                root.path().display()
            ),
        )
        .unwrap();

        let mut cmd = Command::new("dbus-daemon");
        isolated_env(&mut cmd, root.path());
        let mut bus = cmd
            .arg(format!("--config-file={}", config.display()))
            .args(["--nofork", "--nopidfile", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn dbus-daemon");
        let mut address = String::new();
        BufReader::new(bus.stdout.take().unwrap())
            .read_line(&mut address)
            .expect("read bus address");
        let address = address.trim().to_owned();
        assert!(address.starts_with("unix:"), "bad bus address {address:?}");
        Self { root, bus, address }
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub async fn connect(&self) -> zbus::Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .method_timeout(Duration::from_secs(10))
            .build()
            .await
            .expect("connect to test bus")
    }
}

impl Drop for TestBus {
    fn drop(&mut self) {
        let _ = self.bus.kill();
        let _ = self.bus.wait();
    }
}

pub struct TestDaemon {
    daemon: Child,
    // Dropped after the daemon (field order), so the bus outlives it.
    pub bus: TestBus,
}

impl TestDaemon {
    /// Starts an isolated oo7-daemon with an unlocked login keyring, or
    /// returns `None` (skipping the test) if oo7-daemon isn't installed.
    pub async fn start() -> Option<Self> {
        let binary = daemon_binary();
        if !binary.exists() {
            eprintln!("skipping: {} not found (set OO7_DAEMON)", binary.display());
            return None;
        }
        let bus = TestBus::start();
        let log = std::fs::File::create(bus.root().join("daemon.log")).unwrap();
        let mut cmd = Command::new(&binary);
        isolated_env(&mut cmd, bus.root());
        let mut daemon = cmd
            .env("DBUS_SESSION_BUS_ADDRESS", &bus.address)
            .arg("--login")
            .stdin(Stdio::piped())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .expect("spawn oo7-daemon");
        {
            let mut stdin = daemon.stdin.take().unwrap();
            stdin.write_all(TEST_PASSWORD.as_bytes()).unwrap();
        } // closes stdin

        let this = Self { daemon, bus };
        this.wait_for_name().await;
        Some(this)
    }

    async fn wait_for_name(&self) {
        let cnx = self.bus.connect().await;
        let dbus = zbus::fdo::DBusProxy::new(&cnx).await.unwrap();
        let name = zbus::names::BusName::try_from(SECRETS_NAME).unwrap();
        for _ in 0..100 {
            if dbus.name_has_owner(name.clone()).await.unwrap_or(false) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let log = std::fs::read_to_string(self.bus.root().join("daemon.log")).unwrap_or_default();
        panic!("oo7-daemon did not start:\n{log}");
    }

    pub async fn keyring(&self) -> moneypenny_lib::keyring::Keyring {
        moneypenny_lib::keyring::Keyring::with_connection(&self.bus.connect().await)
            .await
            .expect("connect keyring")
    }

    /// Raw oo7 client, for seeding test data.
    pub async fn service(&self) -> oo7::dbus::Service {
        oo7::dbus::Service::new_with_connection(&self.bus.connect().await)
            .await
            .expect("connect service")
    }
}

impl TestDaemon {
    /// Restarts oo7-daemon on the same bus and data (like a crash + restart).
    pub async fn restart_daemon(&mut self) {
        self.stop_daemon();
        let log = std::fs::File::create(self.bus.root().join("daemon2.log")).unwrap();
        let mut cmd = Command::new(daemon_binary());
        isolated_env(&mut cmd, self.bus.root());
        let mut daemon = cmd
            .env("DBUS_SESSION_BUS_ADDRESS", &self.bus.address)
            .arg("--login")
            .stdin(Stdio::piped())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .expect("respawn oo7-daemon");
        daemon
            .stdin
            .take()
            .unwrap()
            .write_all(TEST_PASSWORD.as_bytes())
            .unwrap();
        self.daemon = daemon;
        self.wait_for_name().await;
    }

    /// Stops oo7-daemon but keeps the bus running.
    pub fn stop_daemon(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

impl Drop for TestDaemon {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

/// One answer from the fake prompter: `Some(password)` accepts, `None` cancels.
pub type PromptReply = Option<String>;

/// Stands in for the password dialog. oo7-daemon falls back to this D-Bus
/// "CLI prompter" for callers outside a graphical session, see
/// [`run_headless`].
#[derive(Clone, Default)]
pub struct FakePrompter {
    replies: Arc<Mutex<VecDeque<PromptReply>>>,
    /// Labels of the keyrings the daemon prompted for, in order.
    pub prompts: Arc<Mutex<Vec<String>>>,
}

impl FakePrompter {
    pub fn reply(&self, reply: Option<&str>) -> &Self {
        self.replies
            .lock()
            .unwrap()
            .push_back(reply.map(str::to_owned));
        self
    }

    pub fn prompted(&self) -> Vec<String> {
        self.prompts.lock().unwrap().clone()
    }
}

#[zbus::interface(name = "org.freedesktop.secrets.CliPrompter")]
impl FakePrompter {
    async fn prompt(
        &self,
        label: &str,
        _description: &str,
    ) -> zbus::fdo::Result<(zbus::OwnedFd, bool)> {
        self.prompts.lock().unwrap().push(label.to_owned());
        let reply = self.replies.lock().unwrap().pop_front().flatten();
        let (mut ours, theirs) =
            UnixStream::pair().map_err(|e| zbus::fdo::Error::IOError(e.to_string()))?;
        if let Some(password) = &reply {
            ours.write_all(password.as_bytes())
                .map_err(|e| zbus::fdo::Error::IOError(e.to_string()))?;
        }
        drop(ours); // EOF tells the daemon the password is complete.
        let fd: std::os::fd::OwnedFd = theirs.into();
        Ok((fd.into(), reply.is_some()))
    }
}

impl TestDaemon {
    /// Serves a [`FakePrompter`] on the test bus. Keep the returned
    /// connection alive for as long as prompts may happen.
    pub async fn prompter(&self) -> (zbus::Connection, FakePrompter) {
        let prompter = FakePrompter::default();
        let cnx = zbus::connection::Builder::address(self.bus.address.as_str())
            .name("org.freedesktop.secrets.CliPrompter")
            .serve_at("/org/freedesktop/secrets/CliPrompter", prompter.clone())
            .build()
            .await
            .expect("serve fake prompter");
        (cnx, prompter)
    }
}

/// oo7-daemon only uses the CLI prompter for callers outside a graphical
/// session, which it decides from the caller's `WAYLAND_DISPLAY`/`DISPLAY`.
/// A process can't remove those from its own `/proc/<pid>/environ`, so when
/// they're set this re-runs `test` in a child process without them and
/// returns `false` (the caller should return). Returns `true` when the test
/// body should run here.
pub fn run_headless(test: &str) -> bool {
    const CHILD: &str = "MONEYPENNY_HEADLESS_CHILD";
    let graphical =
        std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some();
    if std::env::var_os(CHILD).is_some() || !graphical {
        return true;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .output()
        .expect("re-run test headless");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "headless run of {test} failed:\n{stdout}\n{stderr}"
    );
    false
}
