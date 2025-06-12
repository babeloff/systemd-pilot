use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use std::rc::Rc;

mod app;
mod remote_host;
mod service_manager;
mod ui;
mod utils;

use app::SystemdPilotApp;

const APP_ID: &str = "io.github.mfat.systemdpilot";
const APP_NAME: &str = "systemd Pilot";
const APP_VERSION: &str = "3.0.0";

fn main() -> glib::ExitCode {
    // Initialize logger
    env_logger::init();
    log::info!("Starting {} v{}", APP_NAME, APP_VERSION);

    // Create GTK application
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    // Create main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title(&format!("{} v{}", APP_NAME, APP_VERSION))
        .default_width(1000)
        .default_height(600)
        .build();

    // Create the main application
    let systemd_app = Rc::new(SystemdPilotApp::new(&window));

    // Setup UI
    systemd_app.setup_ui();

    // Load saved configuration
    systemd_app.load_saved_hosts();

    // Show the window
    window.present();
}
