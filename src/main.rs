// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod config;
mod i18n;

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,examine=info,warn")).init();
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);
    let settings = cosmic::app::Settings::default();
    cosmic::app::run::<app::AppModel>(settings, ())
}
