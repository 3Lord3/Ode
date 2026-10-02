use std::cell::RefCell;
use std::rc::Rc;

use gtk4::{gio, glib};
use gtk4::prelude::*;
use libadwaita::prelude::*;

use crate::ui::search_page;
use crate::ui::settings::{self, Msg};
use crate::ui::song_page;
use crate::ui::AppState;

pub const APP_ID: &str = "org.example.Ode";

pub fn run() -> glib::ExitCode {
    let app = libadwaita::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::default())
        .build();

    app.set_accels_for_action("app.quit", &["<Control>q"]);
    app.set_accels_for_action("app.settings", &["<Control>comma"]);
    app.set_accels_for_action("app.about", &["F1"]);

    let app2 = app.clone();
    app.connect_activate(move |app| {
        build_window(&app2, app);
    });

    app.run()
}

fn build_window(app: &libadwaita::Application, _gapp: &libadwaita::Application) {
    let app = app.clone();
    let state = AppState::new();

    let window = libadwaita::ApplicationWindow::builder()
        .application(&app)
        .title(state.tr.strings().app_title)
        .default_width(1120)
        .default_height(760)
        .build();
    window.set_title(Some(state.tr.strings().app_title));

    // AdwNavigationView stack: search -> song page, with the built-in transitions
    // and a correct HeaderBar title
    let song = Rc::new(RefCell::new(song_page::build(&state, window.upcast_ref())));
    let nav = libadwaita::NavigationView::new();
    nav.set_vexpand(true);
    let song_page_nav = libadwaita::NavigationPage::builder()
        .tag("song")
        .title(state.tr.strings().app_title)
        .child(&song.borrow().content)
        .build();

    let search = search_page::build(
        &state,
        {
            let song = song.clone();
            let nav = nav.clone();
            let song_page_nav = song_page_nav.clone();
            let state = state.clone();
            Rc::new(move |id: u64| {
                song.borrow_mut().open(&state, id);
                nav.push(&song_page_nav);
            })
        },
        {
            let nav = nav.clone();
            Rc::new(move || {
                nav.pop();
            })
        },
    );
    let search_page_nav = libadwaita::NavigationPage::builder()
        .tag("search")
        .title(state.tr.strings().search_results)
        .child(&search.content)
        .build();
    nav.add(&search_page_nav);
    nav.add(&song_page_nav);
    nav.pop_to_page(&search_page_nav);

    // Leaving the song page: an explicit back button in the page body plus Escape,
    // which AdwNavigationView handles itself (pop-on-escape)
    let back = {
        let nav = nav.clone();
        let entry = search.entry.clone();
        move || {
            nav.pop();
            entry.grab_focus();
        }
    };
    song.borrow().set_back(back);

    // Pop-on-escape moves focus back to search but does not run the button handler,
    // so subscribe to the pop itself
    let entry_pop = search.entry.clone();
    nav.connect_popped(move |_, _| {
        entry_pop.grab_focus();
    });

    let entry = search.entry.clone();
    let act_search = gio::SimpleAction::new("search", None);
    act_search.connect_activate(move |_, _| {
        entry.grab_focus();
    });
    let actions = gio::SimpleActionGroup::new();
    actions.add_action(&act_search);
    window.insert_action_group("win", Some(&actions));
    app.set_accels_for_action("win.search", &["<Control>f"]);

    let header = libadwaita::HeaderBar::new();
    let se = search.entry.clone();
    header.pack_start(&se);

    let menu = gio::Menu::new();
    let section = gio::Menu::new();
    section.append(Some(state.tr.strings().settings), Some("app.settings"));
    section.append(Some(state.tr.strings().about), Some("app.about"));
    menu.append_section(None, &section);
    let pop = gtk4::PopoverMenu::from_model(Some(&menu));
    let menu_btn = gtk4::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .popover(&pop)
        .tooltip_text(state.tr.strings().settings)
        .build();
    header.pack_end(&menu_btn);

    let overlay = libadwaita::ToastOverlay::new();
    overlay.set_child(Some(&nav));
    *state.toast.borrow_mut() = Some(overlay.clone());

    // AdwToolbarView: header on top, content filling the rest
    let toolbar = libadwaita::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&overlay));
    window.set_content(Some(&toolbar));

    // The theme is left to the system default
    apply_token(&state);

    add_app_actions(&app, &state, window.upcast_ref());

    window.present();
}

/// Token used for API requests, embedded at build time (see api::credentials)
fn apply_token(state: &Rc<AppState>) {
    if let Some(t) = crate::api::credentials::token() {
        *state.token.borrow_mut() = Some(t);
    }
}

fn handle_msg(state: &Rc<AppState>, _window: &gtk4::Window, m: Msg) {
    match m {
        Msg::CacheCleared => state.show_toast(state.tr.strings().cache_cleared),
    }
}

fn add_app_actions(app: &libadwaita::Application, state: &Rc<AppState>, window: &gtk4::Window) {
    let state_s = state.clone();
    let win_s = window.clone();
    let act_settings = gio::SimpleAction::new("settings", None);
    act_settings.connect_activate(move |_, _| {
        let on_msg: Rc<dyn Fn(Msg)> = {
            let s = state_s.clone();
            let w = win_s.clone();
            Rc::new(move |m| handle_msg(&s, &w, m))
        };
        let win = settings::build(&state_s, &win_s, on_msg);
        win.present();
    });
    app.add_action(&act_settings);

    let state_a = state.clone();
    let win_a = window.clone();
    let act_about = gio::SimpleAction::new("about", None);
    act_about.connect_activate(move |_, _| {
        let dlg = libadwaita::AboutWindow::builder()
            .application_name(state_a.tr.strings().app_title)
            .version("0.1.0")
            .website("https://docs.genius.com/")
            .license_type(gtk4::License::MitX11)
            .transient_for(&win_a)
            .build();
        dlg.present();
    });
    app.add_action(&act_about);

    let act_quit = gio::SimpleAction::new("quit", None);
    let app_q = app.clone();
    act_quit.connect_activate(move |_, _| app_q.quit());
    app.add_action(&act_quit);
}