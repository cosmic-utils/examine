// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use cosmic::app::{Command, Core};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::iced_winit::winit::window::WindowId;
use cosmic::widget::{self, icon, list_column, menu, nav_bar, row, settings};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use etc_os_release::OsRelease;
use futures_util::SinkExt;
use itertools::Itertools;
use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};
use log::{error, warn};

const REPOSITORY: &str = "https://github.com/cosmic-utils/examine";
const APP_ICON: &[u8] =
    include_bytes!("../res/icons/hicolor/scalable/apps/io.github.cosmic_utils.Examine.svg");

pub struct AppModel {
    core: Core,
    context_page: ContextPage,
    nav: nav_bar::Model,
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    config: Config,
    dmidecode: Option<String>,
    lscpu: Option<String>,
    lspci: Option<String>,
    lsusb: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LaunchUrl(String),
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
}

impl Application for AppModel {
    type Executor = cosmic::executor::Default;

    type Flags = ();

    type Message = Message;

    const APP_ID: &'static str = "io.github.cosmic_utils.Examine";

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
            .text(fl!("motherboard"))
            .data::<Page>(Page::Motherboard)
            .icon(icon::from_name("applications-system-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("processor"))
            .data::<Page>(Page::Processor)
            .icon(icon::from_name("system-run-symbolic"));

        nav.insert()
            .text(fl!("pci-devices"))
            .data::<Page>(Page::PCIs)
            .icon(icon::from_name("drive-harddisk-usb-symbolic"));

        nav.insert()
            .text(fl!("usb-devices"))
            .data::<Page>(Page::USBs)
            .icon(icon::from_name("media-removable-symbolic"));

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
            dmidecode: None,
            lscpu: None,
            lspci: None,
            lsusb: None,
        };

        let dmidecode_cmd = std::process::Command::new("dmidecode -t baseboard").output();
        if dmidecode_cmd.is_ok() {
            app.dmidecode = Some(String::from_utf8(dmidecode_cmd.unwrap().stdout).unwrap());
        } else if let Err(e) = dmidecode_cmd {
            app.dmidecode = Some(fl!("error-occurred-with-msg", error = e.to_string()));
            error!("dmidecode command failed: {}", e);
        }

        let lscpu_cmd = std::process::Command::new("lscpu").output();
        if lscpu_cmd.is_ok() {
            app.lscpu = Some(String::from_utf8(lscpu_cmd.unwrap().stdout).unwrap());
        } else if let Err(e) = lscpu_cmd {
            app.lscpu = Some(fl!("error-occurred-with-msg", error = e.to_string()));
            error!("lscpu command failed: {}", e);
        }

        let lspci_cmd = std::process::Command::new("lspci").output();
        if lspci_cmd.is_ok() {
            app.lspci = Some(String::from_utf8(lspci_cmd.unwrap().stdout).unwrap());
        } else if let Err(e) = lspci_cmd {
            app.lspci = Some(fl!("error-occurred-with-msg", error = e.to_string()));
            error!("lspci command failed: {}", e);
        }

        let lsusb_cmd = std::process::Command::new("lsusb").output();
        if lsusb_cmd.is_ok() {
            app.lsusb = Some(String::from_utf8(lsusb_cmd.unwrap().stdout).unwrap());
        } else if let Err(e) = lsusb_cmd {
            app.lsusb = Some(fl!("error-occurred-with-msg", error = e.to_string()));
            error!("lsusb command failed: {}", e);
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
            Some(Page::Motherboard) => {
                let Some(dmidecode) = &self.dmidecode else {
                    return widget::text::title1(fl!("error-occurred")).into();
                };

                if let Some(dmidecode_str) = &self.dmidecode {
                    if dmidecode_str.starts_with(fl!("error-occurred").as_str()) {
                        return widget::text::title1(dmidecode_str).into();
                    } else {
                        let dmidecode = dmidecode
                            .lines()
                            .map(|line: &str| {
                                let (prefix, suffix) = line.split_once(':').unwrap();
                                settings::item(prefix, widget::text::body(suffix)).into()
                            })
                            .collect::<Vec<Element<Message>>>();

                        let mut section = list_column();
                        for item in dmidecode {
                            section = section.add(item);
                        }
                        return section.apply(widget::scrollable).into()
                    }
                } else {
                    return widget::text::title1(fl!("error-occurred")).into();
                }
            }
            Some(Page::Processor) => {
                let Some(lscpu) = &self.lscpu else {
                    return widget::text::title1(fl!("error-occurred")).into();
                };

                if let Some(lscpu_str) = &self.lscpu {
                    if lscpu_str.starts_with(fl!("error-occurred").as_str()) {
                        return widget::text::title1(lscpu_str).into();
                    } else {
                        let lscpu = lscpu
                            .lines()
                            .map(|line: &str| {
                                let (prefix, suffix) = line.split_once(':').unwrap();
                                settings::item(prefix, widget::text::body(suffix)).into()
                            })
                            .collect::<Vec<Element<Message>>>();

                        let mut section = list_column();
                        for item in lscpu {
                            section = section.add(item);
                        }
                        return section.apply(widget::scrollable).into()
                    }
                } else {
                    return widget::text::title1(fl!("error-occurred")).into();
                }
            }
            Some(Page::PCIs) => {
                let Some(lspci) = &self.lspci else {
                    return widget::text::title1(fl!("error-occurred")).into();
                };

                if let Some(lspci_str) = &self.lspci {
                    if lspci_str.starts_with(fl!("error-occurred").as_str()) {
                        return widget::text::title1(lspci_str).into();
                    } else {
                        let lspci = lspci
                            .lines()
                            .map(|line: &str| {
                                let (prefix, suffix) = line.split_once(": ").unwrap();
                                settings::item(suffix, widget::text::body(prefix)).into()
                            })
                            .collect::<Vec<Element<Message>>>();

                        let mut section = list_column();
                        for item in lspci {
                            section = section.add(item);
                        }
                        return section.apply(widget::scrollable).into()
                    }
                } else {
                    return widget::text::title1(fl!("error-occurred")).into();
                }
            }
            Some(Page::USBs) => {
                let Some(lsusb) = &self.lsusb else {
                    return widget::text::title1(fl!("error-occurred")).into();
                };

                if let Some(lsusb_str) = &self.lsusb {
                    if lsusb_str.starts_with(fl!("error-occurred").as_str()) {
                        return widget::text::title1(lsusb_str).into();
                    } else {
                        let lsusb = lsusb
                            .lines()
                            .map(|line: &str| {
                                let (prefix, suffix) = line.split_once(": ").unwrap();
                                settings::item(suffix, widget::text::body(prefix)).into()
                            })
                            .collect::<Vec<Element<Message>>>();

                        let mut section = list_column();
                        for item in lsusb {
                            section = section.add(item);
                        }
                        return section.apply(widget::scrollable).into()
                    }
                } else {
                    return widget::text::title1(fl!("error-occurred")).into();
                }
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
            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    warn!("failed to open {:?}: {}", url, err);
                }
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
        let hash = env!("VERGEN_GIT_SHA");
        let short_hash: String = hash.chars().take(7).collect();
        let date = env!("VERGEN_GIT_COMMIT_DATE");

        let repo = widget::button::link(REPOSITORY)
            .on_press(Message::LaunchUrl(REPOSITORY.to_string()))
            .padding(0);

        let commit = widget::button::link(fl!(
                "git-description",
                hash = short_hash.as_str(),
                date = date
            ))
            .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(repo)
            .push(commit)
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
    Motherboard,
    Processor,
    PCIs,
    USBs,
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
