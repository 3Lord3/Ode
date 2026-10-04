mod api;
mod app;
mod cache;
mod config;
mod i18n;
mod ui;

use app::run;
use gtk4::glib;

fn main() -> glib::ExitCode {
    run()
}
