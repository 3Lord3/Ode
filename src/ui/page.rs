use gtk4::prelude::*;

/// Page shell: a root `GtkOverlay` holds an outer stack of a `body` +
/// `Adw.StatusPage` switch and a spinner, so the spinner covers the whole page.
/// States are switched with `show_loading / show_body / show_status`. The overlay
/// root allows a hidden but realized child, which is how the WebKitWebView loads
/// lyrics in background without ever being shown
pub struct Page {
    pub root: gtk4::Overlay,
    stack: gtk4::Stack,
    holder: gtk4::Stack,
    body: gtk4::Box,
    status: libadwaita::StatusPage,
    spinner: gtk4::Spinner,
}

impl Page {
    pub fn new() -> Self {
        let body = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        body.set_vexpand(true);

        let status = libadwaita::StatusPage::builder()
            .vexpand(true)
            .valign(gtk4::Align::Center)
            .build();

        let holder = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();
        holder.add_named(&body, Some("body"));
        holder.add_named(&status, Some("status"));
        holder.set_visible_child_name("body");

        // Spinner matches the status page icon size: 128x128
        let spinner = gtk4::Spinner::builder()
            .spinning(true)
            .width_request(128)
            .height_request(128)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();

        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();
        stack.add_named(&holder, Some("content"));
        stack.add_named(&spinner, Some("spinner"));
        stack.set_visible_child_name("content");

        let root = gtk4::Overlay::new();
        root.set_child(Some(&stack));

        Self {
            root,
            stack,
            holder,
            body,
            status,
            spinner,
        }
    }

    pub fn append<W: IsA<gtk4::Widget>>(&self, w: &W) {
        self.body.append(w);
    }

    /// Adds a hidden but realized widget on top, used for the lyrics WebKitWebView
    /// which has to be realized to load a page, but must not be visible
    pub fn attach_hidden<W: IsA<gtk4::Widget>>(&self, w: &W) {
        self.root.add_overlay(w);
        w.set_visible(false);
    }

    /// The `AdwStatusPage` itself, to attach an action button when needed
    pub fn status(&self) -> &libadwaita::StatusPage {
        &self.status
    }

    pub fn show_loading(&self) {
        self.spinner.start();
        self.stack.set_visible_child_name("spinner");
    }

    pub fn show_body(&self) {
        self.spinner.stop();
        self.stack.set_visible_child_name("content");
        self.holder.set_visible_child_name("body");
    }

    pub fn show_status(&self, title: &str, description: Option<&str>, icon: &str) {
        self.spinner.stop();
        self.status.set_title(title);
        self.status.set_description(description);
        self.status.set_icon_name(Some(icon));
        self.stack.set_visible_child_name("content");
        self.holder.set_visible_child_name("status");
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new()
    }
}
