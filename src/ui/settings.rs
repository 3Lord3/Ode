use std::rc::Rc;

use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::ui::AppState;

pub enum Msg {
    CacheCleared,
}

/// Build the preferences window. `on_msg` fires on events that affect the rest of the UI or the style
pub fn build(state: &Rc<AppState>, on_msg: Rc<dyn Fn(Msg)>) -> libadwaita::PreferencesDialog {
    let tr = state.tr.strings();

    let general = libadwaita::PreferencesGroup::builder()
        .title(tr.general_tab)
        .build();

    let clear_btn = libadwaita::ActionRow::builder()
        .title(tr.clear_cache)
        .subtitle(state.tr.strings().cache_desc)
        .build();
    let clear_btn_w = gtk4::Button::new();
    clear_btn_w.add_css_class("flat");
    clear_btn_w.set_icon_name("user-trash-symbolic");
    clear_btn.set_activatable_widget(Some(&clear_btn_w));
    clear_btn.add_suffix(&clear_btn_w);

    general.add(&clear_btn);

    let page = libadwaita::PreferencesPage::new();
    page.add(&general);

    let dlg = libadwaita::PreferencesDialog::builder()
        .content_width(480)
        .content_height(480)
        .build();
    dlg.add(&page);

    let state_c = state.clone();
    let on_msg_c = on_msg.clone();
    clear_btn.connect_activated(move |_| {
        let _ = state_c.cache.clear();
        on_msg_c(Msg::CacheCleared);
    });

    dlg
}
