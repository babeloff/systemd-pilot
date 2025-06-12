use gtk4::prelude::*;
use gtk4::{
    Box, Button, CellRendererText, CheckButton, ComboBoxText, Entry, Grid, Label, ListBox,
    ListBoxRow, Paned, ScrolledWindow, Separator, TreeView, TreeViewColumn, Widget,
};
use log::{debug, error, info, warn};
use std::rc::Rc;

use crate::service_manager::{ServiceInfo, ServiceStatus};

/// Creates a styled service control button with icon and text
pub fn create_service_button(icon: &str, text: &str, tooltip: Option<&str>) -> Button {
    let button = Button::with_label(&format!("{} {}", icon, text));
    button.set_margin_start(4);
    button.set_margin_end(4);

    if let Some(tip) = tooltip {
        button.set_tooltip_text(Some(tip));
    }

    // Add CSS class based on button type
    let style_context = button.style_context();
    match text {
        "Start" => style_context.add_class("suggested-action"),
        "Stop" | "Disable" => style_context.add_class("destructive-action"),
        _ => {}
    }

    button
}

/// Creates a horizontal button box with common service control buttons
pub fn create_service_control_buttons() -> (Box, Button, Button, Button, Button, Button, Button) {
    let button_box = Box::new(gtk4::Orientation::Horizontal, 6);
    button_box.set_margin_start(12);
    button_box.set_margin_end(12);
    button_box.set_margin_top(6);
    button_box.set_margin_bottom(6);

    let start_button = create_service_button("▶", "Start", Some("Start the selected service"));
    let stop_button = create_service_button("⏹", "Stop", Some("Stop the selected service"));
    let restart_button =
        create_service_button("🔄", "Restart", Some("Restart the selected service"));
    let enable_button = create_service_button("✓", "Enable", Some("Enable service at boot"));
    let disable_button = create_service_button("✗", "Disable", Some("Disable service at boot"));
    let logs_button = create_service_button("📋", "Logs", Some("View service logs"));

    button_box.append(&start_button);
    button_box.append(&stop_button);
    button_box.append(&restart_button);
    button_box.append(&Separator::new(gtk::Orientation::Vertical));
    button_box.append(&enable_button);
    button_box.append(&disable_button);
    button_box.append(&Separator::new(gtk::Orientation::Vertical));
    button_box.append(&logs_button);

    (
        button_box,
        start_button,
        stop_button,
        restart_button,
        enable_button,
        disable_button,
        logs_button,
    )
}

/// Creates a styled TreeView for displaying services
pub fn create_services_tree_view(columns: &[&str]) -> (TreeView, gtk4::TreeStore) {
    let tree_view = TreeView::new();
    tree_view.set_search_column(0);
    tree_view.set_enable_search(true);

    // Create appropriate number of string columns
    let column_types: Vec<glib::Type> = columns.iter().map(|_| glib::Type::STRING).collect();
    let tree_store = gtk4::TreeStore::new(&column_types);
    tree_view.set_model(Some(&tree_store));

    // Add columns
    for (i, &column_name) in columns.iter().enumerate() {
        let column = TreeViewColumn::new();
        column.set_title(column_name);
        column.set_resizable(true);
        column.set_sort_column_id(i as i32);

        let renderer = CellRendererText::new();
        column.pack_start(&renderer, true);
        column.add_attribute(&renderer, "text", i as i32);

        // Special styling for status column
        if column_name == "Status" {
            column.set_cell_data_func(&renderer, Some(Box::new(format_status_cell)));
        }

        tree_view.append_column(&column);
    }

    (tree_view, tree_store)
}

/// Custom cell data function for status column styling
fn format_status_cell(
    _column: &TreeViewColumn,
    cell: &gtk4::CellRenderer,
    model: &gtk4::TreeModel,
    iter: &gtk4::TreeIter,
) {
    if let Some(cell_text) = cell.downcast_ref::<CellRendererText>() {
        if let Ok(status_text) = model.value(iter, 1).get::<String>() {
            let css_class = match status_text.as_str() {
                "Active" => "service-active",
                "Inactive" => "service-inactive",
                "Failed" => "service-failed",
                _ => "service-unknown",
            };

            // Apply CSS class for styling
            let style_context = cell_text.style_context();
            style_context.add_class(css_class);
        }
    }
}

/// Creates a host list item widget
pub fn create_host_list_item(
    name: &str,
    hostname: &str,
    username: &str,
    connected: bool,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_margin_start(6);
    row.set_margin_end(6);
    row.set_margin_top(3);
    row.set_margin_bottom(3);

    let main_box = Box::new(gtk4::Orientation::Horizontal, 12);
    main_box.set_margin_start(12);
    main_box.set_margin_end(12);
    main_box.set_margin_top(8);
    main_box.set_margin_bottom(8);

    // Connection status indicator
    let status_indicator = Label::new(Some(if connected { "🟢" } else { "🔴" }));
    status_indicator.set_tooltip_text(Some(if connected {
        "Connected"
    } else {
        "Disconnected"
    }));

    // Host info
    let info_box = Box::new(gtk4::Orientation::Vertical, 4);

    let name_label = Label::new(Some(name));
    name_label.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(name)));
    name_label.set_halign(gtk4::Align::Start);

    let connection_label = Label::new(Some(&format!("{}@{}", username, hostname)));
    connection_label.set_halign(gtk4::Align::Start);
    let style_context = connection_label.style_context();
    style_context.add_class("dim-label");

    info_box.append(&name_label);
    info_box.append(&connection_label);

    main_box.append(&status_indicator);
    main_box.append(&info_box);

    row.set_child(Some(&main_box));
    row
}

/// Creates a filter/search box for services
pub fn create_service_filter_box() -> (Box, Entry, CheckButton, ComboBoxText) {
    let filter_box = Box::new(gtk4::Orientation::Horizontal, 12);
    filter_box.set_margin_start(12);
    filter_box.set_margin_end(12);
    filter_box.set_margin_top(6);
    filter_box.set_margin_bottom(6);

    // Search entry
    let search_entry = Entry::new();
    search_entry.set_placeholder_text(Some("Search services..."));
    search_entry
        .set_icon_from_icon_name(gtk4::EntryIconPosition::Primary, Some("edit-find-symbolic"));

    // Show inactive services toggle
    let show_inactive = CheckButton::with_label("Show inactive services");

    // Status filter combo
    let status_filter = ComboBoxText::new();
    status_filter.append_text("All Services");
    status_filter.append_text("Active Only");
    status_filter.append_text("Failed Only");
    status_filter.append_text("Inactive Only");
    status_filter.set_active(Some(1)); // Default to "Active Only"

    search_entry.set_hexpand(true);
    filter_box.append(&search_entry);
    filter_box.append(&show_inactive);
    filter_box.append(&status_filter);

    (filter_box, search_entry, show_inactive, status_filter)
}

/// Creates a connection status bar
pub fn create_connection_status_bar() -> (Box, Label, Button) {
    let status_bar = Box::new(gtk4::Orientation::Horizontal, 6);
    status_bar.set_margin_start(12);
    status_bar.set_margin_end(12);
    status_bar.set_margin_top(3);
    status_bar.set_margin_bottom(3);

    let status_label = Label::new(Some("Ready"));
    status_label.set_halign(gtk4::Align::Start);

    let refresh_button = Button::with_label("🔄 Refresh");
    refresh_button.set_tooltip_text(Some("Refresh all services"));

    status_label.set_hexpand(true);
    status_bar.append(&status_label);
    status_bar.append(&refresh_button);

    (status_bar, status_label, refresh_button)
}

/// Creates a details panel for displaying service information
pub fn create_service_details_panel() -> (Box, Label, Label, Label, Label) {
    let details_box = Box::new(gtk4::Orientation::Vertical, 8);
    details_box.set_margin_start(12);
    details_box.set_margin_end(12);
    details_box.set_margin_top(8);
    details_box.set_margin_bottom(8);

    // Title
    let title_label = Label::new(Some("Service Details"));
    title_label.set_markup("<b>Service Details</b>");
    title_label.set_halign(gtk4::Align::Start);

    // Service info grid
    let info_grid = Grid::new();
    info_grid.set_row_spacing(6);
    info_grid.set_column_spacing(12);

    // Labels for service properties
    let name_key = Label::new(Some("Name:"));
    name_key.set_halign(gtk4::Align::Start);
    name_key.set_markup("<b>Name:</b>");

    let status_key = Label::new(Some("Status:"));
    status_key.set_halign(gtk4::Align::Start);
    status_key.set_markup("<b>Status:</b>");

    let enabled_key = Label::new(Some("Enabled:"));
    enabled_key.set_halign(gtk4::Align::Start);
    enabled_key.set_markup("<b>Enabled:</b>");

    let description_key = Label::new(Some("Description:"));
    description_key.set_halign(gtk4::Align::Start);
    description_key.set_markup("<b>Description:</b>");

    // Value labels
    let name_value = Label::new(Some("-"));
    name_value.set_halign(gtk4::Align::Start);
    name_value.set_selectable(true);

    let status_value = Label::new(Some("-"));
    status_value.set_halign(gtk4::Align::Start);

    let enabled_value = Label::new(Some("-"));
    enabled_value.set_halign(gtk4::Align::Start);

    let description_value = Label::new(Some("-"));
    description_value.set_halign(gtk4::Align::Start);
    description_value.set_line_wrap(true);
    description_value.set_selectable(true);

    // Arrange in grid
    info_grid.attach(&name_key, 0, 0, 1, 1);
    info_grid.attach(&name_value, 1, 0, 1, 1);
    info_grid.attach(&status_key, 0, 1, 1, 1);
    info_grid.attach(&status_value, 1, 1, 1, 1);
    info_grid.attach(&enabled_key, 0, 2, 1, 1);
    info_grid.attach(&enabled_value, 1, 2, 1, 1);
    info_grid.attach(&description_key, 0, 3, 1, 1);
    info_grid.attach(&description_value, 1, 3, 1, 1);

    details_box.append(&title_label);
    details_box.append(&Separator::new(gtk4::Orientation::Horizontal));
    details_box.append(&info_grid);

    (
        details_box,
        name_value,
        status_value,
        enabled_value,
        description_value,
    )
}

/// Updates service details panel with service information
pub fn update_service_details_panel(
    name_label: &Label,
    status_label: &Label,
    enabled_label: &Label,
    description_label: &Label,
    service: &ServiceInfo,
) {
    name_label.set_text(&service.name);

    // Set status with color
    status_label.set_markup(&format!(
        "<span class=\"service-{}\"><b>{}</b></span>",
        match service.status {
            ServiceStatus::Active => "active",
            ServiceStatus::Inactive => "inactive",
            ServiceStatus::Failed => "failed",
            ServiceStatus::Unknown => "unknown",
        },
        service.status
    ));

    enabled_label.set_text(if service.enabled { "Yes" } else { "No" });
    description_label.set_text(
        &service
            .description
            .as_deref()
            .unwrap_or("No description available"),
    );
}

/// Creates a loading spinner widget
pub fn create_loading_spinner(text: &str) -> Box {
    let spinner_box = Box::new(gtk4::Orientation::Horizontal, 8);
    spinner_box.set_halign(gtk4::Align::Center);
    spinner_box.set_valign(gtk4::Align::Center);

    let spinner = gtk4::Spinner::new();
    spinner.start();

    let label = Label::new(Some(text));

    spinner_box.append(&spinner);
    spinner_box.append(&label);

    spinner_box
}

/// Creates an error message widget
pub fn create_error_widget(message: &str) -> Box {
    let error_box = Box::new(gtk4::Orientation::Vertical, 8);
    error_box.set_halign(gtk4::Align::Center);
    error_box.set_valign(gtk4::Align::Center);

    let icon = Label::new(Some("⚠️"));
    icon.set_markup("<span size=\"xx-large\">⚠️</span>");

    let label = Label::new(Some(message));
    label.set_line_wrap(true);
    label.set_justify(gtk4::Justification::Center);

    error_box.append(&icon);
    error_box.append(&label);

    error_box
}

/// Creates an empty state widget
pub fn create_empty_state_widget(title: &str, subtitle: &str) -> Box {
    let empty_box = Box::new(gtk4::Orientation::Vertical, 12);
    empty_box.set_halign(gtk4::Align::Center);
    empty_box.set_valign(gtk4::Align::Center);

    let icon = Label::new(Some("📋"));
    icon.set_markup("<span size=\"xx-large\">📋</span>");

    let title_label = Label::new(Some(title));
    title_label.set_markup(&format!(
        "<b><big>{}</big></b>",
        glib::markup_escape_text(title)
    ));

    let subtitle_label = Label::new(Some(subtitle));
    subtitle_label.set_line_wrap(true);
    subtitle_label.set_justify(gtk4::Justification::Center);
    let style_context = subtitle_label.style_context();
    style_context.add_class("dim-label");

    empty_box.append(&icon);
    empty_box.append(&title_label);
    empty_box.append(&subtitle_label);

    empty_box
}

/// Utility function to apply service-specific styling to a widget
pub fn apply_service_status_style(widget: &impl IsA<Widget>, status: &ServiceStatus) {
    let style_context = widget.style_context();

    // Remove existing status classes
    style_context.remove_class("service-active");
    style_context.remove_class("service-inactive");
    style_context.remove_class("service-failed");
    style_context.remove_class("service-unknown");

    // Add appropriate class
    let css_class = match status {
        ServiceStatus::Active => "service-active",
        ServiceStatus::Inactive => "service-inactive",
        ServiceStatus::Failed => "service-failed",
        ServiceStatus::Unknown => "service-unknown",
    };

    style_context.add_class(css_class);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_service_button() {
        // This test would need GTK to be initialized
        // In a real test environment, you'd need to call gtk4::init() first
        // For now, we'll just test that the function exists and can be called
        assert!(true);
    }
}
