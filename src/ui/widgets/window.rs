use adw::prelude::NavigationPageExt;
#[cfg(unix)]
use dirs::home_dir;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
#[cfg(windows)]
use std::env;
mod imp {
    use adw::subclass::application_window::AdwApplicationWindowImpl;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/window.ui")]
    pub struct Window {
        #[template_child]
        pub serverentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub portentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub nameentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub passwordentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub selectlist: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub loginbutton: TemplateChild<gtk::Button>,
        #[template_child]
        pub insidestack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub settingspage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub searchpage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub historypage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub homepage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub split_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub navipage: TemplateChild<adw::NavigationPage>,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "AppWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action_async("win.login", None, |window, _, _| async move {
                window.login().await;
            });
            klass.install_action("win.home", None, move |window, _action, _parameter| {
                window.homepage();
            });
            klass.install_action("win.history", None, move |window, _action, _parameter| {
                window.historypage();
            });
            klass.install_action("win.search", None, move |window, _action, _parameter| {
                window.searchpage();
            });
            klass.install_action("win.relogin", None, move |window, _action, _parameter| {
                window.placeholder();
            });
            klass.install_action("win.about", None, move |window, _action, _parameter| {
                window.about();
            });
            klass.install_action("win.sidebar", None, move |window, _action, _parameter| {
                window.sidebar();
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for Window {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
            let obj = self.obj().clone();
            obj.loginenter();
            obj.homepage();
            self.selectlist.connect_row_selected(move |_, row| {
                if let Some(row) = row {
                    let num = row.index();
                    match num {
                        0 => {
                            obj.homepage();
                        }
                        1 => {
                            obj.historypage();
                        }
                        2 => {
                            obj.searchpage();
                        }
                        3 => {
                            obj.settingspage();
                        }
                        _ => {}
                    }
                }
            });
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for Window {}

    // Trait shared by all windows
    impl WindowImpl for Window {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

use glib::Object;
use gtk::{gio, glib};

use crate::ui::network::runtime;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget, gtk::HeaderBar,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        // Create new window
        Object::builder().property("application", app).build()
    }

    pub fn set_title(&self, title: &str) {
        let imp = self.imp();
        imp.navipage.set_title(title);
    }

    fn imp(&self) -> &imp::Window {
        imp::Window::from_obj(self)
    }

    fn mainpage(&self) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("main");
    }

    fn placeholder(&self) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("placeholder");
    }

    fn about(&self) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("placeholder");
    }

    fn homepage(&self) {
        let imp = self.imp();
        imp.homepage
            .set_child(Some(&crate::ui::widgets::home::HomePage::new()));
        imp.insidestack.set_visible_child_name("homepage");
        imp.navipage.set_title("Home");
    }

    fn historypage(&self) {
        let imp = self.imp();
        imp.historypage
            .set_child(Some(&crate::ui::widgets::history::HistoryPage::new()));
        imp.insidestack.set_visible_child_name("historypage");
        imp.navipage.set_title("History");
    }

    fn searchpage(&self) {
        let imp = self.imp();
        imp.searchpage
            .set_child(Some(&crate::ui::widgets::search::SearchPage::new()));
        imp.insidestack.set_visible_child_name("searchpage");
        imp.navipage.set_title("Search");
    }

    fn settingspage(&self) {
        let imp = self.imp();
        imp.settingspage
            .set_child(Some(&crate::ui::widgets::settings::SettingsPage::new()));
        imp.insidestack.set_visible_child_name("settingspage");
        imp.navipage.set_title("Preferences");
    }

    fn sidebar(&self) {
        let imp = self.imp();
        imp.split_view
            .set_show_sidebar(!imp.split_view.shows_sidebar());
    }

    async fn login(&self) {
        let imp = self.imp();
        imp.loginbutton.set_sensitive(false);
        let loginbutton = imp.loginbutton.clone();
        let server = imp.serverentry.text().to_string();
        let port = imp.portentry.text().to_string();
        let name = imp.nameentry.text().to_string();
        let password = imp.passwordentry.text().to_string();
        let (sender, receiver) = async_channel::bounded::<String>(1);
        let selfc = self.clone();
        runtime().spawn(async move {
            match crate::ui::network::login(server, name, password, port).await {
                Ok(_) => {
                    sender
                        .send("1".to_string())
                        .await
                        .expect("The channel needs to be open.");
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        });
        glib::MainContext::default().spawn_local(async move {
            match receiver.recv().await {
                Ok(_) => {
                    loginbutton.set_sensitive(false);
                    selfc.mainpage();
                }
                Err(_) => {
                    loginbutton.set_sensitive(true);
                    loginbutton.set_label("Link Failed");
                }
            }
        });
    }

    fn loginenter(&self) {
        #[cfg(unix)]
        let path = home_dir().unwrap().join(".config/tsukimi.toml");
        #[cfg(windows)]
        let path = env::current_dir()
            .unwrap()
            .join("config")
            .join("tsukimi.toml");

        if path.exists() {
            self.mainpage();
        }
    }
}
