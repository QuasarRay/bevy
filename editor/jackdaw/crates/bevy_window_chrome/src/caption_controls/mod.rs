//! Minimize / maximize / close caption buttons for borderless windows.

mod caption;
mod caption_actions;

use bevy::prelude::*;

pub(crate) use caption::{load_caption_font, sync_caption_chrome};
pub use caption::{window_controls, CaptionButton, CaptionFont};

pub(crate) fn build(app: &mut App) {
    if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
        app.add_plugins(bevy::ui_widgets::ButtonPlugin);
    }
    caption_actions::register_pointer_handlers(app);
    app.add_systems(Last, sync_caption_chrome);
}
