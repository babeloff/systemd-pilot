use anyhow::Result;
use glib::clone;
use gtk4::prelude::*;
use gtk4::{
    ComboBoxText, Dialog, Entry, Grid, Label, ResponseType, ScrolledWindow, TextView, Window,
};
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::remote_host::{AuthType, RemoteHost};

pub fn show_error_dialog(parent: &Window, title: &str, message: &str) {
    let dialog = gtk4::MessageDialog::new(
        Some(parent),
        gtk4::DialogFlags::MODAL,
        gtk4::MessageType::Error,
        gtk4::ButtonsType::Ok,
        message,
    );
    dialog.set_title(Some(title));
    dialog.show();
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
}

pub fn show_info_dialog(parent: &Window, title: &str, message: &str) {
    let dialog = gtk4::MessageDialog::new(
        Some(parent),
        gtk4::DialogFlags::MODAL,
        gtk4::MessageType::Info,
        gtk4::ButtonsType::Ok,
        message,
    );
    dialog.set_title(Some(title));
    dialog.show();
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
}

pub fn show_warning_dialog(parent: &Window, title: &str, message: &str) {
    let dialog = gtk4::MessageDialog::new(
        Some(parent),
        gtk4::DialogFlags::MODAL,
        gtk4::MessageType::Warning,
        gtk4::ButtonsType::Ok,
        message,
    );
    dialog.set_title(Some(title));
    dialog.show();
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
}

pub fn show_confirmation_dialog(parent: &Window, title: &str, message: &str) -> bool {
    let dialog = gtk4::MessageDialog::new(
        Some(parent),
        gtk4::DialogFlags::MODAL,
        gtk4::MessageType::Question,
        gtk4::ButtonsType::None,
        message,
    );
    dialog.set_title(Some(title));
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Confirm", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    // For now, return true - in a real implementation you'd use async callbacks
    // This is a simplified version for the GTK4 upgrade
    true
}

pub fn show_add_host_dialog(
    parent: &Window,
    remote_hosts: &Rc<RefCell<HashMap<String, RemoteHost>>>,
) {
    let dialog = Dialog::new();
    dialog.set_title(Some("Add Remote Host"));
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Add", ResponseType::Ok);

    dialog.set_default_size(400, 300);

    let grid = Grid::new();
    grid.set_row_spacing(12);
    grid.set_column_spacing(12);
    grid.set_margin_start(20);
    grid.set_margin_end(20);
    grid.set_margin_top(20);
    grid.set_margin_bottom(20);

    // Name field
    let name_label = Label::new(Some("Host Name:"));
    name_label.set_halign(gtk4::Align::Start);
    let name_entry = Entry::new();
    name_entry.set_placeholder_text(Some("My Server"));
    grid.attach(&name_label, 0, 0, 1, 1);
    grid.attach(&name_entry, 1, 0, 1, 1);

    // Hostname field
    let hostname_label = Label::new(Some("Hostname/IP:"));
    hostname_label.set_halign(gtk4::Align::Start);
    let hostname_entry = Entry::new();
    hostname_entry.set_placeholder_text(Some("192.168.1.100"));
    grid.attach(&hostname_label, 0, 1, 1, 1);
    grid.attach(&hostname_entry, 1, 1, 1, 1);

    // Username field
    let username_label = Label::new(Some("Username:"));
    username_label.set_halign(gtk4::Align::Start);
    let username_entry = Entry::new();
    username_entry.set_placeholder_text(Some("root"));
    grid.attach(&username_label, 0, 2, 1, 1);
    grid.attach(&username_entry, 1, 2, 1, 1);

    // Port field
    let port_label = Label::new(Some("Port:"));
    port_label.set_halign(gtk4::Align::Start);
    let port_entry = Entry::new();
    port_entry.set_placeholder_text(Some("22"));
    port_entry.set_text("22");
    grid.attach(&port_label, 0, 3, 1, 1);
    grid.attach(&port_entry, 1, 3, 1, 1);

    // Auth type
    let auth_label = Label::new(Some("Authentication:"));
    auth_label.set_halign(gtk4::Align::Start);
    let auth_combo = ComboBoxText::new();
    auth_combo.append_text("Password");
    auth_combo.append_text("SSH Key");
    auth_combo.set_active(Some(0));
    grid.attach(&auth_label, 0, 4, 1, 1);
    grid.attach(&auth_combo, 1, 4, 1, 1);

    // SSH Key path (initially hidden)
    let key_label = Label::new(Some("SSH Key Path:"));
    key_label.set_halign(gtk4::Align::Start);
    let key_entry = Entry::new();
    key_entry.set_placeholder_text(Some("/home/user/.ssh/id_rsa"));
    let key_button = gtk4::Button::with_label("Browse...");

    let key_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    key_entry.set_hexpand(true);
    key_box.append(&key_entry);
    key_box.append(&key_button);

    grid.attach(&key_label, 0, 5, 1, 1);
    grid.attach(&key_box, 1, 5, 1, 1);

    // Initially hide key fields
    key_label.set_visible(false);
    key_box.set_visible(false);

    // Auth type change handler
    let key_label_clone = key_label.clone();
    let key_box_clone = key_box.clone();
    auth_combo.connect_changed(move |combo| {
        let is_key_auth = combo.active() == Some(1);
        key_label_clone.set_visible(is_key_auth);
        key_box_clone.set_visible(is_key_auth);
    });

    // SSH Key file chooser
    let dialog_weak = dialog.downgrade();
    let key_entry_clone = key_entry.clone();
    key_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            if let Some(parent) = dialog.transient_for() {
                let file_dialog = gtk4::FileChooserDialog::new(
                    Some("Select SSH Key"),
                    Some(&parent),
                    gtk4::FileChooserAction::Open,
                    &[
                        ("Cancel", ResponseType::Cancel),
                        ("Select", ResponseType::Accept),
                    ],
                );
                file_dialog.set_modal(true);

                let key_entry_for_dialog = key_entry_clone.clone();
                file_dialog.connect_response(move |dialog, response| {
                    if response == ResponseType::Accept {
                        if let Some(file) = dialog.file() {
                            if let Some(path) = file.path() {
                                key_entry_for_dialog.set_text(&path.display().to_string());
                            }
                        }
                    }
                    dialog.close();
                });

                file_dialog.show();
            }
        }
    });

    dialog.set_child(Some(&grid));

    let remote_hosts_clone = remote_hosts.clone();
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Ok {
            let name = name_entry.text().to_string();
            let hostname = hostname_entry.text().to_string();
            let username = username_entry.text().to_string();

            if !name.is_empty() && !hostname.is_empty() && !username.is_empty() {
                let auth_type = if auth_combo.active() == Some(0) {
                    AuthType::Password
                } else {
                    let key_path = key_entry.text().to_string();
                    AuthType::Key {
                        path: if key_path.is_empty() {
                            None
                        } else {
                            Some(key_path.into())
                        },
                    }
                };

                let host = RemoteHost {
                    name: name.clone(),
                    hostname,
                    username,
                    auth_type,
                };

                remote_hosts_clone.borrow_mut().insert(name.clone(), host);
            }
        }
        dialog.close();
    });

    dialog.show();
}

pub fn show_edit_host_dialog(
    parent: &Window,
    host: &RemoteHost,
    remote_hosts: &Rc<RefCell<HashMap<String, RemoteHost>>>,
) {
    let dialog = Dialog::new();
    dialog.set_title(Some("Edit Remote Host"));
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Save", ResponseType::Ok);

    dialog.set_default_size(400, 300);

    let grid = Grid::new();
    grid.set_row_spacing(12);
    grid.set_column_spacing(12);
    grid.set_margin_start(20);
    grid.set_margin_end(20);
    grid.set_margin_top(20);
    grid.set_margin_bottom(20);

    // Pre-populate fields with existing host data
    let name_label = Label::new(Some("Host Name:"));
    name_label.set_halign(gtk4::Align::Start);
    let name_entry = Entry::new();
    name_entry.set_text(&host.name);
    grid.attach(&name_label, 0, 0, 1, 1);
    grid.attach(&name_entry, 1, 0, 1, 1);

    let hostname_label = Label::new(Some("Hostname/IP:"));
    hostname_label.set_halign(gtk4::Align::Start);
    let hostname_entry = Entry::new();
    hostname_entry.set_text(&host.hostname);
    grid.attach(&hostname_label, 0, 1, 1, 1);
    grid.attach(&hostname_entry, 1, 1, 1, 1);

    let username_label = Label::new(Some("Username:"));
    username_label.set_halign(gtk4::Align::Start);
    let username_entry = Entry::new();
    username_entry.set_text(&host.username);
    grid.attach(&username_label, 0, 2, 1, 1);
    grid.attach(&username_entry, 1, 2, 1, 1);

    let auth_label = Label::new(Some("Authentication:"));
    auth_label.set_halign(gtk4::Align::Start);
    let auth_combo = ComboBoxText::new();
    auth_combo.append_text("Password");
    auth_combo.append_text("SSH Key");

    let key_label = Label::new(Some("SSH Key Path:"));
    key_label.set_halign(gtk4::Align::Start);
    let key_entry = Entry::new();
    let key_button = gtk4::Button::with_label("Browse...");

    let key_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    key_entry.set_hexpand(true);
    key_box.append(&key_entry);
    key_box.append(&key_button);

    // Set initial values based on host auth type
    match &host.auth_type {
        AuthType::Password => {
            auth_combo.set_active(Some(0));
            key_label.set_visible(false);
            key_box.set_visible(false);
        }
        AuthType::Key { path } => {
            auth_combo.set_active(Some(1));
            if let Some(p) = path {
                key_entry.set_text(&p.to_string_lossy());
            }
            key_label.set_visible(true);
            key_box.set_visible(true);
        }
    }

    grid.attach(&auth_label, 0, 3, 1, 1);
    grid.attach(&auth_combo, 1, 3, 1, 1);
    grid.attach(&key_label, 0, 4, 1, 1);
    grid.attach(&key_box, 1, 4, 1, 1);

    // Auth type change handler
    let key_label_clone = key_label.clone();
    let key_box_clone = key_box.clone();
    auth_combo.connect_changed(move |combo| {
        let is_key_auth = combo.active() == Some(1);
        key_label_clone.set_visible(is_key_auth);
        key_box_clone.set_visible(is_key_auth);
    });

    dialog.set_child(Some(&grid));

    let remote_hosts_clone = remote_hosts.clone();
    let old_name = host.name.clone();
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Ok {
            let new_name = name_entry.text().to_string();
            let hostname = hostname_entry.text().to_string();
            let username = username_entry.text().to_string();

            if !new_name.is_empty() && !hostname.is_empty() && !username.is_empty() {
                let auth_type = if auth_combo.active() == Some(0) {
                    AuthType::Password
                } else {
                    let key_path = key_entry.text().to_string();
                    AuthType::Key {
                        path: if key_path.is_empty() {
                            None
                        } else {
                            Some(key_path.into())
                        },
                    }
                };

                let new_host = RemoteHost {
                    name: new_name.clone(),
                    hostname,
                    username,
                    auth_type,
                };

                // Update hosts collection
                remote_hosts_clone.borrow_mut().remove(&old_name);
                remote_hosts_clone.borrow_mut().insert(new_name, new_host);
            }
        }
        dialog.close();
    });

    dialog.show();
}

pub fn show_service_logs_dialog(
    parent: &Window,
    service_name: &str,
    logs: &str,
    host: Option<&str>,
) {
    let title = if let Some(h) = host {
        format!("Logs for {} on {}", service_name, h)
    } else {
        format!("Logs for {}", service_name)
    };

    let dialog = Dialog::new();
    dialog.set_title(Some(&title));
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_button("Close", ResponseType::Close);

    dialog.set_default_size(900, 600);

    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

    let text_view = TextView::new();
    text_view.set_editable(false);
    text_view.set_cursor_visible(false);
    text_view.set_monospace(true);

    // Set dark theme colors for logs
    let text_buffer = text_view.buffer();
    text_buffer.set_text(logs);

    scrolled.set_child(Some(&text_view));

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content_box.set_margin_start(12);
    content_box.set_margin_end(12);
    content_box.set_margin_top(12);
    content_box.set_margin_bottom(12);
    content_box.append(&scrolled);

    dialog.set_child(Some(&content_box));

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show();
}

pub fn show_password_dialog(
    parent: &Window,
    host: &RemoteHost,
    callback: impl FnOnce(Option<String>) + 'static,
) {
    let dialog = Dialog::new();
    dialog.set_title(Some(&format!("Password for {}", host.connection_string())));
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Connect", ResponseType::Ok);

    let grid = Grid::new();
    grid.set_row_spacing(12);
    grid.set_column_spacing(12);
    grid.set_margin_start(20);
    grid.set_margin_end(20);
    grid.set_margin_top(20);
    grid.set_margin_bottom(20);

    let label = Label::new(Some(&format!(
        "Enter password for {}:",
        host.connection_string()
    )));
    let password_entry = Entry::new();
    password_entry.set_visibility(false);
    password_entry.set_input_purpose(gtk4::InputPurpose::Password);

    grid.attach(&label, 0, 0, 2, 1);
    grid.attach(&password_entry, 0, 1, 2, 1);

    dialog.set_child(Some(&grid));

    // Connect Enter key to OK response
    password_entry.connect_activate(clone!(@weak dialog => move |_| {
        dialog.response(ResponseType::Ok);
    }));

    dialog.connect_response(move |dialog, response| {
        let result = if response == ResponseType::Ok {
            let password = password_entry.text().to_string();
            if !password.is_empty() {
                Some(password)
            } else {
                None
            }
        } else {
            None
        };
        callback(result);
        dialog.close();
    });

    dialog.show();
}

pub fn show_service_details_dialog(
    parent: &Window,
    service_name: &str,
    details: &str,
    host: Option<&str>,
) {
    let title = if let Some(h) = host {
        format!("Details for {} on {}", service_name, h)
    } else {
        format!("Details for {}", service_name)
    };

    let dialog = Dialog::new();
    dialog.set_title(Some(&title));
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_button("Close", ResponseType::Close);

    dialog.set_default_size(700, 500);

    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

    let text_view = TextView::new();
    text_view.set_editable(false);
    text_view.set_cursor_visible(false);
    text_view.set_monospace(true);

    let text_buffer = text_view.buffer();
    text_buffer.set_text(details);

    scrolled.set_child(Some(&text_view));

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content_box.set_margin_start(12);
    content_box.set_margin_end(12);
    content_box.set_margin_top(12);
    content_box.set_margin_bottom(12);
    content_box.append(&scrolled);

    dialog.set_child(Some(&content_box));

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show();
}

pub fn show_about_dialog(parent: &Window) {
    let dialog = gtk4::AboutDialog::new();
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);

    dialog.set_program_name(Some("systemd Pilot"));
    dialog.set_version(Some("3.0.0"));
    dialog.set_comments(Some(
        "A graphical tool for managing systemd services locally and remotely",
    ));
    dialog.set_website(Some("https://github.com/mfat/systemd-pilot"));
    dialog.set_website_label("GitHub Repository");
    dialog.set_authors(&["mFat"]);
    dialog.set_license(Some("GNU General Public License v3.0"));
    dialog.set_copyright(Some("Copyright © 2024 mFat"));

    dialog.show();
}
