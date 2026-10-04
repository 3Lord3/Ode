use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use webkit6::prelude::*;

use crate::api::song::LyricAnnotation;
use crate::i18n::Strings;
use crate::ui::page::Page;
use crate::ui::AppState;
use libadwaita::prelude::*;

/// Expression that extracts lyric text from genius.com containers
const LYRICS_JS: &str = r#"(() => {
    const els = document.querySelectorAll(
        '[data-lyrics-container], .lyrics, div[class*="Lyrics__Container"]'
    );
    return Array.from(els).map(e => e.innerText).join("\n\n");
})()"#;

/// Metadata for the info dialog; empty fields are skipped when rendering
#[derive(Clone, Default)]
struct SongInfo {
    title: String,
    artists: String,
    release: String,
    producers: String,
    album: String,
    about: String,
}

pub struct SongPage {
    pub content: gtk4::Widget,
    page: Rc<Page>,
    fact_cover: gtk4::Picture,
    fact_title: gtk4::Label,
    fact_sub: gtk4::Label,
    lyrics_view: gtk4::TextView,
    lyrics_spin: gtk4::Spinner,
    annotations: Rc<RefCell<Vec<LyricAnnotation>>>,
    annot_lines: Rc<RefCell<HashSet<i32>>>,
    hover_line: Rc<RefCell<Option<i32>>>,
    open_btn: gtk4::Button,
    copy_btn: gtk4::Button,
    info_btn: gtk4::Button,
    info: Rc<RefCell<Option<SongInfo>>>,
    window: gtk4::Window,
    webview: webkit6::WebView,
    handler: Rc<RefCell<Option<glib::SignalHandlerId>>>,
    cur_lyrics: Rc<RefCell<String>>,
    cur_url: Rc<RefCell<String>>,
    cur_id: Rc<Cell<u64>>,
    back_btn: gtk4::Button,
}

pub fn build(state: &Rc<AppState>, window: &gtk4::Window) -> SongPage {
    let tr = state.tr.strings();
    let page = Rc::new(Page::new());

    let cover = gtk4::Picture::builder()
        .width_request(96)
        .height_request(96)
        .content_fit(gtk4::ContentFit::Cover)
        .css_classes(["icon-dropshadow"])
        .build();
    let title = gtk4::Label::builder().halign(gtk4::Align::Start).build();
    title.add_css_class("title-1");
    // Wrap long titles and artist lists instead of eliding them
    title.set_wrap(true);
    let sub = gtk4::Label::builder().halign(gtk4::Align::Start).build();
    sub.add_css_class("dim-label");
    sub.set_wrap(true);

    let head = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(16)
        .build();
    head.append(&cover);
    let headtext = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .valign(gtk4::Align::Center)
        .halign(gtk4::Align::Start)
        .hexpand(true)
        .build();
    headtext.append(&title);
    headtext.append(&sub);
    head.append(&headtext);

    let lyrics_view = gtk4::TextView::new();
    lyrics_view.set_editable(false);
    lyrics_view.set_cursor_visible(true);
    lyrics_view.set_wrap_mode(gtk4::WrapMode::WordChar);
    lyrics_view.set_monospace(false);
    lyrics_view.set_pixels_above_lines(4);
    lyrics_view.set_left_margin(2);
    lyrics_view.set_right_margin(2);
    lyrics_view.set_hexpand(true);
    lyrics_view.set_vexpand(true);
    lyrics_view.add_css_class("song-lyrics");
    // Spinner over the lyrics text only (not the whole page), so the song header and
    // action buttons stay visible while the text loads
    let lyrics_spin = gtk4::Spinner::builder().spinning(true).build();
    lyrics_spin.set_halign(gtk4::Align::Center);
    lyrics_spin.set_valign(gtk4::Align::Center);
    lyrics_spin.set_visible(false);
    let lyrics_area = gtk4::Overlay::new();
    lyrics_area.set_hexpand(true);
    lyrics_area.set_vexpand(true);
    lyrics_area.set_child(Some(&lyrics_view));
    lyrics_area.add_overlay(&lyrics_spin);
    // Large, regular (not italic, not monospace) lyrics text
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(
        // No hard pt size: the size is relative to the system text setting
        ".song-lyrics { font-size: 1.15em; font-style: normal; }",
    );
    // Display-level CSS provider, so the style is scoped to the .song-lyrics class
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let attribution = gtk4::Label::builder()
        .label(tr.lyrics_attribution)
        .halign(gtk4::Align::End)
        .build();
    attribution.add_css_class("dim-label");

    // Action buttons: back, copy, open on genius (text lives in the tooltip)
    // The back button is explicit: the automatic AdwNavigationView button shows up
    // in the window header, not in the page body
    let back_btn = gtk4::Button::builder()
        .icon_name("go-previous-symbolic")
        .tooltip_text(tr.back)
        .build();
    back_btn.add_css_class("flat");

    let copy_btn = gtk4::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text(tr.copy_lyrics)
        .build();
    copy_btn.add_css_class("flat");

    let open_btn = gtk4::Button::builder()
        .icon_name("adw-external-link-symbolic")
        .tooltip_text(tr.open_genius)
        .build();
    open_btn.add_css_class("flat");

    let info_btn = gtk4::Button::builder()
        .icon_name("help-about-symbolic")
        .tooltip_text(tr.song_info)
        .build();
    info_btn.add_css_class("flat");

    let top = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    let spacer = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .hexpand(true)
        .build();
    top.append(&back_btn);
    top.append(&spacer);
    top.append(&info_btn);
    top.append(&copy_btn);
    top.append(&open_btn);

    let actions = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Start)
        .build();
    actions.append(&attribution);

    let body = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .hexpand(true)
        .vexpand(true)
        .build();
    body.append(&top);
    body.append(&head);
    body.append(&lyrics_area);
    body.append(&actions);

    // The scroll wraps the whole song page (header, lyrics, buttons). The scrollbar
    // has to sit at the window edge rather than at the text column edge, so the
    // ScrolledWindow wraps the Clamp, not the other way around
    let clamp = libadwaita::Clamp::builder()
        .maximum_size(860)
        .child(&body)
        .build();
    let scroll = gtk4::ScrolledWindow::builder()
        .child(&clamp)
        .vexpand(true)
        .hexpand(true)
        .build();
    page.append(&scroll);

    // Hidden but realized WebKitWebView, the background source of lyrics
    let webview = webkit6::WebView::new();
    // Deny every page permission request (camera, mic, geolocation, notifications),
    // the app only needs the lyrics text
    webview.connect_permission_request(|_, req| {
        req.deny();
        true
    });
    page.attach_hidden(&webview);

    // Nothing to show yet
    page.show_status(
        tr.select_title,
        Some(tr.select_desc),
        "audio-x-generic-symbolic",
    );

    let mut page_obj = SongPage {
        content: page.root.clone().upcast::<gtk4::Widget>(),
        page,
        fact_cover: cover,
        fact_title: title,
        fact_sub: sub,
        lyrics_view,
        lyrics_spin,
        annotations: Rc::new(RefCell::new(Vec::new())),
        annot_lines: Rc::new(RefCell::new(HashSet::new())),
        hover_line: Rc::new(RefCell::new(None)),
        open_btn,
        copy_btn,
        info_btn,
        info: Rc::new(RefCell::new(None)),
        window: window.clone(),
        webview,
        handler: Rc::new(RefCell::new(None)),
        cur_lyrics: Rc::new(RefCell::new(String::new())),
        cur_url: Rc::new(RefCell::new(String::new())),
        cur_id: Rc::new(Cell::new(0)),
        back_btn,
    };
    page_obj.wire(state);
    page_obj
}

fn split_ft_by(full: &str) -> (String, String) {
    match full.rfind(" by ") {
        Some(idx) => (
            full[..idx].trim().to_string(),
            full[idx + 4..].trim().to_string(),
        ),
        None => (full.to_string(), String::new()),
    }
}

/// Looks the tag up in the buffer table, creating and registering it on a miss
fn tag_for(
    table: &gtk4::TextTagTable,
    name: &str,
    build: impl FnOnce() -> gtk4::TextTag,
) -> gtk4::TextTag {
    if let Some(t) = table.lookup(name) {
        return t;
    }
    let tag = build();
    table.add(&tag);
    tag
}

fn line_iters(buf: &gtk4::TextBuffer, line: i32) -> Option<(gtk4::TextIter, gtk4::TextIter)> {
    let start = buf.iter_at_line(line)?;
    let mut end = start;
    if !end.ends_line() {
        end.forward_to_line_end();
    } else {
        end.forward_char();
    }
    Some((start, end))
}

/// Moves the background tag to `line` and clears it from the previous one
fn apply_hover(
    view: &gtk4::TextView,
    line: i32,
    annot_lines: &Rc<RefCell<HashSet<i32>>>,
    hover_line: &Rc<RefCell<Option<i32>>>,
) {
    let buf = view.buffer();
    let table = buf.tag_table();
    let hover = table.lookup("annot_hover");
    let prev = *hover_line.borrow();
    if prev == Some(line) {
        return;
    }
    // Clear hover from the previous line
    if let Some(p) = prev {
        if let (Some((s, e)), Some(h)) = (line_iters(&buf, p), hover.as_ref()) {
            buf.remove_tag(h, &s, &e);
        }
    }
    if annot_lines.borrow().contains(&line) {
        *hover_line.borrow_mut() = Some(line);
        if let (Some((s, e)), Some(h)) = (line_iters(&buf, line), hover) {
            buf.apply_tag(&h, &s, &e);
        }
    } else {
        *hover_line.borrow_mut() = None;
    }
}

/// Annotation color from the current theme: a translucent layer over
/// accent_bg_color, so it reads on light and dark backgrounds and follows the
/// system theme variants (accent, success, etc.)
#[allow(deprecated)] // style_context/lookup_color is the only way to read a named theme color
fn themed_alpha_color(widget: &impl IsA<gtk4::Widget>, name: &str, alpha: f32) -> gtk4::gdk::RGBA {
    let fallback = gtk4::gdk::RGBA::parse(if alpha > 0.3 {
        "rgba(53, 132, 228, 0.45)"
    } else {
        "rgba(53, 132, 228, 0.22)"
    })
    .unwrap();
    let mut rgba = widget
        .as_ref()
        .style_context()
        .lookup_color(name)
        .unwrap_or(fallback);
    rgba.set_alpha(alpha);
    rgba
}

/// Trims the junk Genius puts before the lyrics: the "About" block, the
/// "Read More" link, etc. Cut everything before the first section line like
/// "[Verse 1]" / "[Chorus]" / "[Intro]", or before "Read More" / "About" if
/// there is none
fn clean_lyrics(text: &str) -> String {
    let section = text.lines().position(|l| l.trim_start().starts_with('['));
    let about = text
        .lines()
        .position(|l| matches!(l.trim().to_lowercase().as_str(), "read more" | "about"));
    let start = match (section, about) {
        (Some(a), Some(b)) => a.min(b),
        (a, b) => a.or(b).unwrap_or(0),
    };
    text.lines()
        .skip(start)
        .take_while(|l| !matches!(l.trim().to_lowercase().as_str(), "read more" | "about"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn show_lyrics(
    text: String,
    view: &gtk4::TextView,
    cur: &Rc<RefCell<String>>,
    annots: &[LyricAnnotation],
    annot_lines: &Rc<RefCell<HashSet<i32>>>,
    hover_line: &Rc<RefCell<Option<i32>>>,
) {
    // Strip the Genius junk (About / Read More) before writing to the buffer and to
    // `cur`, so the copy button yields the actual song text
    let text = clean_lyrics(&text);
    // Hide the cursor while refilling: GTK would otherwise autoscroll the TextView to
    // the moving insert mark, sending the already visible text (e.g. on redraw
    // after annotations arrive) to the very end
    view.set_cursor_visible(false);
    let buffer = view.buffer();
    buffer.set_text("");
    let table = buffer.tag_table();
    // Translucent background: readable on light and dark themes without covering the
    // text color
    let annot = tag_for(&table, "annot", || {
        gtk4::TextTag::builder()
            .name("annot")
            .background_rgba(&themed_alpha_color(view, "accent_bg_color", 0.22))
            .build()
    });
    let _hover = tag_for(&table, "annot_hover", || {
        gtk4::TextTag::builder()
            .name("annot_hover")
            .background_rgba(&themed_alpha_color(view, "accent_bg_color", 0.45))
            .build()
    });
    annot_lines.borrow_mut().clear();
    *hover_line.borrow_mut() = None;
    let mut caret = buffer.end_iter();
    let mut line_no = 0i32;
    let mut prev_empty = false;
    for line in text.split('\n') {
        // Skip Genius utility headings ("N Contributors", "Translations", "X Lyrics")
        let trimmed = line.trim();
        let low = trimmed.to_lowercase();
        let stray = low.ends_with("lyrics")
            || low.ends_with("contributors")
            || low == "translations"
            || low.ends_with(" translations")
            || low.ends_with(" contributors");
        if stray {
            continue;
        }
        // Collapse runs of blank lines to one, keeping [Verse] / [Chorus] blocks
        // separated by exactly one empty line as in the source
        if trimmed.is_empty() {
            if prev_empty {
                continue;
            }
            prev_empty = true;
            buffer.insert(&mut caret, "\n");
            line_no += 1;
            continue;
        }
        prev_empty = false;
        let line_low = line.to_lowercase();
        let has = annots.iter().any(|a| {
            let f = a.fragment.trim();
            !f.is_empty() && line_low.contains(&f.to_lowercase())
        });
        let start_off = caret.offset();
        buffer.insert(&mut caret, line);
        if has {
            let seg_start = buffer.iter_at_offset(start_off);
            let seg_end = buffer.iter_at_offset(caret.offset());
            buffer.apply_tag(&annot, &seg_start, &seg_end);
            annot_lines.borrow_mut().insert(line_no);
        }
        buffer.insert(&mut caret, "\n");
        line_no += 1;
    }
    // Start at the top so the first click does not jump to the end
    buffer.place_cursor(&buffer.start_iter());
    *cur.borrow_mut() = text;
    view.set_cursor_visible(true);
}

/// Metadata list plus the description, as two pages of one stack
fn show_song_info(parent: &gtk4::Window, tr: &'static Strings, s: &SongInfo) {
    let dlg = libadwaita::Dialog::builder().content_width(400).build();
    let header = libadwaita::HeaderBar::new();
    let title = libadwaita::WindowTitle::new(tr.song_info, "");
    header.set_title_widget(Some(&title));
    let back = gtk4::Button::from_icon_name("go-previous-symbolic");
    back.add_css_class("flat");
    back.set_visible(false);
    header.pack_start(&back);

    let stack = gtk4::Stack::new();

    let page = libadwaita::PreferencesPage::new();
    let group = libadwaita::PreferencesGroup::new();
    for (icon, label, value) in [
        ("audio-x-generic-symbolic", tr.info_title, &s.title),
        ("avatar-default-symbolic", tr.info_artists, &s.artists),
        ("user-info-symbolic", tr.info_producers, &s.producers),
        ("x-office-calendar-symbolic", tr.info_release, &s.release),
        ("folder-music-symbolic", tr.info_album, &s.album),
    ] {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let row = libadwaita::ActionRow::builder()
            .title(label)
            .subtitle(value)
            .title_lines(0)
            .subtitle_lines(0)
            .subtitle_selectable(true)
            .build();
        row.add_prefix(&gtk4::Image::from_icon_name(icon));
        group.add(&row);
    }

    let about = s.about.trim();
    if !about.is_empty() {
        let row = libadwaita::ActionRow::builder()
            .title(tr.info_about)
            .activatable(true)
            .build();
        row.add_prefix(&gtk4::Image::from_icon_name("help-about-symbolic"));
        row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));
        let st_open = stack.clone();
        let back_open = back.clone();
        let title_open = title.clone();
        let title_about = tr.info_about.to_string();
        row.connect_activated(move |_| {
            st_open.set_visible_child_name("about");
            title_open.set_title(&title_about);
            back_open.set_visible(true);
        });
        group.add(&row);

        let label = gtk4::Label::builder()
            .label(about)
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::Start)
            .hexpand(true)
            .margin_top(18)
            .margin_bottom(18)
            .margin_start(18)
            .margin_end(18)
            .build();
        let scroll = gtk4::ScrolledWindow::builder()
            .child(&label)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .max_content_height(420)
            .propagate_natural_height(true)
            .hexpand(true)
            .build();
        stack.add_named(&scroll, Some("about"));

        let st_back = stack.clone();
        let title_back = title.clone();
        let title_list = tr.song_info.to_string();
        let back_self = back.clone();
        back.connect_clicked(move |_| {
            st_back.set_visible_child_name("list");
            title_back.set_title(&title_list);
            back_self.set_visible(false);
        });
    }
    page.add(&group);
    stack.add_named(&page, Some("list"));
    stack.set_visible_child_name("list");

    let toolbar = libadwaita::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&stack));
    dlg.set_child(Some(&toolbar));
    dlg.present(Some(parent));
}

fn open_url(url: &str) {
    let _ = gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>);
}

fn load_cover(state: &Rc<AppState>, url: String, pic: gtk4::Picture) {
    let st = state.clone();
    st.async_fetch(
        move || {
            let bytes = reqwest::blocking::get(&url)
                .map_err(|e| e.to_string())?
                .bytes()
                .map_err(|e| e.to_string())?;
            gtk4::gdk::Texture::from_bytes(&glib::Bytes::from(&bytes)).map_err(|e| e.to_string())
        },
        move |res| {
            if let Ok(tex) = res {
                pic.set_paintable(Some(&tex));
            }
        },
    );
}

impl SongPage {
    fn wire(&mut self, state: &Rc<AppState>) {
        let url = self.cur_url.clone();
        self.open_btn.connect_clicked(move |_| {
            let u = url.borrow().clone();
            if !u.is_empty() {
                open_url(&u);
            }
        });

        // Copy exactly what is shown: take the text from the TextView buffer, which
        // already excludes utility headings and extra blank lines
        let info = self.info.clone();
        let st_info = state.clone();
        let parent_info = self.window.clone();

        self.info_btn.connect_clicked(move |_| {
            let tr = st_info.tr.strings();
            if let Some(s) = info.borrow().as_ref() {
                show_song_info(&parent_info, tr, s);
            }
        });

        let copy_view = self.lyrics_view.clone();
        let st_copy = state.clone();
        self.copy_btn.connect_clicked(move |_| {
            let buf = copy_view.buffer();
            let start = buf.start_iter();
            let end = buf.end_iter();
            let txt = buf.text(&start, &end, false).trim().to_string();
            if !txt.is_empty() {
                if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_text(&txt);
                }
                st_copy.show_toast(st_copy.tr.strings().copied);
            }
        });

        // Clicking an annotated lyric line opens its decoding
        let view = self.lyrics_view.clone();
        let view_click = view.clone();
        let annots = self.annotations.clone();
        let st_click = state.clone();
        let parent = self.window.clone();
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(1);
        gesture.connect_pressed(move |_, _, x, y| {
            let Some(iter) = view_click.iter_at_location(x as i32, y as i32) else {
                return;
            };
            let buf = view_click.buffer();
            let line = iter.line();
            let start = buf.iter_at_line(line).unwrap_or(iter);
            let mut end = start;
            if !end.ends_line() {
                end.forward_to_line_end();
            } else {
                end.forward_char();
            }
            let line_txt = buf.text(&start, &end, false).to_string();
            let line_low = line_txt.to_lowercase();
            let list = annots.borrow();
            let matched: Vec<&LyricAnnotation> = list
                .iter()
                .filter(|a| {
                    let f = a.fragment.trim();
                    !f.is_empty() && line_low.contains(&f.to_lowercase())
                })
                .collect();
            if !matched.is_empty() {
                let body = matched
                    .iter()
                    .map(|a| a.text.clone())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let tr = st_click.tr.strings();
                let dlg = libadwaita::AlertDialog::builder()
                    .heading(tr.annot_title)
                    .body(&body)
                    .build();
                dlg.add_response("close", tr.close);
                dlg.choose(Some(&parent), None::<&gio::Cancellable>, |_| {});
            }
        });
        view.add_controller(gesture);

        // Hover: strengthens the background of the annotated line under the cursor
        let view_move = view.clone();
        let anl_move = self.annot_lines.clone();
        let hv = self.hover_line.clone();
        let motion = gtk4::EventControllerMotion::new();
        let mv = view_move.clone();
        motion.connect_motion(move |_, x, y| {
            let Some(iter) = mv.iter_at_location(x as i32, y as i32) else {
                return;
            };
            apply_hover(&mv, iter.line(), &anl_move, &hv);
        });
        let view_leave = view.clone();
        let hv_leave = self.hover_line.clone();
        motion.connect_leave(move |_| {
            if let Some(p) = *hv_leave.borrow() {
                let buf = view_leave.buffer();
                if let (Some((s, e)), Some(t)) =
                    (line_iters(&buf, p), buf.tag_table().lookup("annot_hover"))
                {
                    buf.remove_tag(&t, &s, &e);
                }
            }
            *hv_leave.borrow_mut() = None;
        });
        view.add_controller(motion);
    }

    pub fn set_back<F: Fn() + 'static>(&self, f: F) {
        let btn = self.back_btn.clone();
        btn.connect_clicked(move |_| f());
    }

    pub fn open(&mut self, state: &Rc<AppState>, id: u64) {
        self.open_song(state, id);
    }

    fn open_song(&mut self, state: &Rc<AppState>, id: u64) {
        let tr = state.tr.strings();
        self.page.show_loading();
        self.cur_id.set(id);
        *self.info.borrow_mut() = None;
        // Clear the header right away so the previous track's cover and title do not flash
        self.fact_cover.set_paintable(None::<&gtk4::gdk::Texture>);
        self.fact_title.set_label("");
        self.fact_sub.set_label("");
        let client = match state.client() {
            Some(c) => c.clone(),
            None => return,
        };
        let client_lyr = client.clone();
        let page = self.page.clone();
        let title = self.fact_title.clone();
        let sub = self.fact_sub.clone();
        let cover = self.fact_cover.clone();
        let cur_url = self.cur_url.clone();
        let webview = self.webview.clone();
        let handler = self.handler.clone();
        let view = self.lyrics_view.clone();
        let view_ann = view.clone();
        let spin = self.lyrics_spin.clone();
        let cur = self.cur_lyrics.clone();
        let annots = self.annotations.clone();
        let annot_lines = self.annot_lines.clone();
        let hover_line = self.hover_line.clone();
        let st = state.clone();
        let st2 = st.clone();
        let rc_info = self.info.clone();
        let req_id = self.cur_id.clone();

        st.async_fetch(
            move || crate::api::song::song(&client_lyr, id).map_err(|e| e.to_string()),
            move |res| match res {
                Ok(song) => {
                    // The user may have switched songs meanwhile
                    if req_id.get() != id {
                        return;
                    }
                    // Title and cover appear only together with the lyrics, so the header layout
                    // does not break before the text arrives
                    let title_txt = song.title.clone().unwrap_or_default();
                    // Artists: artist_names > (full_title after " by ") > primary
                    let full = song.full_title.clone().unwrap_or_default();
                    let (_, ft_artists) = split_ft_by(&full);
                    let artist = song
                        .artist_names
                        .clone()
                        .filter(|v| !v.is_empty())
                        .or_else(|| (!ft_artists.is_empty()).then_some(ft_artists))
                        .unwrap_or_else(|| {
                            song.primary_artist
                                .as_ref()
                                .map(|a| a.name.clone())
                                .unwrap_or_default()
                        });
                    let sub_txt = [
                        artist.clone(),
                        tr.localize_date(
                            song.release_date_for_display.as_deref().unwrap_or_default(),
                        ),
                    ]
                    .join(" · ");
                    let art_url = song.song_art_image_url.clone().unwrap_or_default();
                    *cur_url.borrow_mut() = song.url.clone();
                    *rc_info.borrow_mut() = Some(SongInfo {
                        // `title` is the clean name; `full_title` carries "by ..."
                        title: if title_txt.trim().is_empty() {
                            split_ft_by(&full).0
                        } else {
                            title_txt.clone()
                        },
                        artists: artist.clone(),
                        release: tr.localize_date(
                            song.release_date_for_display.as_deref().unwrap_or_default(),
                        ),
                        producers: song
                            .producer_artists
                            .iter()
                            .map(|p| p.name.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                        album: song
                            .album
                            .as_ref()
                            .map(|a| {
                                let name = a
                                    .name
                                    .clone()
                                    .or_else(|| {
                                        a.full_title.as_deref().map(split_ft_by).map(|(t, _)| t)
                                    })
                                    .unwrap_or_default();
                                match song.track_number {
                                    Some(n) if n > 0 => format!("{n}. {name}"),
                                    _ => name,
                                }
                            })
                            .unwrap_or_default(),
                        about: crate::api::song::description_plain(&song).unwrap_or_default(),
                    });

                    match crate::api::song::api_lyrics(&song) {
                        Some(lyr) => {
                            title.set_label(&title_txt);
                            sub.set_label(&sub_txt);
                            load_cover(&st2, art_url, cover);
                            show_lyrics(
                                lyr,
                                &view,
                                &cur,
                                &annots.borrow(),
                                &annot_lines,
                                &hover_line,
                            );
                            page.show_body();
                        }
                        None => {
                            // header and lyrics are shown as one block once the text loads
                            fetch_lyrics_web(
                                webview,
                                handler,
                                view,
                                spin,
                                cur,
                                annots,
                                annot_lines,
                                hover_line,
                                page,
                                song.url.clone(),
                                tr,
                                title,
                                sub,
                                cover,
                                title_txt,
                                sub_txt,
                                art_url,
                                st2,
                            );
                        }
                    }
                }
                Err(e) => {
                    page.show_status(
                        tr.error_title,
                        Some(&e.to_string()),
                        "dialog-error-symbolic",
                    );
                }
            },
        );

        // Annotations for lyric lines, fetched in parallel. Once they arrive, annotated
        // lines get marked and a click on them opens the explanation. If lyrics
        // are already on screen, redraw them
        let rc_annots = self.annotations.clone();
        let rc_view = view_ann;
        let rc_cur = self.cur_lyrics.clone();
        let rc_annot_lines = self.annot_lines.clone();
        let rc_hover_line = self.hover_line.clone();
        rc_annots.borrow_mut().clear();
        let client_a = client.clone();
        let st_a = state.clone();
        st_a.async_fetch(
            move || crate::api::song::annotations(&client_a, id).map_err(|e| e.to_string()),
            move |res| match res {
                Ok(list) if !list.is_empty() => {
                    *rc_annots.borrow_mut() = list;
                    let cur_txt = rc_cur.borrow().clone();
                    if !cur_txt.is_empty() {
                        show_lyrics(
                            cur_txt,
                            &rc_view,
                            &rc_cur,
                            &rc_annots.borrow(),
                            &rc_annot_lines,
                            &rc_hover_line,
                        );
                    }
                }
                _ => {}
            },
        );
    }
}

/// Loads the song URL in a hidden WebKitWebView and extracts the lyrics via
/// `evaluate_javascript`. On failure, shows a status page with a retry button
#[allow(clippy::too_many_arguments)]
fn fetch_lyrics_web(
    webview: webkit6::WebView,
    handler: Rc<RefCell<Option<glib::SignalHandlerId>>>,
    view: gtk4::TextView,
    spin: gtk4::Spinner,
    cur: Rc<RefCell<String>>,
    annots: Rc<RefCell<Vec<LyricAnnotation>>>,
    annot_lines: Rc<RefCell<HashSet<i32>>>,
    hover_line: Rc<RefCell<Option<i32>>>,
    page: Rc<Page>,
    url: String,
    tr: &'static Strings,
    title: gtk4::Label,
    sub: gtk4::Label,
    cover: gtk4::Picture,
    title_txt: String,
    sub_txt: String,
    art_url: String,
    state: Rc<AppState>,
) {
    // Until the text arrives, show only the loader, no header and no cover
    view.buffer().set_text("");
    spin.set_spinning(false);
    spin.set_visible(false);
    if let Some(h) = handler.borrow_mut().take() {
        webview.disconnect(h);
    }

    let w = webview.clone();
    let v = view.clone();
    let s = spin.clone();
    let c = cur.clone();
    let p = page.clone();
    let u = url.clone();
    let hd = handler.clone();
    let st = state.clone();
    let an = annots.clone();
    let anl = annot_lines.clone();
    let hl = hover_line.clone();
    let tl = title.clone();
    let sl = sub.clone();
    let cv = cover.clone();

    let hid = w.connect_load_changed(move |wv, ev| {
        if ev == webkit6::LoadEvent::Finished {
            let wv2 = wv.clone();
            let vc = v.clone();
            let sc = s.clone();
            let cc = c.clone();
            let pc = p.clone();
            let stc = st.clone();
            let hdc = hd.clone();
            let uc = u.clone();
            let anc = an.clone();
            let anlc = anl.clone();
            let hlc = hl.clone();
            let tlc = tl.clone();
            let slc = sl.clone();
            let cvc = cv.clone();
            let ttxt = title_txt.clone();
            let stxt = sub_txt.clone();
            let aurl = art_url.clone();
            wv.evaluate_javascript(
                LYRICS_JS,
                None,
                None,
                None::<&gio::Cancellable>,
                move |res| {
                    let text: String = res
                        .ok()
                        .map(|val| val.to_str().to_string())
                        .unwrap_or_default();
                    let lyr = text.trim().to_string();
                    if !lyr.is_empty() {
                        // Header and text are shown as one block after the lyrics load
                        tlc.set_label(&ttxt);
                        slc.set_label(&stxt);
                        load_cover(&stc, aurl.clone(), cvc);
                        show_lyrics(lyr, &vc, &cc, &anc.borrow(), &anlc, &hlc);
                        sc.set_spinning(false);
                        sc.set_visible(false);
                        pc.show_body();
                    } else {
                        sc.set_spinning(false);
                        sc.set_visible(false);
                        let action = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
                        action.set_valign(gtk4::Align::Center);
                        let retry = gtk4::Button::builder()
                            .child(&{
                                let c = libadwaita::ButtonContent::builder()
                                    .icon_name("view-refresh-symbolic")
                                    .label(tr.no_lyrics_action)
                                    .use_underline(true)
                                    .build();
                                c.upcast::<gtk4::Widget>()
                            })
                            .css_classes(["suggested-action"])
                            .build();
                        action.append(&retry);
                        pc.status().set_child(Some(&action));

                        let wvr = wv2.clone();
                        let str = stc.clone();
                        let vr = vc.clone();
                        let sr = sc.clone();
                        let cr = cc.clone();
                        let pr = pc.clone();
                        let hdr = hdc.clone();
                        let urlr = uc.clone();
                        let anr = anc.clone();
                        let anlr = anlc.clone();
                        let hlr = hlc.clone();
                        let trl = tlc.clone();
                        let srl = slc.clone();
                        let cvr = cvc.clone();
                        let tt_r = ttxt.clone();
                        let st_r = stxt.clone();
                        let au_r = aurl.clone();
                        retry.connect_clicked(move |_| {
                            fetch_lyrics_web(
                                wvr.clone(),
                                hdr.clone(),
                                vr.clone(),
                                sr.clone(),
                                cr.clone(),
                                anr.clone(),
                                anlr.clone(),
                                hlr.clone(),
                                pr.clone(),
                                urlr.clone(),
                                tr,
                                trl.clone(),
                                srl.clone(),
                                cvr.clone(),
                                tt_r.clone(),
                                st_r.clone(),
                                au_r.clone(),
                                str.clone(),
                            );
                        });

                        *cc.borrow_mut() = String::new();
                        pc.show_status(
                            tr.no_lyrics_title,
                            Some(tr.no_lyrics_desc),
                            "audio-x-generic-symbolic",
                        );
                    }
                },
            );
        }
    });
    *handler.borrow_mut() = Some(hid);
    webview.load_uri(&url);
}
