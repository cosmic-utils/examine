// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use cosmic::app::{Command, Core};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, icon, menu, nav_bar, settings, row, list_column};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use futures_util::SinkExt;
use std::{collections::HashMap, path::PathBuf, fs, str::FromStr};
use etc_os_release::OsRelease;
use itertools::Itertools;

const REPOSITORY: &str = "https://github.com/sungsphinx/examine";
const APP_ICON: &[u8] = include_bytes!("../res/icons/hicolor/scalable/apps/page.codeberg.sungsphinx.Examine.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: Config,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "page.codeberg.sungsphinx.Examine";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        // Create a nav bar with three page items.
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("distribution"))
            .data::<Page>(Page::DistributionPage)
            .icon(icon::from_name("applications-system-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("processor"))
            .data::<Page>(Page::ProcessorPage)
            .icon(icon::from_name("system-run-symbolic"));

        // nav.insert()
        //     .text(fl!("page-id", num = 3))
        //     .data::<Page>(Page::Page3)
        //     .icon(icon::from_name("applications-games-symbolic"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    }
                })
                .unwrap_or_default(),
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<Element<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => self.about(),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let page = self.nav.data::<Page>(self.nav.active());
        let is_flatpak = PathBuf::from("/.flatpak-info").exists();
        let spacing = theme::active().cosmic().spacing;

        let content: Element<Self::Message> = match page {
            Some(Page::DistributionPage) => {
                let osrelease;
                if is_flatpak {
                    osrelease = OsRelease::from_str(&fs::read_to_string("/run/host/os-release").unwrap()).unwrap();
                } else {
                    osrelease = OsRelease::open().unwrap();
                };

                let mut list = list_column();

                list = list.add(settings::item(fl!("pretty-name"), widget::text::body(String::from(osrelease.pretty_name()))));
                list = list.add(settings::item(fl!("name"), widget::text::body(String::from(osrelease.name()))));
                if String::from(osrelease.version().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("version"), widget::text::body(String::from(osrelease.version().unwrap_or_default()))));
                }
                if String::from(osrelease.version_id().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("version-id"), widget::text::body(String::from(osrelease.version_id().unwrap_or_default()))));
                }
                list = list.add(settings::item(fl!("id"), widget::text::body(String::from(osrelease.id()))));
                if osrelease.id_like().is_some() {
                    list = list.add(settings::item(fl!("id-like"), widget::text::body(String::from(osrelease.id_like().unwrap().intersperse(", ").collect::<String>()))));
                }
                if String::from(osrelease.version_codename().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("version-codename"), widget::text::body(String::from(osrelease.version_codename().unwrap_or_default()))));
                }
                if String::from(osrelease.build_id().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("build-id"), widget::text::body(String::from(osrelease.build_id().unwrap_or_default()))));
                }
                if String::from(osrelease.image_id().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("image-id"), widget::text::body(String::from(osrelease.image_id().unwrap_or_default()))));
                }
                if String::from(osrelease.image_version().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("image-version"), widget::text::body(String::from(osrelease.image_version().unwrap_or_default()))));
                }
                if String::from(osrelease.vendor_name().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("vendor-name"), widget::text::body(String::from(osrelease.vendor_name().unwrap_or_default()))));
                }
                if osrelease.ansi_color().unwrap_or_default().is_empty() == false {
                    list = list.add(settings::item(fl!("ansi-color"), widget::text::body(String::from(osrelease.ansi_color().unwrap()))));
                }
                if osrelease.logo().unwrap_or_default().is_empty() == false {
                    list = list.add(settings::item(fl!("logo"), row::with_capacity(2)
                        .push(icon::from_name(String::from(osrelease.logo().unwrap())))
                        .push(widget::text::body(String::from(osrelease.logo().unwrap())))
                        .align_items(Alignment::Center)
                        .spacing(spacing.space_xxxs)
                    ));
                }
                if osrelease.cpe_name().unwrap_or_default().is_empty() == false {
                    list = list.add(settings::item(fl!("cpe-name"), widget::text::body(String::from(osrelease.cpe_name().unwrap()))));
                }
                if osrelease.home_url().unwrap_or_default().take().is_none() == false {
                    list = list.add(settings::item(fl!("home-url"), widget::text::body(String::from(osrelease.home_url().ok().unwrap().take().unwrap().as_str()))));
                }
                if osrelease.vendor_url().unwrap_or_default().take().is_none() == false {
                    list = list.add(settings::item(fl!("vendor-url"), widget::text::body(String::from(osrelease.vendor_url().ok().unwrap().take().unwrap().as_str()))));
                }
                if osrelease.documentation_url().unwrap_or_default().take().is_none() == false {
                    list = list.add(settings::item(fl!("doc-url"), widget::text::body(String::from(osrelease.documentation_url().ok().unwrap().take().unwrap().as_str()))));
                }
                if osrelease.support_url().unwrap_or_default().take().is_none() == false {
                    list = list.add(settings::item(fl!("support-url"), widget::text::body(String::from(osrelease.support_url().ok().unwrap().take().unwrap().as_str()))));
                }
                if osrelease.bug_report_url().unwrap_or_default().take().is_none() == false {
                    list = list.add(settings::item(fl!("bug-report-url"), widget::text::body(String::from(osrelease.bug_report_url().ok().unwrap().take().unwrap().as_str()))));
                }
                if osrelease.privacy_policy_url().unwrap_or_default().take().is_none() == false {
                    list = list.add(settings::item(fl!("privacy-policy-url"), widget::text::body(String::from(osrelease.privacy_policy_url().unwrap().take().unwrap().to_string()))));
                }
                if osrelease.support_end().unwrap_or_default().take().is_none() == false {
                    list = list.add(settings::item(fl!("support-end"), widget::text::body(String::from(osrelease.support_end().unwrap().take().unwrap().to_string()))));
                }
                if String::from(osrelease.variant().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("variant"), widget::text::body(String::from(osrelease.variant().unwrap_or_default()))));
                }
                if String::from(osrelease.variant_id().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("variant-id"), widget::text::body(String::from(osrelease.variant_id().unwrap_or_default()))));
                }
                if String::from(osrelease.default_hostname().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("default-hostname"), widget::text::body(String::from(osrelease.default_hostname().unwrap_or_default()))));
                }
                if String::from(osrelease.architecture().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item(fl!("arch"), widget::text::body(String::from(osrelease.architecture().unwrap_or_default()))));
                }
                if String::from(osrelease.sysext_level().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item("SYSEXT_LEVEL", widget::text::body(String::from(osrelease.sysext_level().unwrap_or_default()))));
                }
                if osrelease.sysext_scope().is_some() {
                    list = list.add(settings::item("SYSEXT_SCOPE", widget::text::body(String::from(osrelease.sysext_scope().unwrap().intersperse(", ").collect::<String>()))));
                }
                if String::from(osrelease.confext_level().unwrap_or_default()).is_empty() == false {
                    list = list.add(settings::item("CONFEXT_LEVEL", widget::text::body(String::from(osrelease.confext_level().unwrap_or_default()))));
                }
                if osrelease.confext_scope().is_some() {
                    list = list.add(settings::item("CONFEXT_SCOPE", widget::text::body(String::from(osrelease.confext_scope().unwrap().intersperse(", ").collect::<String>()))));
                }
                if osrelease.portable_prefixes().is_some() {
                    list = list.add(settings::item(fl!("portable-prefixes"), widget::text::body(String::from(osrelease.portable_prefixes().unwrap().intersperse(", ").collect::<String>()))));
                }

                widget::column::with_capacity(2)
                    .spacing(spacing.space_xxs)
                    .push(list)
                    .apply(widget::container)
                    .height(Length::Shrink)
                    .apply(widget::scrollable)
                    .height(Length::Fill)
                    .into()
            }
            Some(Page::ProcessorPage) => {
                let lscpu_cmd = std::process::Command::new("lscpu")
                        .output()
                        .unwrap();
                let lscpu = String::from_utf8(lscpu_cmd.stdout).unwrap();

                widget::text::body(lscpu)
                    .apply(cosmic::iced::widget::scrollable)
                    .into()
            }
            None => widget::text::title1(fl!("no-page")).into(),
        };

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            cosmic::iced::subscription::channel(
                std::any::TypeId::of::<MySubscription>(),
                4,
                move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                },
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Commands may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::OpenRepositoryUrl => {
                _ = open::that_detached(REPOSITORY);
            }

            Message::SubscriptionChannel => {
                // For example purposes only.
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }

                // Set the title of the context drawer.
                self.set_context_title(context_page.title());
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }
        }
        Command::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Command<Self::Message> {
        // Activate the page in the model.
        self.nav.activate(id);

        self.update_title()
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

        let title = widget::text::title3(fl!("app-title"));

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .align_items(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Command<Message> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        self.set_window_title(window_title)
    }
}

/// The page to display in the application.
pub enum Page {
    DistributionPage,
    ProcessorPage,
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

impl ContextPage {
    fn title(&self) -> String {
        match self {
            Self::About => fl!("about"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}
