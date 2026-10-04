use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use gtk4::glib;
use gtk4::prelude::*;

use crate::api::models::SearchResult;
use crate::ui::page::Page;
use crate::ui::AppState;

pub struct SearchPage {
    pub entry: gtk4::SearchEntry,
    pub content: gtk4::Widget,
    page: Rc<Page>,
    list: gtk4::ListBox,
    results: Rc<RefCell<Vec<SearchResult>>>,
    generation: Rc<AtomicU64>,
}

type PickFn = Rc<dyn Fn(u64)>;
type FocusFn = Rc<dyn Fn()>;

/// Build the search master panel. `on_pick` fires on result selection,
/// `on_search` when a search starts (switch to the results page)
pub fn build(state: &Rc<AppState>, on_pick: PickFn, on_search: FocusFn) -> SearchPage {
    let tr = &state.tr.strings();

    let entry = gtk4::SearchEntry::builder()
        .placeholder_text(tr.search_placeholder)
        .hexpand(true)
        .halign(gtk4::Align::Fill)
        .build();

    let page = Rc::new(Page::new());

    let list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .build();
    list.add_css_class("card");

    let scroll = gtk4::ScrolledWindow::builder()
        .child(&list)
        .hexpand(true)
        .vexpand(true)
        .build();
    let clamp = libadwaita::Clamp::builder()
        .maximum_size(840)
        .child(&scroll)
        .build();
    page.append(&clamp);
    page.show_status(tr.init_search, Some(tr.search_hint), "edit-find-symbolic");

    let page = SearchPage {
        entry,
        content: page.root.clone().upcast::<gtk4::Widget>(),
        page,
        list,
        results: Rc::new(RefCell::new(Vec::new())),
        generation: Rc::new(AtomicU64::new(0)),
    };

    // Shared search trigger: typing (debounced), Enter and clicks on the icon
    let gen = page.generation.clone();
    let list = page.list.clone();
    let page_state = page.page.clone();
    let results = page.results.clone();
    let state_b = state.clone();
    let trigger: Rc<dyn Fn(String, bool)> = {
        let gen = gen.clone();
        let list = list.clone();
        let page_state = page_state.clone();
        let results = results.clone();
        let state_b = state_b.clone();
        let on_search = on_search.clone();
        Rc::new(move |q: String, immediate: bool| {
            let q = q.trim().to_string();
            gen.fetch_add(1, Ordering::SeqCst);
            if q.is_empty() {
                page_state.show_status(
                    state_b.tr.strings().init_search,
                    Some(state_b.tr.strings().search_hint),
                    "edit-find-symbolic",
                );
                clear_rows(&list);
                return;
            }
            on_search();
            // Drop old results right away so they do not flash on a repeated search
            clear_rows(&list);
            page_state.show_loading();
            let state = state_b.clone();
            let list = list.clone();
            let results = results.clone();
            let ps = page_state.clone();
            let gen = gen.clone();
            let delay = if immediate {
                std::time::Duration::ZERO
            } else {
                std::time::Duration::from_millis(300)
            };
            glib::timeout_add_local_once(delay, move || {
                run(&state, q, gen, &list, &ps, &results);
            });
        })
    };

    let entry = page.entry.clone();
    let t = trigger.clone();
    entry.connect_search_changed(move |e| {
        t(e.text().to_string(), false);
    });
    let t = trigger.clone();
    let entry2 = entry.clone();
    entry.connect_activate(move |_| {
        t(entry2.text().to_string(), true);
    });
    // Typing and Enter are handled above, so only react to clicks on the icon itself,
    // not to any click on the text
    let t = trigger.clone();
    let entry3 = entry.clone();
    let click = gtk4::GestureClick::new();
    // Capture phase: run before GtkSearchEntry's own click handling
    gtk4::prelude::EventControllerExt::set_propagation_phase(
        &click,
        gtk4::PropagationPhase::Capture,
    );
    click.connect_pressed(move |_, _, x, _| {
        // The magnifier icon sits at the start of the entry (left in LTR), roughly the first 40px
        if (0.0..40.0).contains(&x) {
            let q = entry3.text().to_string();
            if !q.trim().is_empty() {
                t(q, true);
            }
        }
    });
    entry.add_controller(click);

    let results = page.results.clone();
    let list = page.list.clone();
    list.connect_row_activated(move |_, row| {
        let idx = row.index();
        if idx < 0 {
            return;
        }
        let mut store = results.borrow_mut();
        if let Some(SearchResult::Song(s)) = store.get_mut(idx as usize) {
            let id = s.id;
            on_pick(id);
        }
    });

    page
}

fn clear_rows(list: &gtk4::ListBox) {
    while let Some(row) = list.row_at_index(0) {
        list.remove(&row);
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    state: &Rc<AppState>,
    query: String,
    gen: Rc<AtomicU64>,
    list: &gtk4::ListBox,
    page: &Rc<Page>,
    results: &Rc<RefCell<Vec<SearchResult>>>,
) {
    let my_gen = gen.load(Ordering::SeqCst);
    let tr = state.tr.strings();
    let client = match state.client() {
        Some(c) => c,
        None => {
            page.show_status(
                tr.no_token_title,
                Some(tr.no_token_desc),
                "dialog-error-symbolic",
            );
            return;
        }
    };
    let list_owned = list.clone();
    let results_owned = results.clone();
    let state_rc = state.clone();
    let state_row = state_rc.clone();
    let page_c = page.clone();

    state_rc.async_fetch(
        move || crate::api::search::search(&client, &query).map_err(|e| e.to_string()),
        move |res| {
            if gen.load(Ordering::SeqCst) != my_gen {
                return; // stale request
            }
            clear_rows(&list_owned);
            match res {
                Ok(items) if items.is_empty() => {
                    page_c.show_status(tr.empty_title, Some(tr.empty_desc), "edit-find-symbolic");
                }
                Ok(items) => {
                    for r in &items {
                        list_owned.append(&row_for(&state_row, r));
                    }
                    page_c.show_body();
                    let mut store = results_owned.borrow_mut();
                    *store = items;
                }
                Err(e) => {
                    let msg = e.to_string();
                    page_c.show_status(tr.error_title, Some(&msg), "dialog-error-symbolic");
                }
            }
        },
    );
}

/// Splits a genius `full_title` into track title and artists on the last " by "
fn split_title_by(full: &str) -> (String, String) {
    match full.rfind(" by ") {
        Some(idx) => (
            full[..idx].trim().to_string(),
            full[idx + 4..].trim().to_string(),
        ),
        None => (full.to_string(), String::new()),
    }
}

fn row_for(state: &Rc<AppState>, result: &SearchResult) -> gtk4::ListBoxRow {
    let (title, subtitle) = match result {
        SearchResult::Song(s) => {
            // `title` is the clean name, unlike `full_title` which carries "by ..."
            let title = s.title.clone().unwrap_or_default();
            let full = s.full_title.clone().unwrap_or_default();
            let (_, artists) = split_title_by(&full);
            // Artists: exact API field > part of full_title after " by " > primary
            let subtitle = s
                .artist_names
                .clone()
                .filter(|v| !v.is_empty())
                .or_else(|| (!artists.is_empty()).then_some(artists))
                .unwrap_or_else(|| {
                    s.primary_artist
                        .as_ref()
                        .map(|a| a.name.clone())
                        .unwrap_or_default()
                });
            (title, subtitle)
        }
        SearchResult::Artist(a) => (a.name.clone(), String::new()),
        SearchResult::Album(a) => (a.full_title.clone().unwrap_or_default(), String::new()),
    };
    let cover_url = match result {
        SearchResult::Song(s) => s.song_art_image_url.clone(),
        SearchResult::Artist(a) => a.image_url.clone(),
        SearchResult::Album(a) => a.cover_art_url.clone(),
    };

    let cover = gtk4::Picture::builder()
        .width_request(64)
        .height_request(64)
        .content_fit(gtk4::ContentFit::Cover)
        .halign(gtk4::Align::Start)
        .valign(gtk4::Align::Start)
        .hexpand(false)
        .vexpand(false)
        .build();
    cover.add_css_class("icon-dropshadow");

    let title_l = gtk4::Label::builder()
        .label(&title)
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .wrap(true)
        .build();
    title_l.add_css_class("heading");
    let sub_l = gtk4::Label::builder()
        .label(&subtitle)
        .halign(gtk4::Align::Start)
        .wrap(true)
        .build();
    sub_l.add_css_class("dim-label");
    let text = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .build();
    text.append(&title_l);
    text.append(&sub_l);

    let h = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(14)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .build();
    h.append(&cover);
    h.append(&text);

    let row = gtk4::ListBoxRow::new();
    row.set_child(Some(&h));
    if matches!(result, SearchResult::Song(_)) {
        row.set_activatable(true);
        row.set_selectable(true);
        if let Some(url) = cover_url {
            load_cover(state, url, cover);
        }
    }
    row
}

fn load_cover(state: &Rc<AppState>, url: String, pic: gtk4::Picture) {
    const SIZE: i32 = 64;
    let st = state.clone();
    st.async_fetch(
        move || {
            use gtk4::gdk_pixbuf::prelude::PixbufLoaderExt;
            let bytes = reqwest::blocking::get(&url)
                .map_err(|e| e.to_string())?
                .bytes()
                .map_err(|e| e.to_string())?;
            let loader = gtk4::gdk_pixbuf::PixbufLoader::new();
            loader.write(&bytes).map_err(|e| e.to_string())?;
            loader.close().map_err(|e| e.to_string())?;
            let pb = loader
                .pixbuf()
                .ok_or_else(|| "no pixbuf".to_string())?
                .scale_simple(SIZE, SIZE, gtk4::gdk_pixbuf::InterpType::Bilinear)
                .ok_or_else(|| "scale failed".to_string())?;
            Ok(gtk4::gdk::Texture::for_pixbuf(&pb))
        },
        move |res| {
            if let Ok(tex) = res {
                pic.set_paintable(Some(&tex));
            }
        },
    );
}
