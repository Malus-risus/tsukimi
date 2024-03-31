use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::ui::network::Latest;

use self::imp::Page;
use super::item::ItemPage;
use super::movie::MoviePage;
use super::window::Window;

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    pub enum Page {
        Movie(Box<gtk::Widget>),
        Item(Box<gtk::Widget>),
    }

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/home.ui")]
    pub struct HomePage {
        #[template_child]
        pub root: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub libscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub librevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub liblist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub libsbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub libsrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for HomePage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "HomePage";
        type Type = super::HomePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for HomePage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            // request library
            obj.set_library();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for HomePage {}

    // Trait shared by all windows
    impl WindowImpl for HomePage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for HomePage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for HomePage {}
}

glib::wrapper! {
    pub struct HomePage(ObjectSubclass<imp::HomePage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

impl HomePage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    fn set(&self, page: Page) {
        let imp = self.imp();
        let widget = match page {
            Page::Movie(widget) => widget,
            Page::Item(widget) => widget,
        };
        imp.root.set_child(Some(&*widget));
    }

    pub fn set_library(&self) {
        let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::View>>(3);
        crate::ui::network::runtime().spawn(async move {
            let views = crate::ui::network::get_library().await.expect("msg");
            sender.send(views).await.expect("msg");
        });
        glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
            while let Ok(views) = receiver.recv().await {
                obj.set_libraryscorll(&views);
                obj.get_librarysscroll(&views);
            }
        }));
    }

    pub fn set_libraryscorll(&self, views: &Vec<crate::ui::network::View>) {
        let imp = self.imp();
        let libscrolled = imp.libscrolled.get();
        imp.librevealer.set_reveal_child(true);
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for view in views {
            let object = glib::BoxedAnyObject::new(view.clone());
            store.append(&object);
        }
        imp.selection.set_model(Some(&store));
        let selection = &imp.selection;
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .height_request(150)
                .width_request(300)
                .build();
            let label = gtk::Label::builder()
                .halign(gtk::Align::Center)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            listbox.append(&picture);
            listbox.append(&label);
            list_item.set_child(Some(&listbox));
        });
        factory.connect_bind(move |_, item| {
            let picture = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .first_child()
                .expect("Needs to be Picture");
            let label = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .last_child()
                .expect("Needs to be Picture");
            let entry = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("Needs to be BoxedAnyObject");
            let view: std::cell::Ref<crate::ui::network::View> = entry.borrow();
            if picture.is::<gtk::Box>() {
                if let Some(_revealer) = picture
                    .downcast_ref::<gtk::Box>()
                    .expect("Needs to be Box")
                    .first_child()
                {
                } else {
                    let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
                    let img = crate::ui::image::setimage(view.Id.clone(), mutex.clone());
                    picture
                        .downcast_ref::<gtk::Box>()
                        .expect("Needs to be Box")
                        .append(&img);
                }
            }
            if label.is::<gtk::Label>() {
                let str = format!("{}", view.Name);
                label
                    .downcast_ref::<gtk::Label>()
                    .expect("Needs to be Label")
                    .set_text(&str);
            }
        });
        imp.liblist.set_factory(Some(&factory));
        imp.liblist.set_model(Some(selection));
        let liblist = imp.liblist.get();
        libscrolled.set_child(Some(&liblist));
    }

    pub fn get_librarysscroll(&self, views: &Vec<crate::ui::network::View>) {
        let libsrevealer = self.imp().libsrevealer.get();
        libsrevealer.set_reveal_child(true);
        for view in views.clone() {
            let libsbox = self.imp().libsbox.get();
            let scrolledwindow = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Automatic)
                .vscrollbar_policy(gtk::PolicyType::Never)
                .overlay_scrolling(true)
                .build();
            let scrollbox = gtk::Box::new(gtk::Orientation::Vertical, 15);
            let revealer = gtk::Revealer::builder()
                .transition_type(gtk::RevealerTransitionType::SlideDown)
                .transition_duration(700)
                .reveal_child(false)
                .child(&scrollbox)
                .build();
            libsbox.append(&revealer);
            let label = gtk::Label::builder()
                .label(format!("<b>Latest {}</b>", view.Name))
                .halign(gtk::Align::Start)
                .use_markup(true)
                .margin_top(15)
                .margin_start(10)
                .build();
            scrollbox.append(&label);
            scrollbox.append(&scrolledwindow);
            let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
            let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::Latest>>(3);
            crate::ui::network::runtime().spawn(async move {
                let latest = crate::ui::network::get_latest(view.Id.clone(), mutex)
                    .await
                    .expect("msg");
                sender.send(latest).await.expect("msg");
            });
            glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
                while let Ok(latest) = receiver.recv().await {
                    obj.set_librarysscroll(latest.clone());
                    let listview = obj.set_librarysscroll(latest);
                    scrolledwindow.set_child(Some(&listview));
                    revealer.set_reveal_child(true);
                }
            }));
        }
        self.imp().spinner.set_visible(false);
    }

    pub fn set_librarysscroll(&self, latests: Vec<crate::ui::network::Latest>) -> gtk::ListView {
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for latest in latests {
            let object = glib::BoxedAnyObject::new(latest.clone());
            store.append(&object);
        }
        let selection = gtk::SingleSelection::new(Some(store));
        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .height_request(240)
                .width_request(167)
                .build();
            let label = gtk::Label::builder()
                .halign(gtk::Align::Center)
                .justify(gtk::Justification::Center)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            listbox.append(&picture);
            listbox.append(&label);
            list_item.set_child(Some(&listbox));
        });
        factory.connect_bind(move |_, item| {
            let picture = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .first_child()
                .expect("Needs to be Picture");
            let label = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .last_child()
                .expect("Needs to be Picture");
            let entry = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("Needs to be BoxedAnyObject");
            let latest: std::cell::Ref<crate::ui::network::Latest> = entry.borrow();
            if latest.Type == "MusicAlbum" {
                picture.set_size_request(167, 167);
            }
            if picture.is::<gtk::Box>() {
                if let Some(_revealer) = picture
                    .downcast_ref::<gtk::Box>()
                    .expect("Needs to be Box")
                    .first_child()
                {
                } else {
                    let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
                    let img = crate::ui::image::setimage(latest.Id.clone(), mutex.clone());
                    let overlay = gtk::Overlay::builder().child(&img).build();
                    if let Some(userdata) = &latest.UserData {
                        if let Some(unplayeditemcount) = userdata.UnplayedItemCount {
                            if unplayeditemcount > 0 {
                                let mark = gtk::Label::new(Some(
                                    &userdata
                                        .UnplayedItemCount
                                        .expect("no unplayeditemcount")
                                        .to_string(),
                                ));
                                mark.set_valign(gtk::Align::Start);
                                mark.set_halign(gtk::Align::End);
                                mark.set_height_request(40);
                                mark.set_width_request(40);
                                overlay.add_overlay(&mark);
                            }
                        }
                    }
                    picture
                        .downcast_ref::<gtk::Box>()
                        .expect("Needs to be Box")
                        .append(&overlay);
                }
            }
            if label.is::<gtk::Label>() {
                let mut str = format!("{}", latest.Name);
                if let Some(productionyear) = latest.ProductionYear {
                    str.push_str(&format!("\n{}", productionyear));
                }
                label
                    .downcast_ref::<gtk::Label>()
                    .expect("Needs to be Label")
                    .set_text(&str);
            }
        });
        let listview = gtk::ListView::new(Some(selection), Some(factory));
        listview.set_orientation(gtk::Orientation::Horizontal);
        listview.connect_activate(glib::clone!(@weak self as obj => move |listview, position| {
            let model = listview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<Latest> = item.borrow();
                let item_page;
                if result.Type == "Movie" {
                    item_page = Page::Movie(Box::new(MoviePage::new(result.Id.clone(),result.Name.clone()).into()));
                    obj.set(item_page);
                } else if result.Type == "Series" {
                    item_page = Page::Item(Box::new(ItemPage::new(result.Id.clone(),result.Id.clone()).into()));
                    obj.set(item_page);
                } else {
                    let toast = adw::Toast::builder()
                        .title(format!("{} is not supported",result.Type))
                        .timeout(3)
                        .build();
                    obj.imp().toast.add_toast(toast);
                }
                let window = obj.root();
                if let Some(window) = window {
                    if window.is::<Window>() {
                        let window = window.downcast::<Window>().unwrap();
                        window.set_title(&result.Name);
                    }
                }
        }));
        listview
    }
}
