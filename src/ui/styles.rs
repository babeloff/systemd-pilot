use gtk4::prelude::*;
use gtk4::{CssProvider, StyleContext, Widget, STYLE_PROVIDER_PRIORITY_APPLICATION};
use log::{debug, error, warn};

/// Additional CSS styles for specific components
const COMPONENT_STYLES: &str = r#"
    /* Service list specific styles */
    .service-list {
        background: @theme_base_color;
        border: 1px solid @borders;
        border-radius: 8px;
    }

    .service-list row {
        padding: 8px 12px;
        border-bottom: 1px solid alpha(@borders, 0.3);
        transition: background-color 200ms ease;
    }

    .service-list row:last-child {
        border-bottom: none;
    }

    .service-list row:hover {
        background: alpha(@theme_selected_bg_color, 0.05);
    }

    .service-list row:selected {
        background: @theme_selected_bg_color;
        color: @theme_selected_fg_color;
    }

    /* Host list styles */
    .host-list {
        background: @theme_base_color;
        border: 1px solid @borders;
        border-radius: 8px;
        min-width: 200px;
    }

    .host-list row {
        padding: 12px;
        border-bottom: 1px solid alpha(@borders, 0.2);
    }

    .host-list row:last-child {
        border-bottom: none;
    }

    .host-item {
        padding: 8px;
    }

    .host-name {
        font-weight: bold;
        font-size: 1.1em;
    }

    .host-connection {
        font-size: 0.9em;
        opacity: 0.7;
    }

    /* Button group styles */
    .button-group {
        background: alpha(@theme_bg_color, 0.5);
        border: 1px solid @borders;
        border-radius: 8px;
        padding: 6px;
    }

    .button-group button {
        margin: 2px;
        border-radius: 6px;
        min-width: 80px;
    }

    .button-group separator {
        margin: 0 4px;
        background: @borders;
    }

    /* Status indicators */
    .status-indicator {
        padding: 4px 8px;
        border-radius: 12px;
        font-size: 0.85em;
        font-weight: bold;
        text-transform: uppercase;
    }

    .status-active {
        background: alpha(#27ae60, 0.2);
        color: #27ae60;
        border: 1px solid alpha(#27ae60, 0.4);
    }

    .status-inactive {
        background: alpha(#7f8c8d, 0.2);
        color: #7f8c8d;
        border: 1px solid alpha(#7f8c8d, 0.4);
    }

    .status-failed {
        background: alpha(#e74c3c, 0.2);
        color: #e74c3c;
        border: 1px solid alpha(#e74c3c, 0.4);
    }

    .status-unknown {
        background: alpha(#f39c12, 0.2);
        color: #f39c12;
        border: 1px solid alpha(#f39c12, 0.4);
    }

    /* Connection status */
    .connection-connected {
        color: #27ae60;
    }

    .connection-disconnected {
        color: #e74c3c;
    }

    .connection-connecting {
        color: #f39c12;
    }

    /* Logs viewer styles */
    .logs-viewer {
        background: #1e1e1e;
        color: #d4d4d4;
        font-family: 'Fira Code', 'Source Code Pro', 'Liberation Mono', monospace;
        font-size: 0.9em;
        line-height: 1.4;
        border: 1px solid @borders;
        border-radius: 8px;
        padding: 8px;
    }

    .logs-viewer.light {
        background: #f8f9fa;
        color: #212529;
    }

    /* Filter bar styles */
    .filter-bar {
        background: alpha(@theme_bg_color, 0.7);
        border-bottom: 1px solid @borders;
        padding: 8px 12px;
    }

    .filter-bar entry {
        min-width: 200px;
    }

    .filter-bar combobox {
        min-width: 120px;
    }

    /* Service details panel */
    .service-details {
        background: @theme_base_color;
        border: 1px solid @borders;
        border-radius: 8px;
        padding: 12px;
    }

    .service-details .property-key {
        font-weight: bold;
        color: @theme_fg_color;
        min-width: 80px;
    }

    .service-details .property-value {
        color: alpha(@theme_fg_color, 0.8);
        word-wrap: break-word;
    }

    /* Loading states */
    .loading-overlay {
        background: alpha(@theme_bg_color, 0.9);
        border-radius: 8px;
    }

    .loading-spinner {
        color: @theme_selected_bg_color;
    }

    /* Error states */
    .error-widget {
        background: alpha(#e74c3c, 0.1);
        border: 1px solid alpha(#e74c3c, 0.3);
        border-radius: 8px;
        padding: 16px;
    }

    .error-icon {
        color: #e74c3c;
        font-size: 2em;
    }

    .error-message {
        color: #e74c3c;
        font-weight: bold;
    }

    /* Empty state */
    .empty-state {
        padding: 32px;
        text-align: center;
    }

    .empty-state .icon {
        font-size: 3em;
        opacity: 0.5;
        margin-bottom: 16px;
    }

    .empty-state .title {
        font-size: 1.2em;
        font-weight: bold;
        margin-bottom: 8px;
    }

    .empty-state .subtitle {
        opacity: 0.7;
        line-height: 1.4;
    }

    /* Dialog improvements */
    dialog .dialog-content {
        padding: 20px;
    }

    dialog headerbar {
        border-radius: 8px 8px 0 0;
    }

    dialog .dialog-action-area {
        padding: 12px 20px;
        border-top: 1px solid @borders;
    }

    /* Notebook improvements */
    notebook > header {
        background: @theme_bg_color;
        border-bottom: 1px solid @borders;
    }

    notebook > header > tabs > tab {
        padding: 12px 20px;
        font-weight: 500;
        border-radius: 6px 6px 0 0;
        margin: 2px 1px 0 1px;
        transition: background-color 200ms ease;
    }

    notebook > header > tabs > tab:hover {
        background: alpha(@theme_selected_bg_color, 0.1);
    }

    notebook > header > tabs > tab:checked {
        background: @theme_base_color;
        border-bottom: 2px solid @theme_selected_bg_color;
        color: @theme_selected_bg_color;
    }

    /* TreeView improvements */
    treeview.view {
        border-radius: 8px;
        background: @theme_base_color;
    }

    treeview.view:selected {
        background: @theme_selected_bg_color;
        color: @theme_selected_fg_color;
    }

    treeview.view:selected:backdrop {
        background: alpha(@theme_selected_bg_color, 0.7);
    }

    treeview header button {
        background: @theme_bg_color;
        border-bottom: 1px solid @borders;
        border-right: 1px solid alpha(@borders, 0.5);
        padding: 8px 12px;
        font-weight: 600;
        text-transform: uppercase;
        font-size: 0.85em;
        letter-spacing: 0.5px;
    }

    treeview header button:last-child {
        border-right: none;
    }

    treeview header button:hover {
        background: alpha(@theme_selected_bg_color, 0.1);
    }

    /* Paned separator styling */
    paned > separator {
        background: @borders;
        min-width: 1px;
        min-height: 1px;
        transition: background-color 200ms ease;
    }

    paned > separator:hover {
        background: @theme_selected_bg_color;
    }

    /* Scrollbar improvements */
    scrollbar {
        background: transparent;
        border: none;
    }

    scrollbar.horizontal {
        border-top: 1px solid alpha(@borders, 0.3);
    }

    scrollbar.vertical {
        border-left: 1px solid alpha(@borders, 0.3);
    }

    scrollbar slider {
        background: alpha(@theme_fg_color, 0.3);
        border-radius: 3px;
        min-width: 6px;
        min-height: 6px;
        margin: 2px;
        transition: background-color 200ms ease;
    }

    scrollbar slider:hover {
        background: alpha(@theme_fg_color, 0.5);
    }

    scrollbar slider:active {
        background: @theme_selected_bg_color;
    }

    /* Entry improvements */
    entry {
        border-radius: 6px;
        padding: 8px 12px;
        border: 1px solid @borders;
        background: @theme_base_color;
        transition: border-color 200ms ease, box-shadow 200ms ease;
    }

    entry:focus {
        border-color: @theme_selected_bg_color;
        box-shadow: 0 0 0 2px alpha(@theme_selected_bg_color, 0.2);
        outline: none;
    }

    entry.search {
        background: alpha(@theme_base_color, 0.9);
    }

    /* ComboBox improvements */
    combobox {
        border-radius: 6px;
    }

    combobox button {
        padding: 8px 12px;
        border: 1px solid @borders;
        background: @theme_base_color;
    }

    combobox button:hover {
        background: alpha(@theme_selected_bg_color, 0.1);
    }

    /* CheckButton improvements */
    checkbutton {
        padding: 4px 8px;
    }

    checkbutton check {
        border-radius: 3px;
        border: 1px solid @borders;
        background: @theme_base_color;
        transition: all 200ms ease;
    }

    checkbutton:checked check {
        background: @theme_selected_bg_color;
        border-color: @theme_selected_bg_color;
        color: @theme_selected_fg_color;
    }

    /* Animation classes */
    .fade-in {
        animation: fadeIn 300ms ease-in-out;
    }

    .slide-in-right {
        animation: slideInRight 300ms ease-out;
    }

    .pulse {
        animation: pulse 2s infinite;
    }

    @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }

    @keyframes slideInRight {
        from {
            opacity: 0;
            transform: translateX(20px);
        }
        to {
            opacity: 1;
            transform: translateX(0);
        }
    }

    @keyframes pulse {
        0% { opacity: 1; }
        50% { opacity: 0.5; }
        100% { opacity: 1; }
    }

    /* Utility classes */
    .dim-label {
        opacity: 0.7;
    }

    .bold-text {
        font-weight: bold;
    }

    .small-text {
        font-size: 0.85em;
    }

    .large-text {
        font-size: 1.15em;
    }

    .monospace {
        font-family: monospace;
    }

    .text-center {
        text-align: center;
    }

    .margin-small {
        margin: 6px;
    }

    .margin-medium {
        margin: 12px;
    }

    .margin-large {
        margin: 18px;
    }

    .padding-small {
        padding: 6px;
    }

    .padding-medium {
        padding: 12px;
    }

    .padding-large {
        padding: 18px;
    }
"#;

/// Applies additional component-specific styles to a widget
pub fn apply_component_styles(widget: &impl IsA<Widget>) -> Result<(), Box<dyn std::error::Error>> {
    let css_provider = CssProvider::new();

    css_provider.load_from_data(COMPONENT_STYLES);

    let style_context = widget.style_context();
    style_context.add_provider(&css_provider, STYLE_PROVIDER_PRIORITY_APPLICATION);

    debug!("Applied component-specific styles");
    Ok(())
}

/// Adds a CSS class to a widget
pub fn add_css_class(widget: &impl IsA<Widget>, class_name: &str) {
    let style_context = widget.style_context();
    style_context.add_class(class_name);
}

/// Removes a CSS class from a widget
pub fn remove_css_class(widget: &impl IsA<Widget>, class_name: &str) {
    let style_context = widget.style_context();
    style_context.remove_class(class_name);
}

/// Toggles a CSS class on a widget
pub fn toggle_css_class(widget: &impl IsA<Widget>, class_name: &str) {
    let style_context = widget.style_context();
    if style_context.has_class(class_name) {
        style_context.remove_class(class_name);
    } else {
        style_context.add_class(class_name);
    }
}

/// Sets multiple CSS classes on a widget, removing any existing classes first
pub fn set_css_classes(widget: &impl IsA<Widget>, class_names: &[&str]) {
    let style_context = widget.style_context();

    // Remove existing classes (this is a simplified approach)
    // In a real implementation, you might want to track which classes were added
    for class_name in class_names {
        style_context.remove_class(class_name);
    }

    // Add new classes
    for class_name in class_names {
        style_context.add_class(class_name);
    }
}

/// Creates a styled separator widget
pub fn create_styled_separator(orientation: gtk4::Orientation) -> gtk4::Separator {
    let separator = gtk4::Separator::new(orientation);
    add_css_class(&separator, "styled-separator");
    separator
}

/// Applies loading state styling to a widget
pub fn apply_loading_state(widget: &impl IsA<Widget>) {
    add_css_class(widget, "loading-state");
    add_css_class(widget, "pulse");
}

/// Removes loading state styling from a widget
pub fn remove_loading_state(widget: &impl IsA<Widget>) {
    remove_css_class(widget, "loading-state");
    remove_css_class(widget, "pulse");
}

/// Applies error state styling to a widget
pub fn apply_error_state(widget: &impl IsA<Widget>) {
    add_css_class(widget, "error-state");
}

/// Removes error state styling from a widget
pub fn remove_error_state(widget: &impl IsA<Widget>) {
    remove_css_class(widget, "error-state");
}

/// Applies success state styling to a widget
pub fn apply_success_state(widget: &impl IsA<Widget>) {
    add_css_class(widget, "success-state");
}

/// Removes success state styling from a widget
pub fn remove_success_state(widget: &impl IsA<Widget>) {
    remove_css_class(widget, "success-state");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_styles_not_empty() {
        assert!(!COMPONENT_STYLES.is_empty());
        assert!(COMPONENT_STYLES.contains("service-list"));
        assert!(COMPONENT_STYLES.contains("host-list"));
        assert!(COMPONENT_STYLES.contains("button-group"));
    }

    #[test]
    fn test_css_class_names() {
        let test_cases = vec![
            "service-list",
            "status-active",
            "loading-state",
            "error-state",
            "success-state",
        ];

        for class_name in test_cases {
            assert!(COMPONENT_STYLES.contains(class_name));
        }
    }
}
