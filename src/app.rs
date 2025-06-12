use anyhow::{anyhow, Result};
use glib::{clone, MainContext, PRIORITY_DEFAULT};
use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Box, Button, CellRendererText, CheckButton, ComboBoxText, Dialog, Entry,
    Grid, Label, ListBox, ListBoxRow, Notebook, Paned, ResponseType, ScrolledWindow, TextView,
    TreeIter, TreePath, TreeSelection, TreeStore, TreeView, TreeViewColumn, Window,
};
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::remote_host::{AuthType, RemoteHost};
use crate::service_manager::{ServiceInfo, ServiceManager, ServiceStatus};
use crate::ui::dialogs::*;
use crate::utils::theme::ThemeManager;

pub struct SystemdPilotApp {
    window: ApplicationWindow,
    notebook: Notebook,
    remote_hosts: Rc<RefCell<HashMap<String, RemoteHost>>>,
    active_connections: Arc<Mutex<HashMap<String, ssh2::Session>>>,
    service_manager: Rc<ServiceManager>,
    theme_manager: Rc<ThemeManager>,
    runtime: Arc<Runtime>,

    // UI Components
    local_services_list: TreeView,
    remote_services_list: TreeView,
    hosts_listbox: ListBox,
    show_inactive_button: CheckButton,

    // Tree stores
    local_services_store: TreeStore,
    remote_services_store: TreeStore,
}

impl SystemdPilotApp {
    pub fn new(window: &ApplicationWindow) -> Self {
        let runtime = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));

        let theme_manager = Rc::new(ThemeManager::new());
        let service_manager = Rc::new(ServiceManager::new(runtime.clone()));

        // Create tree stores
        let local_services_store = TreeStore::new(&[
            glib::Type::STRING, // Service name
            glib::Type::STRING, // Status
            glib::Type::STRING, // Description
        ]);

        let remote_services_store = TreeStore::new(&[
            glib::Type::STRING, // Host
            glib::Type::STRING, // Service name
            glib::Type::STRING, // Status
            glib::Type::STRING, // Description
        ]);

        Self {
            window: window.clone(),
            notebook: Notebook::new(),
            remote_hosts: Rc::new(RefCell::new(HashMap::new())),
            active_connections: Arc::new(Mutex::new(HashMap::new())),
            service_manager,
            theme_manager,
            runtime,
            local_services_list: TreeView::new(),
            remote_services_list: TreeView::new(),
            hosts_listbox: ListBox::new(),
            show_inactive_button: CheckButton::with_label("Show inactive services"),
            local_services_store,
            remote_services_store,
        }
    }

    pub fn setup_ui(&self) {
        let main_box = Box::new(gtk4::Orientation::Vertical, 0);

        // Setup header bar
        self.setup_header_bar();

        // Setup notebook with tabs
        self.setup_notebook();

        main_box.append(&self.notebook);

        self.window.set_child(Some(&main_box));

        // Apply theme
        self.theme_manager.apply_theme(&self.window);

        // Setup signal handlers
        self.setup_signal_handlers();
    }

    fn setup_header_bar(&self) {
        let header_bar = gtk4::HeaderBar::new();
        header_bar.set_title(Some("systemd Pilot"));
        header_bar.set_show_title_buttons(true);

        // Add theme toggle button
        let theme_button = Button::with_label("🌙");
        theme_button.set_tooltip_text(Some("Toggle dark/light theme"));

        let theme_manager = self.theme_manager.clone();
        let window = self.window.clone();
        theme_button.connect_clicked(move |_| {
            theme_manager.toggle_theme();
            theme_manager.apply_theme(&window);
        });

        header_bar.pack_end(&theme_button);

        // Add refresh button
        let refresh_button = Button::with_label("🔄");
        refresh_button.set_tooltip_text(Some("Refresh services"));

        let app_weak = Rc::downgrade(&Rc::new(RefCell::new(self)));
        refresh_button.connect_clicked(move |_| {
            if let Some(app) = app_weak.upgrade() {
                app.borrow().refresh_all_services();
            }
        });

        header_bar.pack_start(&refresh_button);

        self.window.set_titlebar(Some(&header_bar));
    }

    fn setup_notebook(&self) {
        // Local services tab
        let local_page = self.create_local_page();
        self.notebook
            .append_page(&local_page, Some(&Label::new(Some("Local"))));

        // Remote services tab
        let remote_page = self.create_remote_page();
        self.notebook
            .append_page(&remote_page, Some(&Label::new(Some("Remote"))));

        self.notebook.set_tab_pos(gtk4::PositionType::Top);
        self.notebook.set_scrollable(true);
    }

    fn create_local_page(&self) -> Box {
        let main_box = Box::new(gtk4::Orientation::Vertical, 6);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // Control buttons
        let button_box = Box::new(gtk4::Orientation::Horizontal, 6);

        let start_button = Button::with_label("▶ Start");
        let stop_button = Button::with_label("⏹ Stop");
        let restart_button = Button::with_label("🔄 Restart");
        let enable_button = Button::with_label("✓ Enable");
        let disable_button = Button::with_label("✗ Disable");
        let logs_button = Button::with_label("📋 Logs");

        button_box.append(&start_button);
        button_box.append(&stop_button);
        button_box.append(&restart_button);
        button_box.append(&enable_button);
        button_box.append(&disable_button);
        button_box.append(&logs_button);

        // Show inactive services toggle
        button_box.append(&self.show_inactive_button);

        main_box.append(&button_box);

        // Services list
        self.setup_local_services_list();
        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
        scrolled.set_child(Some(&self.local_services_list));

        scrolled.set_vexpand(true);
        main_box.append(&scrolled);

        // Setup local service control signals
        self.setup_local_service_signals(
            &start_button,
            &stop_button,
            &restart_button,
            &enable_button,
            &disable_button,
            &logs_button,
        );

        main_box
    }

    fn create_remote_page(&self) -> Box {
        let paned = Paned::new(gtk4::Orientation::Horizontal);

        // Left panel - hosts
        let hosts_box = Box::new(gtk4::Orientation::Vertical, 6);
        hosts_box.set_margin_start(12);
        hosts_box.set_margin_end(6);
        hosts_box.set_margin_top(12);
        hosts_box.set_margin_bottom(12);

        let hosts_label = Label::new(Some("Remote Hosts"));
        hosts_label.set_markup("<b>Remote Hosts</b>");
        hosts_box.append(&hosts_label);

        let add_host_button = Button::with_label("+ Add Host");
        hosts_box.append(&add_host_button);

        let scrolled_hosts = ScrolledWindow::new();
        scrolled_hosts.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scrolled_hosts.set_child(Some(&self.hosts_listbox));
        scrolled_hosts.set_size_request(250, -1);

        scrolled_hosts.set_vexpand(true);
        hosts_box.append(&scrolled_hosts);

        paned.set_start_child(Some(&hosts_box));

        // Right panel - remote services
        let services_box = Box::new(gtk4::Orientation::Vertical, 6);
        services_box.set_margin_start(6);
        services_box.set_margin_end(12);
        services_box.set_margin_top(12);
        services_box.set_margin_bottom(12);

        // Remote service control buttons
        let remote_button_box = Box::new(gtk4::Orientation::Horizontal, 6);

        let remote_start_button = Button::with_label("▶ Start");
        let remote_stop_button = Button::with_label("⏹ Stop");
        let remote_restart_button = Button::with_label("🔄 Restart");
        let remote_enable_button = Button::with_label("✓ Enable");
        let remote_disable_button = Button::with_label("✗ Disable");
        let remote_logs_button = Button::with_label("📋 Logs");

        remote_button_box.append(&remote_start_button);
        remote_button_box.append(&remote_stop_button);
        remote_button_box.append(&remote_restart_button);
        remote_button_box.append(&remote_enable_button);
        remote_button_box.append(&remote_disable_button);
        remote_button_box.append(&remote_logs_button);

        services_box.append(&remote_button_box);

        // Remote services list
        self.setup_remote_services_list();
        let scrolled_services = ScrolledWindow::new();
        scrolled_services.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
        scrolled_services.set_child(Some(&self.remote_services_list));

        scrolled_services.set_vexpand(true);
        services_box.append(&scrolled_services);

        paned.set_end_child(Some(&services_box));

        // Setup remote host signals
        self.setup_remote_host_signals(&add_host_button);
        self.setup_remote_service_signals(
            &remote_start_button,
            &remote_stop_button,
            &remote_restart_button,
            &remote_enable_button,
            &remote_disable_button,
            &remote_logs_button,
        );

        paned.upcast()
    }

    fn setup_local_services_list(&self) {
        self.local_services_list
            .set_model(Some(&self.local_services_store));

        // Service name column
        let name_column = TreeViewColumn::new();
        name_column.set_title("Service");
        name_column.set_resizable(true);
        name_column.set_sort_column_id(0);

        let name_renderer = CellRendererText::new();
        name_column.pack_start(&name_renderer, true);
        name_column.add_attribute(&name_renderer, "text", 0);

        self.local_services_list.append_column(&name_column);

        // Status column
        let status_column = TreeViewColumn::new();
        status_column.set_title("Status");
        status_column.set_resizable(true);
        status_column.set_sort_column_id(1);

        let status_renderer = CellRendererText::new();
        status_column.pack_start(&status_renderer, true);
        status_column.add_attribute(&status_renderer, "text", 1);

        self.local_services_list.append_column(&status_column);

        // Description column
        let desc_column = TreeViewColumn::new();
        desc_column.set_title("Description");
        desc_column.set_resizable(true);

        let desc_renderer = CellRendererText::new();
        desc_column.pack_start(&desc_renderer, true);
        desc_column.add_attribute(&desc_renderer, "text", 2);

        self.local_services_list.append_column(&desc_column);
    }

    fn setup_remote_services_list(&self) {
        self.remote_services_list
            .set_model(Some(&self.remote_services_store));

        // Host column
        let host_column = TreeViewColumn::new();
        host_column.set_title("Host");
        host_column.set_resizable(true);
        host_column.set_sort_column_id(0);

        let host_renderer = CellRendererText::new();
        host_column.pack_start(&host_renderer, true);
        host_column.add_attribute(&host_renderer, "text", 0);

        self.remote_services_list.append_column(&host_column);

        // Service name column
        let name_column = TreeViewColumn::new();
        name_column.set_title("Service");
        name_column.set_resizable(true);
        name_column.set_sort_column_id(1);

        let name_renderer = CellRendererText::new();
        name_column.pack_start(&name_renderer, true);
        name_column.add_attribute(&name_renderer, "text", 1);

        self.remote_services_list.append_column(&name_column);

        // Status column
        let status_column = TreeViewColumn::new();
        status_column.set_title("Status");
        status_column.set_resizable(true);
        status_column.set_sort_column_id(2);

        let status_renderer = CellRendererText::new();
        status_column.pack_start(&status_renderer, true);
        status_column.add_attribute(&status_renderer, "text", 2);

        self.remote_services_list.append_column(&status_column);

        // Description column
        let desc_column = TreeViewColumn::new();
        desc_column.set_title("Description");
        desc_column.set_resizable(true);

        let desc_renderer = CellRendererText::new();
        desc_column.pack_start(&desc_renderer, true);
        desc_column.add_attribute(&desc_renderer, "text", 3);

        self.remote_services_list.append_column(&desc_column);
    }

    fn setup_signal_handlers(&self) {
        // Show inactive services toggle
        let service_manager = self.service_manager.clone();
        let local_store = self.local_services_store.clone();

        self.show_inactive_button.connect_toggled(move |button| {
            let show_inactive = button.is_active();
            // Refresh local services with new filter
            // This would need to be implemented in service_manager
        });
    }

    fn setup_local_service_signals(
        &self,
        start_btn: &Button,
        stop_btn: &Button,
        restart_btn: &Button,
        enable_btn: &Button,
        disable_btn: &Button,
        logs_btn: &Button,
    ) {
        let selection = self.local_services_list.selection();

        // Start service
        let service_manager = self.service_manager.clone();
        let tree_selection = selection.clone();
        start_btn.connect_clicked(move |_| {
            if let Some(service_name) = get_selected_service_name(&tree_selection) {
                // Implement start service logic
                info!("Starting local service: {}", service_name);
            }
        });

        // Stop service
        let service_manager = self.service_manager.clone();
        let tree_selection = selection.clone();
        stop_btn.connect_clicked(move |_| {
            if let Some(service_name) = get_selected_service_name(&tree_selection) {
                info!("Stopping local service: {}", service_name);
            }
        });

        // Restart service
        let service_manager = self.service_manager.clone();
        let tree_selection = selection.clone();
        restart_btn.connect_clicked(move |_| {
            if let Some(service_name) = get_selected_service_name(&tree_selection) {
                info!("Restarting local service: {}", service_name);
            }
        });

        // Enable service
        let service_manager = self.service_manager.clone();
        let tree_selection = selection.clone();
        enable_btn.connect_clicked(move |_| {
            if let Some(service_name) = get_selected_service_name(&tree_selection) {
                info!("Enabling local service: {}", service_name);
            }
        });

        // Disable service
        let service_manager = self.service_manager.clone();
        let tree_selection = selection.clone();
        disable_btn.connect_clicked(move |_| {
            if let Some(service_name) = get_selected_service_name(&tree_selection) {
                info!("Disabling local service: {}", service_name);
            }
        });

        // Show logs
        let window = self.window.clone();
        let tree_selection = selection.clone();
        logs_btn.connect_clicked(move |_| {
            if let Some(service_name) = get_selected_service_name(&tree_selection) {
                show_service_logs_dialog(&window, &service_name, None);
            }
        });
    }

    fn setup_remote_host_signals(&self, add_host_btn: &Button) {
        let window = self.window.clone();
        let remote_hosts = self.remote_hosts.clone();

        add_host_btn.connect_clicked(move |_| {
            show_add_host_dialog(&window, &remote_hosts);
        });
    }

    fn setup_remote_service_signals(
        &self,
        start_btn: &Button,
        stop_btn: &Button,
        restart_btn: &Button,
        enable_btn: &Button,
        disable_btn: &Button,
        logs_btn: &Button,
    ) {
        let selection = self.remote_services_list.selection();

        // Similar to local service signals but for remote services
        // Implementation would handle remote SSH connections
    }

    pub fn load_saved_hosts(&self) {
        // Load saved remote hosts from configuration
        if let Ok(hosts) = self.load_hosts_from_config() {
            let mut remote_hosts = self.remote_hosts.borrow_mut();
            *remote_hosts = hosts;
            self.refresh_hosts_list();
        }
    }

    fn load_hosts_from_config(&self) -> Result<HashMap<String, RemoteHost>> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;
        let config_file = config_dir.join("systemd-pilot").join("hosts.json");

        if !config_file.exists() {
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(&config_file)?;
        let hosts: HashMap<String, RemoteHost> = serde_json::from_str(&content)?;
        Ok(hosts)
    }

    pub fn save_hosts(&self) -> Result<()> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow!("Could not find config directory"))?;
        let app_config_dir = config_dir.join("systemd-pilot");
        std::fs::create_dir_all(&app_config_dir)?;

        let config_file = app_config_dir.join("hosts.json");
        let hosts = self.remote_hosts.borrow();
        let content = serde_json::to_string_pretty(&*hosts)?;
        std::fs::write(&config_file, content)?;

        Ok(())
    }

    fn refresh_hosts_list(&self) {
        // Clear existing hosts in UI
        let children: Vec<gtk4::Widget> = self.hosts_listbox.children();
        for child in children {
            self.hosts_listbox.remove(&child);
        }

        // Add hosts to UI
        let hosts = self.remote_hosts.borrow();
        for (name, host) in hosts.iter() {
            let row = ListBoxRow::new();
            let label = Label::new(Some(&format!("{}@{}", host.username, host.hostname)));
            label.set_markup(&format!(
                "<b>{}</b>\n{}@{}",
                name, host.username, host.hostname
            ));
            row.add(&label);
            self.hosts_listbox.add(&row);
        }

        self.hosts_listbox.show_all();
    }

    fn refresh_all_services(&self) {
        self.refresh_local_services();
        self.refresh_remote_services();
    }

    fn refresh_local_services(&self) {
        let runtime = self.runtime.clone();
        let service_manager = self.service_manager.clone();
        let store = self.local_services_store.clone();
        let show_inactive = self.show_inactive_button.is_active();

        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);

        runtime.spawn(async move {
            match service_manager.list_local_services(show_inactive).await {
                Ok(services) => {
                    sender.send(services).expect("Failed to send services");
                }
                Err(e) => {
                    error!("Failed to list local services: {}", e);
                }
            }
        });

        receiver.attach(None, move |services| {
            store.clear();
            for service in services {
                let iter = store.append(None);
                store.set(
                    &iter,
                    &[
                        (0, &service.name),
                        (1, &service.status.to_string()),
                        (2, &service.description.unwrap_or_default()),
                    ],
                );
            }
            glib::Continue(false)
        });
    }

    fn refresh_remote_services(&self) {
        // Similar to local services but for remote hosts
        // Would iterate through active connections and refresh each
    }
}

fn get_selected_service_name(selection: &TreeSelection) -> Option<String> {
    if let Some((model, iter)) = selection.selected() {
        model.value(&iter, 0).get::<String>().ok()
    } else {
        None
    }
}

fn show_service_logs_dialog(parent: &ApplicationWindow, service_name: &str, host: Option<&str>) {
    let dialog = Dialog::with_buttons(
        Some(&format!("Logs for {}", service_name)),
        Some(parent),
        DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
        &[("Close", ResponseType::Close)],
    );

    dialog.set_default_size(800, 600);

    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);

    let text_view = TextView::new();
    text_view.set_editable(false);
    text_view.set_cursor_visible(false);

    // Set monospace font
    if let Some(font_desc) = pango::FontDescription::from_string("monospace") {
        text_view.override_font(Some(&font_desc));
    }

    scrolled.add(&text_view);

    let content_area = dialog.content_area();
    content_area.pack_start(&scrolled, true, true, 0);

    dialog.show_all();
    dialog.run();
    dialog.close();
}

fn show_add_host_dialog(
    parent: &ApplicationWindow,
    remote_hosts: &Rc<RefCell<HashMap<String, RemoteHost>>>,
) {
    let dialog = Dialog::with_buttons(
        Some("Add Remote Host"),
        Some(parent),
        DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
        &[("Cancel", ResponseType::Cancel), ("Add", ResponseType::Ok)],
    );

    let grid = Grid::new();
    grid.set_row_spacing(6);
    grid.set_column_spacing(12);
    grid.set_margin_start(12);
    grid.set_margin_end(12);
    grid.set_margin_top(12);
    grid.set_margin_bottom(12);

    // Name field
    let name_label = Label::new(Some("Name:"));
    let name_entry = Entry::new();
    grid.attach(&name_label, 0, 0, 1, 1);
    grid.attach(&name_entry, 1, 0, 1, 1);

    // Hostname field
    let hostname_label = Label::new(Some("Hostname:"));
    let hostname_entry = Entry::new();
    grid.attach(&hostname_label, 0, 1, 1, 1);
    grid.attach(&hostname_entry, 1, 1, 1, 1);

    // Username field
    let username_label = Label::new(Some("Username:"));
    let username_entry = Entry::new();
    grid.attach(&username_label, 0, 2, 1, 1);
    grid.attach(&username_entry, 1, 2, 1, 1);

    // Auth type
    let auth_label = Label::new(Some("Authentication:"));
    let auth_combo = ComboBoxText::new();
    auth_combo.append_text("Password");
    auth_combo.append_text("SSH Key");
    auth_combo.set_active(Some(0));
    grid.attach(&auth_label, 0, 3, 1, 1);
    grid.attach(&auth_combo, 1, 3, 1, 1);

    let content_area = dialog.content_area();
    content_area.pack_start(&grid, true, true, 0);

    dialog.show_all();

    if dialog.run() == ResponseType::Ok {
        let name = name_entry.text().to_string();
        let hostname = hostname_entry.text().to_string();
        let username = username_entry.text().to_string();
        let auth_type = if auth_combo.active() == Some(0) {
            AuthType::Password
        } else {
            AuthType::Key { path: None }
        };

        if !name.is_empty() && !hostname.is_empty() && !username.is_empty() {
            let host = RemoteHost {
                name: name.clone(),
                hostname,
                username,
                auth_type,
            };

            remote_hosts.borrow_mut().insert(name, host);
        }
    }

    dialog.close();
}
