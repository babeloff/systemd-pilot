use gdk4::Display;
use gio::Settings;
use gtk4::prelude::*;
use gtk4::{CssProvider, StyleContext, Widget, STYLE_PROVIDER_PRIORITY_APPLICATION};
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::rc::Rc;

pub struct ThemeManager {
    is_dark_mode: RefCell<bool>,
    css_provider: CssProvider,
}

impl ThemeManager {
    pub fn new() -> Self {
        let css_provider = CssProvider::new();
        let is_dark_mode = RefCell::new(Self::detect_system_theme());

        Self {
            is_dark_mode,
            css_provider,
        }
    }

    pub fn detect_system_theme() -> bool {
        // Try to detect system theme preference
        if let Ok(settings) = Settings::new("org.gnome.desktop.interface") {
            let gtk_theme = settings.string("gtk-theme");
            return gtk_theme.to_lowercase().contains("dark");
        }

        // Fallback to environment variable
        if let Ok(gtk_theme) = std::env::var("GTK_THEME") {
            return gtk_theme.to_lowercase().contains("dark");
        }

        // Default to light theme
        false
    }

    pub fn is_dark_mode(&self) -> bool {
        *self.is_dark_mode.borrow()
    }

    pub fn toggle_theme(&self) {
        let current = *self.is_dark_mode.borrow();
        *self.is_dark_mode.borrow_mut() = !current;
        info!(
            "Theme toggled to: {}",
            if !current { "dark" } else { "light" }
        );
    }

    pub fn set_dark_mode(&self, dark: bool) {
        *self.is_dark_mode.borrow_mut() = dark;
    }

    pub fn apply_theme(&self, window: &impl IsA<gtk4::Widget>) {
        let is_dark = *self.is_dark_mode.borrow();

        // Apply GTK theme preference
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_property("gtk-application-prefer-dark-theme", is_dark);
        }

        // Load custom CSS
        let css = self.get_custom_css(is_dark);

        if let Err(e) = self.css_provider.load_from_data(css.as_bytes()) {
            error!("Failed to load CSS: {}", e);
            return;
        }

        // Apply CSS to the display
        if let Some(display) = Display::default() {
            StyleContext::add_provider_for_display(
                &display,
                &self.css_provider,
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        debug!("Applied {} theme", if is_dark { "dark" } else { "light" });
    }

    fn get_custom_css(&self, is_dark: bool) -> String {
        let base_css = r#"
            /* Base styling for systemd Pilot */

            /* Header bar styling */
            headerbar {
                border-bottom: 1px solid @borders;
            }

            headerbar button {
                margin: 4px;
                padding: 6px 12px;
                border-radius: 6px;
            }

            /* Notebook styling */
            notebook {
                background: @theme_base_color;
            }

            notebook header {
                background: @theme_bg_color;
                border-bottom: 1px solid @borders;
            }

            notebook tab {
                padding: 8px 16px;
                margin: 2px;
                border-radius: 6px 6px 0 0;
            }

            notebook tab:checked {
                background: @theme_base_color;
                border-bottom: 2px solid @theme_selected_bg_color;
            }

            /* TreeView styling */
            treeview {
                background: @theme_base_color;
                border: 1px solid @borders;
                border-radius: 6px;
            }

            treeview header button {
                background: @theme_bg_color;
                border-bottom: 1px solid @borders;
                padding: 8px;
                font-weight: bold;
            }

            treeview:selected {
                background: @theme_selected_bg_color;
                color: @theme_selected_fg_color;
            }

            /* Service status colors */
            .service-active {
                color: #27ae60;
                font-weight: bold;
            }

            .service-inactive {
                color: #7f8c8d;
            }

            .service-failed {
                color: #e74c3c;
                font-weight: bold;
            }

            .service-unknown {
                color: #f39c12;
            }

            /* Button styling */
            button {
                border-radius: 6px;
                padding: 6px 12px;
                margin: 2px;
            }

            button:hover {
                background: alpha(@theme_selected_bg_color, 0.1);
            }

            button.destructive-action {
                background: #e74c3c;
                color: white;
            }

            button.destructive-action:hover {
                background: #c0392b;
            }

            button.suggested-action {
                background: @theme_selected_bg_color;
                color: @theme_selected_fg_color;
            }

            /* ScrolledWindow styling */
            scrolledwindow {
                border: 1px solid @borders;
                border-radius: 6px;
            }

            /* Entry styling */
            entry {
                border-radius: 6px;
                padding: 8px;
                border: 1px solid @borders;
            }

            entry:focus {
                border-color: @theme_selected_bg_color;
                box-shadow: 0 0 0 2px alpha(@theme_selected_bg_color, 0.2);
            }

            /* Dialog styling */
            dialog {
                border-radius: 12px;
            }

            dialog headerbar {
                border-radius: 12px 12px 0 0;
            }

            /* ListBox styling */
            listbox {
                background: @theme_base_color;
                border: 1px solid @borders;
                border-radius: 6px;
            }

            listbox row {
                padding: 12px;
                border-bottom: 1px solid alpha(@borders, 0.5);
            }

            listbox row:last-child {
                border-bottom: none;
            }

            listbox row:selected {
                background: @theme_selected_bg_color;
                color: @theme_selected_fg_color;
            }

            /* Paned styling */
            paned separator {
                background: @borders;
                min-width: 1px;
                min-height: 1px;
            }

            /* TextView styling for logs */
            textview {
                background: @theme_base_color;
                border: 1px solid @borders;
                border-radius: 6px;
                padding: 8px;
            }

            textview.monospace {
                font-family: monospace;
                font-size: 0.9em;
            }
        "#;

        let theme_specific_css = if is_dark {
            r#"
                /* Dark theme specific styles */

                /* Darker backgrounds for logs and code */
                textview.monospace {
                    background: #1e1e1e;
                    color: #d4d4d4;
                }

                /* Darker service status colors for better contrast */
                .service-active {
                    color: #4ade80;
                }

                .service-inactive {
                    color: #9ca3af;
                }

                .service-failed {
                    color: #f87171;
                }

                .service-unknown {
                    color: #fbbf24;
                }

                /* Dark scrollbars */
                scrollbar {
                    background: #2d2d2d;
                }

                scrollbar slider {
                    background: #555555;
                    border-radius: 6px;
                }

                scrollbar slider:hover {
                    background: #666666;
                }
            "#
        } else {
            r#"
                /* Light theme specific styles */

                /* Light backgrounds for logs and code */
                textview.monospace {
                    background: #f8f9fa;
                    color: #212529;
                }

                /* Light scrollbars */
                scrollbar {
                    background: #e9ecef;
                }

                scrollbar slider {
                    background: #adb5bd;
                    border-radius: 6px;
                }

                scrollbar slider:hover {
                    background: #868e96;
                }
            "#
        };

        format!("{}\n{}", base_css, theme_specific_css)
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_manager_creation() {
        let theme_manager = ThemeManager::new();
        // Should not panic
        assert!(theme_manager.is_dark_mode() == true || theme_manager.is_dark_mode() == false);
    }

    #[test]
    fn test_theme_toggle() {
        let theme_manager = ThemeManager::new();
        let initial_state = theme_manager.is_dark_mode();
        theme_manager.toggle_theme();
        assert_ne!(initial_state, theme_manager.is_dark_mode());
    }

    #[test]
    fn test_set_dark_mode() {
        let theme_manager = ThemeManager::new();
        theme_manager.set_dark_mode(true);
        assert!(theme_manager.is_dark_mode());
        theme_manager.set_dark_mode(false);
        assert!(!theme_manager.is_dark_mode());
    }

    #[test]
    fn test_css_generation() {
        let theme_manager = ThemeManager::new();
        let dark_css = theme_manager.get_custom_css(true);
        let light_css = theme_manager.get_custom_css(false);

        assert!(dark_css.contains("Dark theme specific"));
        assert!(light_css.contains("Light theme specific"));
        assert!(dark_css.len() > 0);
        assert!(light_css.len() > 0);
    }
}
