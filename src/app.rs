// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use cosmic::app::{Command, Core};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, icon, list_column, menu, nav_bar, row, settings};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use etc_os_release::OsRelease;
use futures_util::SinkExt;
use itertools::Itertools;
use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};

const REPOSITORY: &str = "https://github.com/cosmic-utils/examine";
const APP_ICON: &[u8] =
    include_bytes!("../res/icons/hicolor/scalable/apps/page.codeberg.sungsphinx.Examine.svg");

pub struct AppModel {
    core: Core,
    context_page: ContextPage,
    nav: nav_bar::Model,
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    config: Config,
    lscpu: Option<String>,
    lspci: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
}

impl Application for AppModel {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "page.codeberg.sungsphinx.Examine";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("distribution"))
            .data::<Page>(Page::Distribution)
            .icon(icon::from_name("applications-system-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("processor"))
            .data::<Page>(Page::Processor)
            .icon(icon::from_name("system-run-symbolic"));

        nav.insert()
            .text(fl!("pci-devices"))
            .data::<Page>(Page::PCI)
            .icon(icon::from_name("drive-harddisk-usb-symbolic"));

        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            lscpu: None,
            lspci: None,
        };

        let lscpu_cmd = std::process::Command::new("lscpu").output().unwrap();
        match String::from_utf8(lscpu_cmd.stdout) {
            Ok(lscpu) => app.lscpu = Some(lscpu),
            Err(err) => {
                eprintln!("Error parsing lscpu: {}", err);
            }
        }

        let lspci_cmd = std::process::Command::new("lspci").output().unwrap();
        match String::from_utf8(lspci_cmd.stdout) {
            Ok(lspci) => app.lspci = Some(lspci),
            Err(err) => {
                eprintln!("Error parsing lspci: {}", err);
            }
        }

        let command = app.update_title();

        (app, command)
    }

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

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn context_drawer(&self) -> Option<Element<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => self.about(),
        })
    }

    fn view(&self) -> Element<Self::Message> {
        let page = self.nav.data::<Page>(self.nav.active());
        let is_flatpak = PathBuf::from("/.flatpak-info").exists();
        let spacing = theme::active().cosmic().spacing;

        let content: Element<Self::Message> = match page {
            Some(Page::Distribution) => {
                let osrelease = if is_flatpak {
                    OsRelease::from_str(&fs::read_to_string("/run/host/os-release").unwrap())
                        .unwrap()
                } else {
                    OsRelease::open().unwrap()
                };

                let mut list = list_column();

                list = list.add(settings::item(
                    fl!("pretty-name"),
                    widget::text::body(osrelease.pretty_name().to_string()),
                ));
                list = list.add(settings::item(
                    fl!("name"),
                    widget::text::body(osrelease.name().to_string()),
                ));
                if let Some(version) = osrelease.version() {
                    list = list.add(settings::item(
                        fl!("version"),
                        widget::text::body(version.to_string()),
                    ));
                }
                if let Some(version_id) = osrelease.version_id() {
                    list = list.add(settings::item(
                        fl!("version-id"),
                        widget::text::body(version_id.to_string()),
                    ));
                }
                list = list.add(settings::item(
                    fl!("id"),
                    widget::text::body(osrelease.id().to_string()),
                ));
                if let Some(mut id_like) = osrelease.id_like() {
                    list = list.add(settings::item(
                        fl!("id-like"),
                        widget::text::body(id_like.join(", ")),
                    ));
                }
                if let Some(version_codename) = osrelease.version_codename() {
                    // Fedora (and possibly other distros) set VERSION_CODENAME to a blank string, so check if it is empty
                    if !version_codename.to_string().is_empty() {
                        list = list.add(settings::item(
                            fl!("version-codename"),
                            widget::text::body(version_codename.to_string()),
                        ));
                    }
                }
                if let Some(build_id) = osrelease.build_id() {
                    list = list.add(settings::item(
                        fl!("build-id"),
                        widget::text::body(build_id.to_string()),
                    ));
                }
                if let Some(image_id) = osrelease.image_id() {
                    list = list.add(settings::item(
                        fl!("image-id"),
                        widget::text::body(image_id.to_string()),
                    ));
                }
                if let Some(image_version) = osrelease.image_version() {
                    list = list.add(settings::item(
                        fl!("image-version"),
                        widget::text::body(image_version.to_string()),
                    ));
                }
                if let Some(vendor_name) = osrelease.vendor_name() {
                    list = list.add(settings::item(
                        fl!("vendor-name"),
                        widget::text::body(vendor_name.to_string()),
                    ));
                }
                if let Some(ansi_color) = osrelease.ansi_color() {
                    list = list.add(settings::item(
                        fl!("ansi-color"),
                        widget::text::body(ansi_color.to_string()),
                    ));
                }
                if let Some(logo) = osrelease.logo() {
                    list = list.add(settings::item(
                        fl!("logo"),
                        row::with_capacity(2)
                            .push(icon::from_name(logo.to_string()))
                            .push(widget::text::body(logo.to_string()))
                            .align_items(Alignment::Center)
                            .spacing(spacing.space_xxxs),
                    ));
                }
                if let Some(cpe_name) = osrelease.cpe_name() {
                    list = list.add(settings::item(
                        fl!("cpe-name"),
                        widget::text::body(cpe_name.to_string()),
                    ));
                }
                if let Ok(Some(home_url)) = osrelease.home_url() {
                    list = list.add(settings::item(
                        fl!("home-url"),
                        widget::text::body(home_url.to_string()),
                    ));
                }
                if let Ok(Some(support_url)) = osrelease.support_url() {
                    list = list.add(settings::item(
                        fl!("vendor-url"),
                        widget::text::body(support_url.to_string()),
                    ));
                }
                if let Ok(Some(documentation_url)) = osrelease.documentation_url() {
                    list = list.add(settings::item(
                        fl!("doc-url"),
                        widget::text::body(documentation_url.to_string()),
                    ));
                }
                if let Ok(Some(support_url)) = osrelease.support_url() {
                    list = list.add(settings::item(
                        fl!("support-url"),
                        widget::text::body(support_url.to_string()),
                    ));
                }
                if let Ok(Some(bug_report_url)) = osrelease.bug_report_url() {
                    list = list.add(settings::item(
                        fl!("bug-report-url"),
                        widget::text::body(bug_report_url.to_string()),
                    ));
                }
                if let Ok(Some(privacy_policy_url)) = osrelease.privacy_policy_url() {
                    list = list.add(settings::item(
                        fl!("privacy-policy-url"),
                        widget::text::body(privacy_policy_url.to_string()),
                    ));
                }
                if let Some(support_end) = osrelease.support_end().unwrap_or_default().take() {
                    list = list.add(settings::item(
                        fl!("support-end"),
                        widget::text::body(support_end.to_string()),
                    ));
                }
                if let Some(variant) = osrelease.variant() {
                    list = list.add(settings::item(
                        fl!("variant"),
                        widget::text::body(variant.to_string()),
                    ));
                }
                if let Some(variant_id) = osrelease.variant_id() {
                    list = list.add(settings::item(
                        fl!("variant-id"),
                        widget::text::body(variant_id.to_string()),
                    ));
                }
                if let Some(default_hostname) = osrelease.default_hostname() {
                    list = list.add(settings::item(
                        fl!("default-hostname"),
                        widget::text::body(default_hostname.to_string()),
                    ));
                }
                if let Some(architecture) = osrelease.architecture() {
                    list = list.add(settings::item(
                        fl!("arch"),
                        widget::text::body(architecture.to_string()),
                    ));
                }
                if let Some(sysext_level) = osrelease.sysext_level() {
                    list = list.add(settings::item(
                        "SYSEXT_LEVEL",
                        widget::text::body(sysext_level.to_string()),
                    ));
                }
                if let Some(mut sysext_scope) = osrelease.sysext_scope() {
                    list = list.add(settings::item(
                        "SYSEXT_SCOPE",
                        widget::text::body(sysext_scope.join(", ")),
                    ));
                }
                if let Some(confext_level) = osrelease.confext_level() {
                    list = list.add(settings::item(
                        "CONFEXT_LEVEL",
                        widget::text::body(confext_level.to_string()),
                    ));
                }
                if let Some(mut confext_scope) = osrelease.confext_scope() {
                    list = list.add(settings::item(
                        "CONFEXT_SCOPE",
                        widget::text::body(confext_scope.join(", ")),
                    ));
                }
                if let Some(mut portable_prefixes) = osrelease.portable_prefixes() {
                    list = list.add(settings::item(
                        fl!("portable-prefixes"),
                        widget::text::body(portable_prefixes.join(", ")),
                    ));
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
            Some(Page::Processor) => {
                let Some(lscpu) = &self.lscpu else {
                    return widget::text::title1(fl!("something-went-wrong")).into();
                };
                let lscpu = lscpu
                    .lines()
                    .map(|line: &str| {
                        let (prefix, suffix) = line.split_once(":").unwrap();
                        widget::settings::item(prefix, widget::text::body(suffix)).into()
                    })
                    .collect::<Vec<Element<Message>>>();

                let mut section = widget::settings::view_section("");
                for item in lscpu {
                    section = section.add(item);
                }
                section.apply(widget::scrollable).into()
            }
            Some(Page::PCI) => {
                let Some(lspci) = &self.lspci else {
                    return widget::text::title1(fl!("something-went-wrong")).into();
                };
                let lspci = lspci
                    .lines()
                    .map(|line: &str| {
                        let (prefix, suffix) = line.split_once(": ").unwrap();
                        widget::settings::item(suffix, widget::text::body(prefix)).into()
                    })
                    .collect::<Vec<Element<Message>>>();

                let mut section = widget::settings::view_section("");
                for item in lspci {
                    section = section.add(item);
                }
                section.apply(widget::scrollable).into()
            }
            None => widget::text::title1(fl!("no-page")).into(),
        };

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            cosmic::iced::subscription::channel(
                std::any::TypeId::of::<MySubscription>(),
                4,
                move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                },
            ),
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

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
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }

                self.set_context_title(context_page.title());
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }
        }
        Command::none()
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Command<Self::Message> {
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
    Distribution,
    Processor,
    PCI,
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
