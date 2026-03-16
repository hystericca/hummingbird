use std::{fs::File, io::BufReader, path::PathBuf, sync::mpsc::channel, time::Duration};

use gpui::{App, AppContext, AsyncApp, EventEmitter, Global, Rgba, rgb, rgba};
use notify::{Event, RecursiveMode, Watcher};
use serde::Deserialize;
use tracing::{error, info, warn};

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Theme {
    pub background_primary: Rgba,
    pub background_secondary: Rgba,
    pub background_tertiary: Rgba,

    pub border_color: Rgba,

    pub album_art_background: Rgba,

    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_disabled: Rgba,
    pub text_link: Rgba,

    pub nav_button_hover: Rgba,
    pub nav_button_hover_border: Rgba,
    pub nav_button_active: Rgba,
    pub nav_button_active_border: Rgba,
    pub nav_button_pressed: Rgba,
    pub nav_button_pressed_border: Rgba,

    pub playback_button: Rgba,
    pub playback_button_hover: Rgba,
    pub playback_button_active: Rgba,
    pub playback_button_border: Rgba,
    pub playback_button_toggled: Rgba,

    pub window_button: Rgba,
    pub window_button_hover: Rgba,
    pub window_button_active: Rgba,

    pub close_button: Rgba,
    pub close_button_hover: Rgba,
    pub close_button_active: Rgba,

    pub queue_item: Rgba,
    pub queue_item_hover: Rgba,
    pub queue_item_active: Rgba,
    pub queue_item_current: Rgba,

    pub button_primary: Rgba,
    pub button_primary_border: Rgba,
    pub button_primary_hover: Rgba,
    pub button_primary_border_hover: Rgba,
    pub button_primary_active: Rgba,
    pub button_primary_border_active: Rgba,
    pub button_primary_text: Rgba,

    pub button_secondary: Rgba,
    pub button_secondary_border: Rgba,
    pub button_secondary_hover: Rgba,
    pub button_secondary_border_hover: Rgba,
    pub button_secondary_active: Rgba,
    pub button_secondary_border_active: Rgba,
    pub button_secondary_text: Rgba,

    pub button_warning: Rgba,
    pub button_warning_border: Rgba,
    pub button_warning_hover: Rgba,
    pub button_warning_border_hover: Rgba,
    pub button_warning_active: Rgba,
    pub button_warning_border_active: Rgba,
    pub button_warning_text: Rgba,

    pub button_danger: Rgba,
    pub button_danger_border: Rgba,
    pub button_danger_hover: Rgba,
    pub button_danger_border_hover: Rgba,
    pub button_danger_active: Rgba,
    pub button_danger_border_active: Rgba,
    pub button_danger_text: Rgba,

    pub slider_foreground: Rgba,
    pub slider_background: Rgba,

    pub elevated_background: Rgba,
    pub elevated_border_color: Rgba,

    pub menu_item: Rgba,
    pub menu_item_hover: Rgba,
    pub menu_item_border_hover: Rgba,
    pub menu_item_active: Rgba,
    pub menu_item_border_active: Rgba,

    pub modal_overlay_bg: Rgba,

    pub text_input_selection: Rgba,
    pub caret_color: Rgba,

    pub palette_item_hover: Rgba,
    pub palette_item_border_hover: Rgba,
    pub palette_item_active: Rgba,
    pub palette_item_border_active: Rgba,

    pub scrollbar_background: Rgba,
    pub scrollbar_foreground: Rgba,

    pub textbox_background: Rgba,
    pub textbox_border: Rgba,

    pub checkbox_background: Rgba,
    pub checkbox_background_hover: Rgba,
    pub checkbox_background_active: Rgba,
    pub checkbox_border: Rgba,
    pub checkbox_border_hover: Rgba,
    pub checkbox_border_active: Rgba,
    pub checkbox_checked: Rgba,
    pub checkbox_checked_bg: Rgba,
    pub checkbox_checked_bg_hover: Rgba,
    pub checkbox_checked_bg_active: Rgba,
    pub checkbox_checked_border: Rgba,
    pub checkbox_checked_border_hover: Rgba,
    pub checkbox_checked_border_active: Rgba,

    pub callout_background: Rgba,
    pub callout_border: Rgba,
    pub callout_text: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background_primary: rgb(0x0D0E12),
            background_secondary: rgb(0x161720),
            background_tertiary: rgb(0x1A1D26),

            border_color: rgb(0x202233),

            album_art_background: rgb(0x303246),

            text: rgb(0xE8E9F2),
            text_secondary: rgb(0xA0A1AD),
            text_disabled: rgb(0x5F5F71),
            text_link: rgb(0x5279D4),

            nav_button_hover: rgb(0x1A1C28),
            nav_button_hover_border: rgb(0x212431),
            nav_button_active: rgb(0x151620),
            nav_button_active_border: rgb(0x191B27),
            nav_button_pressed: rgb(0x1F212D),
            nav_button_pressed_border: rgb(0x292D3F),

            playback_button: rgba(0x00000000),
            playback_button_hover: rgb(0x272B41),
            playback_button_active: rgb(0x08080B),
            playback_button_border: rgba(0x00000000),
            playback_button_toggled: rgb(0x688CF0),

            window_button: rgba(0x00000000),
            window_button_hover: rgb(0x262D42),
            window_button_active: rgb(0x0D0F14),

            queue_item: rgba(0x00000000),
            queue_item_hover: rgb(0x151621),
            queue_item_active: rgb(0x101118),
            queue_item_current: rgb(0x1B1C28),

            close_button: rgba(0x00000000),
            close_button_hover: rgb(0x7E2C2C),
            close_button_active: rgb(0x5B1D1D),

            button_primary: rgb(0x5774E7),
            button_primary_border: rgb(0x6D85E4),
            button_primary_hover: rgb(0x6D92FF),
            button_primary_border_hover: rgb(0x5488FF),
            button_primary_active: rgb(0x495F9F),
            button_primary_border_active: rgb(0x515C8F),
            button_primary_text: rgb(0xE0E7F7),

            button_secondary: rgb(0x373B4E),
            button_secondary_border: rgb(0x4F5267),
            button_secondary_hover: rgb(0x494E67),
            button_secondary_border_hover: rgb(0x565A77),
            button_secondary_active: rgb(0x262636),
            button_secondary_border_active: rgb(0x2F3244),
            button_secondary_text: rgb(0xDDDEEC),

            button_warning: rgb(0x97792C),
            button_warning_border: rgb(0xC59E4F),
            button_warning_hover: rgb(0xA98B4A),
            button_warning_border_hover: rgb(0xC9A558),
            button_warning_active: rgb(0x5D4B2E),
            button_warning_border_active: rgb(0x80683F),
            button_warning_text: rgb(0xF0EBDE),

            button_danger: rgb(0x650B0B),
            button_danger_border: rgb(0x860808),
            button_danger_hover: rgb(0x750C0C),
            button_danger_border_hover: rgb(0x8F0B0B),
            button_danger_active: rgb(0x440A0A),
            button_danger_border_active: rgb(0x650707),
            button_danger_text: rgb(0xE9D4D4),

            slider_foreground: rgb(0x688CF0),
            slider_background: rgb(0x38374E),

            elevated_background: rgb(0x161820),
            elevated_border_color: rgb(0x23253B),

            menu_item: rgba(0x00000000),
            menu_item_hover: rgb(0x1F2334),
            menu_item_border_hover: rgb(0x2B2F44),
            menu_item_active: rgb(0x0E0F15),
            menu_item_border_active: rgb(0x1F212E),

            modal_overlay_bg: rgba(0x00000055),

            text_input_selection: rgba(0x01020388),
            caret_color: rgb(0xE8E8F2),

            palette_item_hover: rgb(0x1F2334),
            palette_item_border_hover: rgb(0x2B2F44),
            palette_item_active: rgb(0x0E0F15),
            palette_item_border_active: rgb(0x1F212E),

            scrollbar_background: rgb(0x252839),
            scrollbar_foreground: rgb(0x616794),

            textbox_background: rgb(0x373B4E),
            textbox_border: rgb(0x4F5267),

            checkbox_background: rgb(0x373B4E),
            checkbox_background_hover: rgb(0x494E67),
            checkbox_background_active: rgb(0x262636),
            checkbox_border: rgb(0x4F5267),
            checkbox_border_hover: rgb(0x565A77),
            checkbox_border_active: rgb(0x2F3244),
            checkbox_checked: rgb(0xC7C7D8),
            checkbox_checked_bg: rgb(0x618EE6),
            checkbox_checked_bg_hover: rgb(0x6080F9),
            checkbox_checked_bg_active: rgb(0x495D9F),
            checkbox_checked_border: rgb(0x7592E7),
            checkbox_checked_border_hover: rgb(0x657DFF),
            checkbox_checked_border_active: rgb(0x515D8F),

            callout_background: rgba(0x2E280053),
            callout_border: rgba(0x5B45008E),
            callout_text: rgb(0xF0EBDE),
        }
    }
}

impl Global for Theme {}

pub fn create_theme(path: &PathBuf) -> Theme {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);

        if let Ok(theme) = serde_json::from_reader(reader) {
            theme
        } else {
            warn!("Theme file exists but it could not be loaded, using default");
            Theme::default()
        }
    } else {
        Theme::default()
    }
}

#[derive(PartialEq, Clone)]
pub struct ThemeEvTransmitter;

impl EventEmitter<Theme> for ThemeEvTransmitter {}

#[allow(dead_code)]
pub struct ThemeWatcher(pub Box<dyn Watcher>);

impl Global for ThemeWatcher {}

pub fn setup_theme(cx: &mut App, path: PathBuf) {
    cx.set_global(create_theme(&path));
    let theme_transmitter = cx.new(|_| ThemeEvTransmitter);

    cx.subscribe(&theme_transmitter, |_, theme, cx| {
        cx.set_global(theme.clone());
        cx.refresh_windows();
    })
    .detach();

    let (tx, rx) = channel::<notify::Result<Event>>();

    let watcher = notify::recommended_watcher(tx);

    if let Ok(mut watcher) = watcher {
        if let Err(e) = watcher.watch(path.parent().unwrap(), RecursiveMode::NonRecursive) {
            warn!("failed to watch settings directory: {:?}", e);
        }

        cx.spawn(async move |cx: &mut AsyncApp| {
            loop {
                while let Ok(event) = rx.try_recv() {
                    match event {
                        Ok(v) => {
                            if v.paths.iter().any(|t| t.ends_with("theme.json")) {
                                match v.kind {
                                    notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                                        info!("Theme changed, updating...");
                                        let theme = create_theme(&path);
                                        theme_transmitter.update(cx, move |_, m| {
                                            m.emit(theme);
                                        });
                                    }
                                    notify::EventKind::Remove(_) => {
                                        info!("Theme file removed, resetting to default...");
                                        theme_transmitter.update(cx, |_, m| {
                                            m.emit(Theme::default());
                                        });
                                    }
                                    _ => (),
                                }
                            }
                        }
                        Err(e) => error!("error occured while watching theme.json: {:?}", e),
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
            }
        })
        .detach();

        // store the watcher in a global so it doesn't go out of scope
        let tw = ThemeWatcher(Box::new(watcher));
        cx.set_global(tw);
    } else if let Err(e) = watcher {
        warn!("failed to watch settings directory: {:?}", e);
    }
}
