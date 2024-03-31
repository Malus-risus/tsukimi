use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use self::imp::Page;

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{gio, glib, CompositeTemplate, Label};

    use crate::ui::widgets::item::ItemPage;
    use crate::ui::widgets::movie::MoviePage;
    use crate::ui::widgets::window::Window;

    pub enum Page {
        Movie(Box<gtk::Widget>),
        Item(Box<gtk::Widget>),
    }

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/search.ui")]
    pub struct SearchPage {
        #[template_child]
        pub searchentry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub searchgrid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub searchscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub searchrevealer: TemplateChild<gtk::Revealer>,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for SearchPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "SearchPage";
        type Type = super::SearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for SearchPage {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();
            let spinner = self.spinner.get();
            let searchrevealer = self.searchrevealer.get();

            let (sender, receiver) =
                async_channel::bounded::<Vec<crate::ui::network::SearchResult>>(1);
            self.searchentry.connect_activate(glib::clone!(@strong sender,@weak spinner=> move |entry| {
                spinner.set_visible(true);
                let search_content = entry.text().to_string();
                crate::ui::network::runtime().spawn(glib::clone!(@strong sender => async move {
                    let search_results = crate::ui::network::search(search_content).await.unwrap_or_else(|e| {
                        eprintln!("Error: {}", e);
                        Vec::<crate::ui::network::SearchResult>::new()
                    });
                    sender.send(search_results).await.expect("search results not received.");
                }));
            }));

            let store = gio::ListStore::new::<glib::BoxedAnyObject>();
            glib::spawn_future_local(glib::clone!(@weak store=> async move {
                while let Ok(search_results) = receiver.recv().await {
                    spinner.set_visible(false);
                    store.remove_all();
                    for result in search_results {
                        if result.Type == "Series" || result.Type == "Movie" {
                            let object = glib::BoxedAnyObject::new(result);
                            store.append(&object);
                        }
                    }
                    searchrevealer.set_reveal_child(true);
                }
            }));

            self.selection.set_model(Some(&store));
            let factory = gtk::SignalListItemFactory::new();
            factory.connect_bind(|_, item| {
                let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
                let entry = listitem
                    .item()
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let result: std::cell::Ref<crate::ui::network::SearchResult> = entry.borrow();
                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
                let overlay = gtk::Overlay::new();
                let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
                let imgbox = crate::ui::image::set_image(result.Id.clone(), mutex);
                imgbox.set_size_request(167, 275);
                overlay.set_child(Some(&imgbox));
                if let Some(userdata) = &result.UserData {
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
                vbox.append(&overlay);
                let label = Label::new(Some(&result.Name));
                let markup = format!("{}", result.Name);
                label.set_markup(markup.as_str());
                label.set_wrap(true);
                label.set_size_request(-1, 24);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                let labeltype = Label::new(Some(&result.Type));
                let markup = format!(
                    "<span color='lightgray' font='small'>{}</span>",
                    result.Type
                );
                labeltype.set_markup(markup.as_str());
                labeltype.set_size_request(-1, 24);
                vbox.append(&label);
                vbox.append(&labeltype);
                listitem.set_child(Some(&vbox));
            });
            factory.connect_unbind(|_, item| {
                let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
                listitem.set_child(None::<&gtk::Widget>);
            });
            self.searchgrid.set_factory(Some(&factory));
            self.searchgrid.set_model(Some(&self.selection));
            self.searchgrid.connect_activate(glib::clone!(@weak obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<crate::ui::network::SearchResult> = item.borrow();
                let item_page;
                if result.Type == "Movie" {
                    item_page = Page::Movie(Box::new(MoviePage::new(result.Id.clone(),result.Name.clone()).into()));
                } else {
                    item_page = Page::Item(Box::new(ItemPage::new(result.Id.clone(),result.Id.clone()).into()));
                }
                obj.set(item_page);
                let window = obj.root();
                if let Some(window) = window {
                    if window.is::<Window>() {
                        let window = window.downcast::<Window>().unwrap();
                        window.set_title(&result.Name);
                    }
                }
            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for SearchPage {}

    // Trait shared by all windows
    impl WindowImpl for SearchPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for SearchPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SearchPage {}
}

glib::wrapper! {
    pub struct SearchPage(ObjectSubclass<imp::SearchPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for SearchPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    fn set(&self, page: crate::ui::widgets::search::imp::Page) {
        let imp = imp::SearchPage::from_obj(self);
        let widget = match page {
            Page::Movie(widget) => widget,
            Page::Item(widget) => widget,
        };
        imp.searchscrolled.set_child(Some(&*widget));
    }
}
