use adw::subclass::prelude::*;
use glib::Object;
use gtk::prelude::*;
use gtk::{gio, glib};
mod imp {
    use crate::ui::network::{self, runtime, SearchResult};
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;
    #[cfg(windows)]
    use std::env;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/movie.ui")]
    #[properties(wrapper_type = super::MoviePage)]
    pub struct MoviePage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub moviename: OnceCell<String>,
        #[template_child]
        pub dropdownspinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub backdrop: TemplateChild<gtk::Picture>,
        #[template_child]
        pub osdbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub logobox: TemplateChild<gtk::Box>,
        #[template_child]
        pub mediainfobox: TemplateChild<gtk::Box>,
        #[template_child]
        pub linksscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub mediainforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub linksrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub actorrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub actorscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub actorlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub overviewrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub itemoverview: TemplateChild<gtk::Inscription>,
        pub selection: gtk::SingleSelection,
        pub actorselection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MoviePage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MoviePage";
        type Type = super::MoviePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for MoviePage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let id = obj.id();
            let name = obj.moviename();

            #[cfg(unix)]
            let pathbuf = dirs::home_dir()
                .unwrap()
                .join(format!(".local/share/tsukimi/b{}.png", id));
            #[cfg(windows)]
            let pathbuf = env::current_dir()
                .unwrap()
                .join("thumbnails")
                .join(format!("b{}.png", id));

            let backdrop = self.backdrop.get();
            let (sender, receiver) = async_channel::bounded::<String>(1);
            let idclone = id.clone();
            if pathbuf.exists() {
                backdrop.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
            } else {
                crate::ui::network::runtime().spawn(async move {
                    let id = crate::ui::network::get_backdropimage(idclone)
                        .await
                        .expect("msg");
                    sender
                        .send(id.clone())
                        .await
                        .expect("The channel needs to be open.");
                });
            }

            let idclone = id.clone();
            let idclonet = id.clone();
            let idc = id.clone();
            let name = name.clone();
            glib::spawn_future_local(async move {
                while let Ok(_) = receiver.recv().await {
                    #[cfg(unix)]
                    let path = dirs::home_dir()
                        .unwrap()
                        .join(format!(".local/share/tsukimi/b{}.png", idclone));
                    #[cfg(windows)]
                    let path = env::current_dir()
                        .unwrap()
                        .join("thumbnails")
                        .join(format!("b{}.png", idclone));

                    let file = gtk::gio::File::for_path(&path);
                    backdrop.set_file(Some(&file));
                }
            });
            let logobox = self.logobox.get();
            obj.logoset(logobox);
            let dropdownspinner = self.dropdownspinner.get();
            let osdbox = self.osdbox.get();
            dropdownspinner.set_visible(true);
            let (sender, receiver) = async_channel::bounded::<crate::ui::network::Media>(1);
            runtime().spawn(async move {
                let playback = network::playbackinfo(id).await.expect("msg");
                sender.send(playback).await.expect("msg");
            });
            glib::spawn_future_local(
                glib::clone!(@weak dropdownspinner,@weak osdbox =>async move {
                    while let Ok(playback) = receiver.recv().await {
                        let info:SearchResult = SearchResult {
                            Id: idclonet.clone(),
                            Name: name.clone(),
                            Type: String::from("Movie"),
                            UserData: None,
                        };
                        let dropdown = crate::ui::moviedrop::newmediadropsel(playback, info);
                        dropdownspinner.set_visible(false);
                        osdbox.append(&dropdown);
                    }
                }),
            );
            obj.setoverview();
            obj.createmediabox(idc);
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for MoviePage {}

    // Trait shared by all windows
    impl WindowImpl for MoviePage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for MoviePage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for MoviePage {}
}

glib::wrapper! {
    pub struct MoviePage(ObjectSubclass<imp::MoviePage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MoviePage {
    pub fn new(id: String, name: String) -> Self {
        Object::builder()
            .property("id", id)
            .property("moviename", name)
            .build()
    }

    pub fn logoset(&self, osd: gtk::Box) {
        let id = self.id();
        let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
        let logo = crate::ui::image::setlogoimage(id.clone(), mutex.clone());
        osd.append(&logo);
        osd.add_css_class("logo");
    }

    pub fn setoverview(&self) {
        let imp = self.imp();
        let id = imp.id.get().unwrap().clone();
        let itemoverview = imp.itemoverview.get();
        let overviewrevealer = imp.overviewrevealer.get();
        let (sender, receiver) = async_channel::bounded::<crate::ui::network::Item>(1);
        crate::ui::network::runtime().spawn(async move {
            let item = crate::ui::network::get_item_overview(id.to_string())
                .await
                .expect("msg");
            sender.send(item).await.expect("msg");
        });
        glib::spawn_future_local(glib::clone!(@weak self as obj=>async move {
            while let Ok(item) = receiver.recv().await {
                if let Some(overview) = item.Overview {
                    itemoverview.set_text(Some(&overview));
                }
                if let Some(links) = item.ExternalUrls {
                    obj.setlinksscrolled(links);
                }
                if let Some(actor) = item.People {
                    obj.setactorscrolled(actor);
                }
                overviewrevealer.set_reveal_child(true);
            }
        }));
    }

    pub fn createmediabox(&self, id: String) {
        let imp = self.imp();
        let mediainfobox = imp.mediainfobox.get();
        let mediainforevealer = imp.mediainforevealer.get();
        let (sender, receiver) = async_channel::bounded::<crate::ui::network::Media>(1);
        crate::ui::network::runtime().spawn(async move {
            let media = crate::ui::network::get_mediainfo(id.to_string())
                .await
                .expect("msg");
            sender.send(media).await.expect("msg");
        });
        glib::spawn_future_local(async move {
            while let Ok(media) = receiver.recv().await {
                while mediainfobox.last_child() != None {
                    mediainfobox
                        .last_child()
                        .map(|child| mediainfobox.remove(&child));
                }
                for mediasource in media.MediaSources {
                    let singlebox = gtk::Box::new(gtk::Orientation::Vertical, 5);
                    let info = format!(
                        "{} {}\n{}",
                        mediasource.Container.to_uppercase(),
                        bytefmt::format(mediasource.Size),
                        mediasource.Name
                    );
                    let label = gtk::Label::builder()
                        .label(&info)
                        .halign(gtk::Align::Start)
                        .margin_start(15)
                        .valign(gtk::Align::Start)
                        .margin_top(5)
                        .build();
                    singlebox.append(&label);

                    let mediascrolled = gtk::ScrolledWindow::builder()
                        .hscrollbar_policy(gtk::PolicyType::Automatic)
                        .vscrollbar_policy(gtk::PolicyType::Never)
                        .overlay_scrolling(true)
                        .sensitive(false)
                        .build();

                    let mediabox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
                    for mediapart in mediasource.MediaStreams {
                        if mediapart.Type == "Attachment" {
                            continue;
                        }
                        let mediapartbox = gtk::Box::builder()
                            .orientation(gtk::Orientation::Vertical)
                            .spacing(0)
                            .width_request(300)
                            .build();
                        let mut str: String = Default::default();
                        let icon = gtk::Image::builder().margin_end(5).build();
                        if mediapart.Type == "Video" {
                            icon.set_from_icon_name(Some("video-x-generic-symbolic"))
                        } else if mediapart.Type == "Audio" {
                            icon.set_from_icon_name(Some("audio-x-generic-symbolic"))
                        } else if mediapart.Type == "Subtitle" {
                            icon.set_from_icon_name(Some("media-view-subtitles-symbolic"))
                        } else {
                            icon.set_from_icon_name(Some("text-x-generic-symbolic"))
                        }
                        let typebox = gtk::Box::builder()
                            .orientation(gtk::Orientation::Horizontal)
                            .spacing(5)
                            .build();
                        typebox.append(&icon);
                        typebox.append(&gtk::Label::new(Some(&mediapart.Type)));
                        if let Some(codec) = mediapart.Codec {
                            str.push_str(format!("Codec: {}", codec).as_str());
                        }
                        if let Some(language) = mediapart.DisplayLanguage {
                            str.push_str(format!("\nLanguage: {}", language).as_str());
                        }
                        if let Some(title) = mediapart.Title {
                            str.push_str(format!("\nTitle: {}", title).as_str());
                        }
                        if let Some(bitrate) = mediapart.BitRate {
                            str.push_str(
                                format!("\nBitrate: {}it/s", bytefmt::format(bitrate)).as_str(),
                            );
                        }
                        if let Some(bitdepth) = mediapart.BitDepth {
                            str.push_str(format!("\nBitDepth: {} bit", bitdepth).as_str());
                        }
                        if let Some(samplerate) = mediapart.SampleRate {
                            str.push_str(format!("\nSampleRate: {} Hz", samplerate).as_str());
                        }
                        if let Some(height) = mediapart.Height {
                            str.push_str(format!("\nHeight: {}", height).as_str());
                        }
                        if let Some(width) = mediapart.Width {
                            str.push_str(format!("\nWidth: {}", width).as_str());
                        }
                        if let Some(colorspace) = mediapart.ColorSpace {
                            str.push_str(format!("\nColorSpace: {}", colorspace).as_str());
                        }
                        if let Some(displaytitle) = mediapart.DisplayTitle {
                            str.push_str(format!("\nDisplayTitle: {}", displaytitle).as_str());
                        }
                        if let Some(channel) = mediapart.Channels {
                            str.push_str(format!("\nChannel: {}", channel).as_str());
                        }
                        if let Some(channellayout) = mediapart.ChannelLayout {
                            str.push_str(format!("\nChannelLayout: {}", channellayout).as_str());
                        }
                        if let Some(averageframerate) = mediapart.AverageFrameRate {
                            str.push_str(
                                format!("\nAverageFrameRate: {}", averageframerate).as_str(),
                            );
                        }
                        if let Some(pixelformat) = mediapart.PixelFormat {
                            str.push_str(format!("\nPixelFormat: {}", pixelformat).as_str());
                        }
                        let inscription = gtk::Inscription::builder()
                            .text(&str)
                            .min_lines(14)
                            .hexpand(true)
                            .yalign(0.0)
                            .build();
                        mediapartbox.append(&typebox);
                        mediapartbox.append(&inscription);
                        mediabox.append(&mediapartbox);
                    }

                    mediascrolled.set_child(Some(&mediabox));
                    singlebox.append(&mediascrolled);
                    mediainfobox.append(&singlebox);
                }
                mediainforevealer.set_reveal_child(true);
            }
        });
    }

    pub fn setlinksscrolled(&self, links: Vec<crate::ui::network::Urls>) {
        let imp = self.imp();
        let linksscrolled = imp.linksscrolled.get();
        let linksrevealer = imp.linksrevealer.get();
        if !links.is_empty() {
            linksrevealer.set_reveal_child(true);
        }
        let linkbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        while linkbox.last_child() != None {
            linkbox.last_child().map(|child| linkbox.remove(&child));
        }
        for url in links {
            let linkbutton = gtk::Button::builder()
                .margin_start(10)
                .margin_top(10)
                .build();
            let buttoncontent = adw::ButtonContent::builder()
                .label(&url.Name)
                .icon_name("send-to-symbolic")
                .build();
            linkbutton.set_child(Some(&buttoncontent));
            linkbutton.connect_clicked(move |_| {
                let _ = gio::AppInfo::launch_default_for_uri(
                    &url.Url,
                    Option::<&gio::AppLaunchContext>::None,
                );
            });
            linkbox.append(&linkbutton);
        }
        linksscrolled.set_child(Some(&linkbox));
        linksrevealer.set_reveal_child(true);
    }

    pub fn setactorscrolled(&self, actors: Vec<crate::ui::network::People>) {
        let imp = self.imp();
        let actorscrolled = imp.actorscrolled.get();
        let actorrevealer = imp.actorrevealer.get();
        if !actors.is_empty() {
            actorrevealer.set_reveal_child(true);
        }
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for people in actors {
            let object = glib::BoxedAnyObject::new(people);
            store.append(&object);
        }
        imp.actorselection.set_model(Some(&store));
        let actorselection = &imp.actorselection;
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .height_request(200)
                .width_request(150)
                .build();
            let label = gtk::Label::builder()
                .halign(gtk::Align::Start)
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
            let people: std::cell::Ref<crate::ui::network::People> = entry.borrow();
            if picture.is::<gtk::Box>() {
                if let Some(_revealer) = picture
                    .downcast_ref::<gtk::Box>()
                    .expect("Needs to be Box")
                    .first_child()
                {
                } else {
                    let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
                    let img = crate::ui::image::setimage(people.Id.clone(), mutex.clone());
                    picture
                        .downcast_ref::<gtk::Box>()
                        .expect("Needs to be Box")
                        .append(&img);
                }
            }
            if label.is::<gtk::Label>() {
                if let Some(role) = &people.Role {
                    let str = format!("{}\n{}", people.Name, role);
                    label
                        .downcast_ref::<gtk::Label>()
                        .expect("Needs to be Label")
                        .set_text(&str);
                }
            }
        });
        imp.actorlist.set_factory(Some(&factory));
        imp.actorlist.set_model(Some(actorselection));
        let actorlist = imp.actorlist.get();
        actorscrolled.set_child(Some(&actorlist));
        actorrevealer.set_reveal_child(true);
    }
}
