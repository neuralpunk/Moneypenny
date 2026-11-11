//! GNOME desktop integration: reads appearance settings from the XDG Settings
//! portal and pushes changes to the frontend so the UI follows the system the
//! way a libadwaita app does.

use ashpd::desktop::settings::{ColorScheme, Contrast, ReducedMotion, Settings};
use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Theme};

/// Event emitted to the frontend whenever a relevant setting changes.
pub const CHANGED_EVENT: &str = "desktop-settings-changed";

const APPEARANCE_NS: &str = "org.freedesktop.appearance";
const WM_NS: &str = "org.gnome.desktop.wm.preferences";
const BUTTON_LAYOUT_KEY: &str = "button-layout";
/// libadwaita's fallback when the setting can't be read.
const DEFAULT_BUTTON_LAYOUT: &str = "appmenu:close";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scheme {
    Default,
    PreferDark,
    PreferLight,
}

impl From<ColorScheme> for Scheme {
    fn from(value: ColorScheme) -> Self {
        match value {
            ColorScheme::PreferDark => Self::PreferDark,
            ColorScheme::PreferLight => Self::PreferLight,
            ColorScheme::NoPreference => Self::Default,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSettings {
    pub color_scheme: Scheme,
    /// sRGB components in 0.0..=1.0, or `None` when the desktop has no accent.
    pub accent_color: Option<[f64; 3]>,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    /// GNOME window-button layout, e.g. `"appmenu:minimize,maximize,close"`.
    pub button_layout: String,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            color_scheme: Scheme::Default,
            accent_color: None,
            high_contrast: false,
            reduced_motion: false,
            button_layout: DEFAULT_BUTTON_LAYOUT.to_owned(),
        }
    }
}

impl DesktopSettings {
    /// GTK has no "no preference" for windows; libadwaita treats it as light.
    pub fn theme(&self) -> Theme {
        match self.color_scheme {
            Scheme::PreferDark => Theme::Dark,
            Scheme::Default | Scheme::PreferLight => Theme::Light,
        }
    }
}

/// Reads all settings; any individual key that fails falls back to its default.
async fn read(settings: &Settings) -> DesktopSettings {
    let defaults = DesktopSettings::default();
    DesktopSettings {
        color_scheme: settings
            .color_scheme()
            .await
            .map(Scheme::from)
            .unwrap_or(defaults.color_scheme),
        accent_color: settings
            .accent_color()
            .await
            .ok()
            .map(|c| [c.red(), c.green(), c.blue()])
            .filter(|rgb| rgb.iter().all(|v| (0.0..=1.0).contains(v))),
        high_contrast: settings.contrast().await.is_ok_and(|c| c == Contrast::High),
        reduced_motion: settings
            .reduced_motion()
            .await
            .is_ok_and(|m| m == ReducedMotion::ReducedMotion),
        button_layout: settings
            .read::<String>(WM_NS, BUTTON_LAYOUT_KEY)
            .await
            .unwrap_or(defaults.button_layout),
    }
}

fn apply_theme(app: &AppHandle, settings: &DesktopSettings) {
    if let Some(window) = app.get_webview_window("main")
        && let Err(err) = window.set_theme(Some(settings.theme()))
    {
        log::warn!("could not set window theme: {err}");
    }
}

/// Returns the current settings, or defaults when the portal is unavailable,
/// so the UI can always render.
#[tauri::command]
pub async fn desktop_settings(app: AppHandle) -> DesktopSettings {
    let current = match Settings::new().await {
        Ok(settings) => read(&settings).await,
        Err(err) => {
            log::warn!("settings portal unavailable, using defaults: {err}");
            DesktopSettings::default()
        }
    };
    apply_theme(&app, &current);
    current
}

/// Watches the portal for changes and re-emits the full settings snapshot.
pub async fn watch(app: AppHandle) {
    let settings = match Settings::new().await {
        Ok(settings) => settings,
        Err(err) => {
            log::warn!("settings portal unavailable, not watching: {err}");
            return;
        }
    };
    let mut changes = match settings.receive_setting_changed().await {
        Ok(stream) => stream,
        Err(err) => {
            log::warn!("cannot subscribe to setting changes: {err}");
            return;
        }
    };
    while let Some(change) = changes.next().await {
        if !is_relevant(change.namespace(), change.key()) {
            continue;
        }
        let current = read(&settings).await;
        apply_theme(&app, &current);
        if let Err(err) = app.emit(CHANGED_EVENT, &current) {
            log::warn!("could not emit settings change: {err}");
        }
    }
}

fn is_relevant(namespace: &str, key: &str) -> bool {
    namespace == APPEARANCE_NS || (namespace == WM_NS && key == BUTTON_LAYOUT_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_libadwaita() {
        let d = DesktopSettings::default();
        assert_eq!(d.button_layout, "appmenu:close");
        assert_eq!(d.theme(), Theme::Light);
    }

    #[test]
    fn serializes_for_frontend() {
        let s = DesktopSettings {
            color_scheme: Scheme::PreferDark,
            accent_color: Some([0.1, 0.2, 0.3]),
            ..Default::default()
        };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["colorScheme"], "prefer-dark");
        assert_eq!(json["accentColor"][1], 0.2);
        assert_eq!(json["buttonLayout"], "appmenu:close");
        assert_eq!(json["highContrast"], false);
    }

    #[test]
    fn filters_relevant_settings() {
        assert!(is_relevant(APPEARANCE_NS, "color-scheme"));
        assert!(is_relevant(WM_NS, BUTTON_LAYOUT_KEY));
        assert!(!is_relevant(WM_NS, "theme"));
        assert!(!is_relevant("org.gnome.desktop.interface", "font-name"));
    }
}
