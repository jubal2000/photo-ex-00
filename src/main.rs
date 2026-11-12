mod api;
mod app;
mod components;
mod config;
mod firebase;
mod i18n;
mod models;
mod platform;
mod screens;
mod state;

#[cfg(target_os = "ios")]
unsafe extern "C" {
    fn init_ios_file_picker();
}

fn main() {
    #[cfg(target_os = "ios")]
    unsafe {
        init_ios_file_picker();
    }
    dioxus::launch(app::App);
}
