pub mod page;
pub mod search_page;
pub mod settings;
pub mod song_page;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk4::glib;

use crate::api::GeniusClient;
use crate::cache::Cache;
use crate::config::Config;
use crate::i18n::Tr;

/// Shared app state, behind an `Rc` across widgets and pages
pub struct AppState {
    pub token: RefCell<Option<String>>,
    pub cache: Arc<Cache>,
    pub tr: Rc<Tr>,
    pub toast: RefCell<Option<libadwaita::ToastOverlay>>,
}

impl AppState {
    pub fn new() -> Rc<Self> {
        let config = Config::load();
        let cache = Arc::new(Cache::new(config.cache_ttl_secs, config.cache_max_mb));
        Rc::new(Self {
            cache,
            token: RefCell::new(None),
            tr: Rc::new(Tr::from_system()),
            toast: RefCell::new(None),
        })
    }

    /// Show a toast on the main overlay
    pub fn show_toast(&self, msg: &str) {
        if let Some(ovl) = self.toast.borrow().as_ref() {
            ovl.add_toast(libadwaita::Toast::new(msg));
        }
    }

    pub fn client(&self) -> Option<GeniusClient> {
        self.token
            .borrow()
            .as_ref()
            .map(|t| GeniusClient::new(t.clone(), self.cache.clone()))
    }

    /// Run a blocking closure on a worker thread and deliver the result to the UI thread
    /// glib 0.21 dropped the glib channel, so an idle source polls a `std::sync::mpsc`
    /// receiver on the main thread. The cost is negligible: an empty queue just reschedules
    pub fn async_fetch<F, T, C>(&self, f: F, on_done: C)
    where
        F: FnOnce() -> Result<T, String> + Send + 'static,
        T: Send + 'static,
        C: FnOnce(Result<T, String>) + 'static,
    {
        fn pump<T, C>(rx: std::sync::mpsc::Receiver<Result<T, String>>, done: C)
        where
            T: Send + 'static,
            C: FnOnce(Result<T, String>) + 'static,
        {
            use std::sync::mpsc::TryRecvError;
            match rx.try_recv() {
                Ok(res) => done(res),
                Err(TryRecvError::Empty) => {
                    glib::idle_add_local_once(move || pump(rx, done));
                }
                Err(TryRecvError::Disconnected) => {}
            }
        }
        let (tx, rx) = std::sync::mpsc::channel::<Result<T, String>>();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        pump(rx, on_done);
    }
}
