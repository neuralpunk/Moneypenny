//! Read access to the Secret Service (oo7-daemon) through oo7's own D-Bus
//! client library.
//!
//! Collections and items are identified by their D-Bus object paths, which are
//! stable for the lifetime of the daemon and are what other Secret Service
//! clients use too.
//!
//! oo7 hides an item's label, attributes and timestamps while its collection
//! is locked; only the number of items is visible. Listing the items of a
//! locked collection therefore fails with [`KeyringError::Locked`] rather than
//! returning partial data.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use futures_util::StreamExt;
use futures_util::future::{Either, join_all, select};
use oo7::dbus::{Collection, Item, Service, ServiceError};
use oo7::zbus::{ObjectPath, OwnedObjectPath};
use oo7::{Secret, zbus};
use serde::{Serialize, Serializer};
use zeroize::Zeroizing;

/// Attribute that libsecret uses to record which schema an item follows.
pub const SCHEMA_ATTRIBUTE: &str = "xdg:schema";

/// Schema given to items created in Moneypenny when none is set: libsecret's
/// note schema, which Seahorse also uses for passwords it creates.
pub const NEW_ITEM_SCHEMA: &str = "org.gnome.keyring.Note";

const SECRETS_NAME: &str = "org.freedesktop.secrets";
const SECRETS_PATH: &str = "/org/freedesktop/secrets";
/// gnome-keyring's private interface, which oo7-daemon also implements; it's
/// the only way to change a keyring password through the daemon's prompt.
const INTERNAL_INTERFACE: &str = "org.gnome.keyring.InternalUnsupportedGuiltRiddenInterface";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "kebab-case")]
pub enum KeyringError {
    #[error("The Secret Service is not available: {0}")]
    Unavailable(String),
    #[error("The keyring is locked")]
    Locked,
    #[error("The keyring or item no longer exists")]
    NotFound,
    #[error("The request was dismissed")]
    Dismissed,
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Backend(String),
}

pub type Result<T, E = KeyringError> = std::result::Result<T, E>;

/// D-Bus error names we map to specific variants.
const DBUS_SERVICE_UNKNOWN: &str = "org.freedesktop.DBus.Error.ServiceUnknown";
const DBUS_NAME_HAS_NO_OWNER: &str = "org.freedesktop.DBus.Error.NameHasNoOwner";
const DBUS_UNKNOWN_OBJECT: &str = "org.freedesktop.DBus.Error.UnknownObject";
const SECRET_IS_LOCKED: &str = "org.freedesktop.Secret.Error.IsLocked";
const SECRET_NO_SUCH_OBJECT: &str = "org.freedesktop.Secret.Error.NoSuchObject";
const SECRET_NO_SESSION: &str = "org.freedesktop.Secret.Error.NoSession";

fn from_zbus(err: &zbus::Error) -> KeyringError {
    let name = match err {
        zbus::Error::MethodError(name, _, _) => Some(name.as_str().to_owned()),
        zbus::Error::FDO(fdo) => Some(zbus::DBusError::name(fdo.as_ref()).as_str().to_owned()),
        _ => None,
    };
    let message = err.to_string();
    match name.as_deref() {
        Some(DBUS_SERVICE_UNKNOWN | DBUS_NAME_HAS_NO_OWNER | SECRET_NO_SESSION) => {
            KeyringError::Unavailable(message)
        }
        Some(SECRET_IS_LOCKED) => KeyringError::Locked,
        Some(DBUS_UNKNOWN_OBJECT | SECRET_NO_SUCH_OBJECT) => KeyringError::NotFound,
        // oo7-daemon reports property reads on locked items as a generic
        // failure whose message mentions the lock.
        _ if message.contains("of a locked object") => KeyringError::Locked,
        // Cancelling oo7-daemon's CLI prompter fails the Prompt call instead
        // of completing the prompt as dismissed.
        _ if message.contains("Prompt dismissed") => KeyringError::Dismissed,
        _ => match err {
            zbus::Error::Address(_) | zbus::Error::InputOutput(_) => {
                KeyringError::Unavailable(message)
            }
            _ => KeyringError::Backend(message),
        },
    }
}

impl From<oo7::dbus::Error> for KeyringError {
    fn from(err: oo7::dbus::Error) -> Self {
        use oo7::dbus::Error as E;
        match err {
            E::ZBus(e) | E::Service(ServiceError::ZBus(e)) => from_zbus(&e),
            E::Service(ServiceError::IsLocked(_)) => Self::Locked,
            E::Service(ServiceError::NoSuchObject(_)) | E::Deleted | E::NotFound(_) => {
                Self::NotFound
            }
            E::Service(ServiceError::NoSession(msg)) => Self::Unavailable(msg),
            E::Dismissed => Self::Dismissed,
            other => Self::Backend(other.to_string()),
        }
    }
}

impl From<zbus::Error> for KeyringError {
    fn from(err: zbus::Error) -> Self {
        from_zbus(&err)
    }
}

impl KeyringError {
    /// Whether the cached service connection should be dropped and rebuilt,
    /// e.g. after the daemon restarted.
    pub fn is_connection_lost(&self) -> bool {
        matches!(self, Self::Unavailable(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionInfo {
    /// D-Bus object path.
    pub id: String,
    pub label: String,
    pub locked: bool,
    /// Holds the `default` alias: where apps store new secrets.
    pub is_default: bool,
    /// The in-memory `session` keyring, cleared at logout.
    pub is_session: bool,
    /// Visible even while locked.
    pub item_count: usize,
    /// Seconds since the Unix epoch; 0 if unknown.
    pub created: u64,
    pub modified: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInfo {
    /// D-Bus object path.
    pub id: String,
    pub collection_id: String,
    pub label: String,
    pub locked: bool,
    /// Sorted for stable display.
    pub attributes: BTreeMap<String, String>,
    /// Value of the `xdg:schema` attribute, if present.
    pub schema: Option<String>,
    pub created: u64,
    pub modified: u64,
}

/// A text value that is wiped from memory when dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretText(Zeroizing<String>);

impl SecretText {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretText(<redacted>)")
    }
}

impl Serialize for SecretText {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecretValue {
    Text {
        value: SecretText,
    },
    /// Non-text secrets (keys, tokens stored as raw bytes) aren't sent to the UI.
    Binary {
        size: usize,
    },
}

impl SecretValue {
    /// Text if the bytes are readable UTF-8, regardless of the declared content
    /// type: many apps store passwords without `text/plain`.
    pub fn classify(secret: &Secret) -> Self {
        match secret {
            Secret::Text(text) => Self::text(text),
            Secret::Blob(bytes) => match std::str::from_utf8(bytes) {
                Ok(text) if is_displayable(text) => Self::text(text),
                _ => Self::Binary { size: bytes.len() },
            },
        }
    }

    fn text(text: &str) -> Self {
        Self::Text {
            value: SecretText(Zeroizing::new(text.to_owned())),
        }
    }
}

fn is_displayable(text: &str) -> bool {
    text.chars()
        .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
}

fn epoch_secs(d: Duration) -> u64 {
    d.as_secs()
}

/// Changes to apply to an item; `None` leaves that part unchanged.
#[derive(Debug, Default)]
pub struct ItemUpdate {
    pub label: Option<String>,
    pub attributes: Option<BTreeMap<String, String>>,
    pub secret: Option<Zeroizing<String>>,
}

fn valid_label(label: &str, what: &str) -> Result<String> {
    let label = label.trim();
    if label.is_empty() {
        return Err(KeyringError::Invalid(format!("{what} name can't be empty")));
    }
    Ok(label.to_owned())
}

fn valid_attributes(attributes: BTreeMap<String, String>) -> Result<HashMap<String, String>> {
    let mut out = HashMap::with_capacity(attributes.len());
    for (key, value) in attributes {
        let key = key.trim().to_owned();
        if key.is_empty() {
            return Err(KeyringError::Invalid(
                "Attribute names can't be empty".into(),
            ));
        }
        if out.contains_key(&key) {
            return Err(KeyringError::Invalid(format!(
                "There are two attributes named “{key}”"
            )));
        }
        out.insert(key, value);
    }
    Ok(out)
}

/// Shows a Secret Service prompt and waits for it to complete. "/" means no
/// prompt was needed.
async fn run_prompt(cnx: &zbus::Connection, path: OwnedObjectPath) -> Result<()> {
    if path.as_str() == "/" {
        return Ok(());
    }
    let prompt = zbus::Proxy::new(cnx, SECRETS_NAME, path, "org.freedesktop.Secret.Prompt").await?;
    // Subscribe before prompting so the Completed signal can't be missed.
    let mut completed = prompt.receive_signal("Completed").await?;
    prompt.call_method("Prompt", &("",)).await?;
    let message = completed
        .next()
        .await
        .ok_or_else(|| KeyringError::Backend("The prompt ended unexpectedly".into()))?;
    let (dismissed, _result): (bool, zbus::OwnedVariant) = message.body().deserialize()?;
    if dismissed {
        Err(KeyringError::Dismissed)
    } else {
        Ok(())
    }
}

/// Drops entries that vanished while a listing was being read (another app
/// deleted them mid-way), instead of failing the whole listing.
fn skip_vanished<T>(results: Vec<Result<T>>) -> Result<Vec<T>> {
    results
        .into_iter()
        .filter(|r| !matches!(r, Err(KeyringError::NotFound)))
        .collect()
}

fn object_path(id: &str) -> Result<ObjectPath<'_>> {
    ObjectPath::try_from(id).map_err(|_| KeyringError::NotFound)
}

/// A connected Secret Service client.
///
/// Operations that need a password (unlocking, creating a keyring, changing
/// its password) go through oo7-daemon's prompt, which on GNOME is shown by
/// the shell. Moneypenny never sees keyring passwords.
#[derive(Debug)]
pub struct Keyring {
    cnx: zbus::Connection,
    service: Service,
}

impl Keyring {
    /// Connects to the user's session bus.
    pub async fn connect() -> Result<Self> {
        let cnx = zbus::connection::Builder::session()
            .method_timeout(Duration::from_secs(30))
            .build()
            .await?;
        Self::with_connection(&cnx).await
    }

    /// Connects over an existing D-Bus connection (used by tests to talk to a
    /// private daemon).
    pub async fn with_connection(cnx: &zbus::Connection) -> Result<Self> {
        Ok(Self {
            cnx: cnx.clone(),
            service: Service::new_with_connection(cnx).await?,
        })
    }

    async fn alias_path(&self, alias: &str) -> Result<Option<String>> {
        Ok(self
            .service
            .with_alias(alias)
            .await?
            .map(|c| c.path().to_string()))
    }

    pub async fn collections(&self) -> Result<Vec<CollectionInfo>> {
        let default = self.alias_path(Service::DEFAULT_COLLECTION).await?;
        let session = self.alias_path(Service::SESSION_COLLECTION).await?;
        let collections = self.service.collections().await?;
        let mut infos = skip_vanished(
            join_all(
                collections
                    .iter()
                    .map(|c| collection_info(c, default.as_deref(), session.as_deref())),
            )
            .await,
        )?;
        // Default first, session last, the rest alphabetically: like Seahorse.
        infos.sort_by(|a, b| {
            (!a.is_default, a.is_session, a.label.to_lowercase()).cmp(&(
                !b.is_default,
                b.is_session,
                b.label.to_lowercase(),
            ))
        });
        Ok(infos)
    }

    async fn find_collection(&self, collection_id: &str) -> Result<Collection> {
        object_path(collection_id)?;
        self.service
            .collections()
            .await?
            .into_iter()
            .find(|c| c.path().as_str() == collection_id)
            .ok_or(KeyringError::NotFound)
    }

    async fn find_item(&self, collection_id: &str, item_id: &str) -> Result<Item> {
        let collection = self.find_collection(collection_id).await?;
        collection
            .items()
            .await?
            .into_iter()
            .find(|i| i.path().as_str() == item_id)
            .ok_or(KeyringError::NotFound)
    }

    pub async fn items(&self, collection_id: &str) -> Result<Vec<ItemInfo>> {
        let collection = self.find_collection(collection_id).await?;
        if collection.is_locked().await? {
            return Err(KeyringError::Locked);
        }
        let items = collection.items().await?;
        let mut infos =
            skip_vanished(join_all(items.iter().map(|i| item_info(i, collection_id))).await)?;
        infos.sort_by(|a, b| {
            a.label
                .to_lowercase()
                .cmp(&b.label.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(infos)
    }

    /// Runs an operation that may show a password prompt, failing with
    /// [`KeyringError::Unavailable`] if oo7-daemon exits meanwhile. Without
    /// this, a prompt whose daemon died would be waited on forever.
    async fn while_daemon_runs<T>(&self, op: impl Future<Output = Result<T>>) -> Result<T> {
        let dbus = zbus::fdo::DBusProxy::new(&self.cnx).await?;
        let mut owner_changed = dbus
            .receive_name_owner_changed_with_args(&[(0, SECRETS_NAME)])
            .await?;
        match select(std::pin::pin!(op), owner_changed.next()).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => Err(KeyringError::Unavailable(
                "The keyring service stopped".into(),
            )),
        }
    }

    /// Unlocks a keyring, prompting for its password. Returns
    /// [`KeyringError::Dismissed`] if the user cancels.
    pub async fn unlock(&self, collection_id: &str) -> Result<()> {
        let collection = self.find_collection(collection_id).await?;
        if collection.is_locked().await? {
            self.while_daemon_runs(async { Ok(collection.unlock(None).await?) })
                .await?;
        }
        Ok(())
    }

    pub async fn lock(&self, collection_id: &str) -> Result<()> {
        self.find_collection(collection_id)
            .await?
            .lock(None)
            .await?;
        Ok(())
    }

    /// Creates a keyring; the daemon prompts for its password. Returns its id.
    pub async fn create_collection(&self, label: &str) -> Result<String> {
        let label = valid_label(label, "Keyring")?;
        let collection = self
            .while_daemon_runs(async {
                Ok(self.service.create_collection(&label, None, None).await?)
            })
            .await?;
        Ok(collection.path().to_string())
    }

    /// Deletes a keyring and everything in it. The daemon prompts to unlock
    /// it first if it's locked.
    pub async fn delete_collection(&self, collection_id: &str) -> Result<()> {
        let collection = self.find_collection(collection_id).await?;
        self.while_daemon_runs(async { Ok(collection.delete(None).await?) })
            .await
    }

    /// Changes a keyring's password through the daemon's prompt, unlocking
    /// it first (which prompts for the current password) if needed.
    pub async fn change_password(&self, collection_id: &str) -> Result<()> {
        let collection = self.find_collection(collection_id).await?;
        self.while_daemon_runs(self.change_password_inner(&collection))
            .await
    }

    async fn change_password_inner(&self, collection: &Collection) -> Result<()> {
        if collection.is_locked().await? {
            collection.unlock(None).await?;
        }
        let prompt_path: OwnedObjectPath = self
            .cnx
            .call_method(
                Some(SECRETS_NAME),
                SECRETS_PATH,
                Some(INTERNAL_INTERFACE),
                "ChangeWithPrompt",
                &(collection.path()),
            )
            .await?
            .body()
            .deserialize()?;
        run_prompt(&self.cnx, prompt_path).await
    }

    /// Adds a text secret. Never replaces an existing item, even one with the
    /// same attributes.
    pub async fn create_item(
        &self,
        collection_id: &str,
        label: &str,
        attributes: BTreeMap<String, String>,
        secret: Zeroizing<String>,
    ) -> Result<ItemInfo> {
        let label = valid_label(label, "Secret")?;
        let mut attributes = valid_attributes(attributes)?;
        attributes
            .entry(SCHEMA_ATTRIBUTE.to_owned())
            .or_insert_with(|| NEW_ITEM_SCHEMA.to_owned());
        let collection = self.find_collection(collection_id).await?;
        if collection.is_locked().await? {
            return Err(KeyringError::Locked);
        }
        let item = collection
            .create_item(
                &label,
                &attributes,
                Secret::text(secret.as_str()),
                false,
                None,
            )
            .await?;
        item_info(&item, collection_id).await
    }

    pub async fn update_item(
        &self,
        collection_id: &str,
        item_id: &str,
        update: ItemUpdate,
    ) -> Result<ItemInfo> {
        let label = update
            .label
            .as_deref()
            .map(|l| valid_label(l, "Secret"))
            .transpose()?;
        let attributes = update.attributes.map(valid_attributes).transpose()?;
        let item = self.find_item(collection_id, item_id).await?;
        if item.is_locked().await? {
            return Err(KeyringError::Locked);
        }
        if let Some(label) = label {
            item.set_label(&label).await?;
        }
        if let Some(attributes) = attributes {
            item.set_attributes(&attributes).await?;
        }
        if let Some(secret) = update.secret {
            item.set_secret(Secret::text(secret.as_str())).await?;
        }
        item_info(&item, collection_id).await
    }

    pub async fn delete_item(&self, collection_id: &str, item_id: &str) -> Result<()> {
        self.find_item(collection_id, item_id)
            .await?
            .delete(None)
            .await?;
        Ok(())
    }

    /// Fetches one secret. Never cached: callers should drop it as soon as
    /// it's been shown or copied.
    pub async fn secret(&self, collection_id: &str, item_id: &str) -> Result<SecretValue> {
        let item = self.find_item(collection_id, item_id).await?;
        if item.is_locked().await? {
            return Err(KeyringError::Locked);
        }
        Ok(SecretValue::classify(&item.secret().await?))
    }
}

async fn collection_info(
    c: &Collection,
    default: Option<&str>,
    session: Option<&str>,
) -> Result<CollectionInfo> {
    let id = c.path().to_string();
    // Item paths are listed even when locked, so the count is always known.
    let item_count = c.items().await?.len();
    Ok(CollectionInfo {
        is_default: default == Some(id.as_str()),
        is_session: session == Some(id.as_str()),
        label: c.label().await?,
        locked: c.is_locked().await?,
        item_count,
        created: c.created().await.map(epoch_secs).unwrap_or(0),
        modified: c.modified().await.map(epoch_secs).unwrap_or(0),
        id,
    })
}

async fn item_info(i: &Item, collection_id: &str) -> Result<ItemInfo> {
    let attributes: BTreeMap<String, String> = i.attributes().await?.into_iter().collect();
    Ok(ItemInfo {
        id: i.path().to_string(),
        collection_id: collection_id.to_owned(),
        label: i.label().await?,
        locked: i.is_locked().await?,
        schema: attributes.get(SCHEMA_ATTRIBUTE).cloned(),
        attributes,
        created: epoch_secs(i.created().await?),
        modified: epoch_secs(i.modified().await?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_text_and_binary() {
        let text = SecretValue::classify(&Secret::text("hunter2"));
        assert!(matches!(&text, SecretValue::Text { value } if value.as_str() == "hunter2"));

        // A UTF-8 password stored as a blob is still shown as text.
        let blob_text = SecretValue::classify(&Secret::blob(b"p@ss word\n"));
        assert!(matches!(blob_text, SecretValue::Text { .. }));

        let binary = SecretValue::classify(&Secret::blob([0u8, 159, 146, 150]));
        assert_eq!(binary, SecretValue::Binary { size: 4 });

        let control = SecretValue::classify(&Secret::blob([b'a', 0x07, b'b']));
        assert_eq!(control, SecretValue::Binary { size: 3 });
    }

    #[test]
    fn secret_text_is_redacted_in_debug_but_serialized_for_ui() {
        let value = SecretValue::classify(&Secret::text("hunter2"));
        assert!(!format!("{value:?}").contains("hunter2"));
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type": "text", "value": "hunter2"})
        );
        let bin = serde_json::to_value(SecretValue::Binary { size: 3 }).unwrap();
        assert_eq!(bin, serde_json::json!({"type": "binary", "size": 3}));
    }

    #[test]
    fn errors_serialize_with_kind() {
        let json = serde_json::to_value(KeyringError::Locked).unwrap();
        assert_eq!(json, serde_json::json!({"kind": "locked"}));
        let json = serde_json::to_value(KeyringError::Unavailable("gone".into())).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "unavailable", "message": "gone"})
        );
    }

    #[test]
    fn maps_dbus_error_names() {
        let method_err = |name: &'static str, msg: &str| {
            let name = zbus::names::OwnedErrorName::try_from(name).unwrap();
            let msg_obj = zbus::message::Message::method_call("/", "Ping")
                .build(&())
                .unwrap();
            zbus::Error::MethodError(name, Some(msg.to_owned()), msg_obj)
        };
        assert_eq!(
            KeyringError::from(method_err(SECRET_IS_LOCKED, "")),
            KeyringError::Locked
        );
        assert_eq!(
            KeyringError::from(method_err(DBUS_UNKNOWN_OBJECT, "")),
            KeyringError::NotFound
        );
        assert!(KeyringError::from(method_err(DBUS_SERVICE_UNKNOWN, "")).is_connection_lost());
        assert_eq!(
            KeyringError::from(method_err(
                "org.freedesktop.DBus.Error.Failed",
                "Cannot get label of a locked object `/x`."
            )),
            KeyringError::Locked
        );
        assert_eq!(
            KeyringError::from(method_err(
                "org.freedesktop.DBus.Error.Failed",
                "Prompt dismissed by user."
            )),
            KeyringError::Dismissed
        );
        assert!(matches!(
            KeyringError::from(method_err("org.example.Other", "boom")),
            KeyringError::Backend(_)
        ));
    }
}
