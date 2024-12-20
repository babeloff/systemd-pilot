#!/usr/bin/env python3
import gi
gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
gi.require_version('GtkSource', '5')
from gi.repository import Gtk, Adw, GLib, Gio, Gdk, Pango, GtkSource
import subprocess
import os
from datetime import datetime

APP_VERSION = "2.0.0"

class SystemdManagerWindow(Adw.ApplicationWindow):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.set_default_size(800, 600)
        self.set_title("systemd Pilot")
        self.all_services = []
        self.is_root = os.geteuid() == 0
        self.current_filter = "all"
        self.show_only_loaded = True

        # Set up search action
        search_action = Gio.SimpleAction.new("search", None)
        search_action.connect("activate", self.toggle_search)
        self.add_action(search_action)

        # Add show_loaded action
        show_loaded_action = Gio.SimpleAction.new_stateful(
            "show_loaded",
            None,
            GLib.Variant.new_boolean(True)
        )
        show_loaded_action.connect("change-state", self.on_show_loaded_changed)
        self.add_action(show_loaded_action)

        # Add show_log action
        show_log_action = Gio.SimpleAction.new_stateful(
            "show_log",
            None,
            GLib.Variant.new_boolean(False)
        )
        show_log_action.connect("change-state", self.on_show_log_changed)
        self.add_action(show_log_action)

        # Main layout
        self.main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.set_content(self.main_box)

        # Header bar
        header = Adw.HeaderBar()
        self.main_box.append(header)

        # Search button
        self.search_button = Gtk.ToggleButton(icon_name="system-search-symbolic")
        self.search_button.set_tooltip_text("Search services (Ctrl+F)")
        self.search_button.connect("toggled", self.on_search_toggled)
        header.pack_end(self.search_button)

        # Create menu
        menu = Gio.Menu()
        menu.append("New Service", "app.new_service")
        menu.append("Reload Configuration", "app.reload")
        menu.append("Show Only Loaded Units", "win.show_loaded")
        menu.append("Show Log", "win.show_log")
        menu.append("Feedback", "app.feedback")
        menu.append("About", "app.about")

        # Menu button
        menu_button = Gtk.MenuButton()
        menu_button.set_icon_name("open-menu-symbolic")
        menu_button.set_menu_model(menu)
        menu_button.set_tooltip_text("Main menu")
        header.pack_end(menu_button)

        # Create filter buttons in a ribbon
        filter_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        filter_box.add_css_class("toolbar")
        filter_box.add_css_class("red-background")  # Add custom CSS class
        filter_box.set_margin_start(6)
        filter_box.set_margin_end(6)
        filter_box.set_margin_top(6)
        filter_box.set_margin_bottom(6)

        # All services filter
        all_button = Gtk.ToggleButton(label="System")
        all_button.set_tooltip_text("Show all system services")
        all_button.set_active(True)
        all_button.connect("toggled", self.on_filter_changed, "all")
        filter_box.append(all_button)

        # Running services filter
        running_button = Gtk.ToggleButton(label="Running")
        running_button.set_tooltip_text("Show only running services")
        running_button.connect("toggled", self.on_filter_changed, "running")
        filter_box.append(running_button)

        # Inactive services filter
        inactive_button = Gtk.ToggleButton(label="Inactive")
        inactive_button.set_tooltip_text("Show only inactive services")
        inactive_button.connect("toggled", self.on_filter_changed, "inactive")
        filter_box.append(inactive_button)

        # Failed services filter
        failed_button = Gtk.ToggleButton(label="Failed")
        failed_button.set_tooltip_text("Show only failed services")
        failed_button.connect("toggled", self.on_filter_changed, "failed")
        filter_box.append(failed_button)

        # User services filter (moved to end)
        user_button = Gtk.ToggleButton(label="User")
        user_button.set_tooltip_text("Show user services")
        user_button.connect("toggled", self.on_filter_changed, "user")
        filter_box.append(user_button)

        # Store filter buttons for toggling
        self.filter_buttons = {
            "all": all_button,
            "running": running_button,
            "inactive": inactive_button,
            "failed": failed_button,
            "user": user_button
        }

        self.main_box.append(filter_box)

        # Search bar
        self.search_bar = Gtk.SearchBar()
        self.search_entry = Gtk.SearchEntry()
        self.search_entry.set_hexpand(True)
        self.search_entry.connect("search-changed", self.on_search_changed)
        self.search_bar.set_child(self.search_entry)
        self.search_bar.set_key_capture_widget(self)
        self.search_bar.connect_entry(self.search_entry)
        self.main_box.append(self.search_bar)

        # Create paned container for main content and log
        self.paned = Gtk.Paned(orientation=Gtk.Orientation.VERTICAL)
        self.paned.set_wide_handle(True)
        self.paned.set_resize_start_child(False)
        self.paned.set_resize_end_child(True)
        self.paned.set_shrink_start_child(False)
        self.paned.set_shrink_end_child(True)
        self.main_box.append(self.paned)

        # Create main content box (for list)
        content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        content_box.set_vexpand(True)
        self.paned.set_start_child(content_box)

        # Create list box and scrolled window
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        self.list_box = Gtk.ListBox()
        self.list_box.set_selection_mode(Gtk.SelectionMode.NONE)
        self.list_box.add_css_class("boxed-list")
        self.list_box.set_filter_func(self.filter_services)
        scrolled.set_child(self.list_box)

        # Create spinner overlay
        self.spinner_overlay = Gtk.Overlay()
        self.spinner_overlay.set_child(scrolled)

        # Add spinner
        self.spinner = Gtk.Spinner()
        self.spinner.set_size_request(24, 24)
        self.spinner.set_halign(Gtk.Align.CENTER)
        self.spinner.set_valign(Gtk.Align.CENTER)
        self.spinner_overlay.add_overlay(self.spinner)

        # Add spinner box for initial loading
        self.spinner_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        self.spinner_box.set_valign(Gtk.Align.CENTER)
        self.spinner_box.set_vexpand(True)
        
        loading_spinner = Gtk.Spinner()
        loading_spinner.set_size_request(32, 32)
        self.spinner_box.append(loading_spinner)
        
        loading_label = Gtk.Label(label="Loading services...")
        self.spinner_box.append(loading_label)
        
        self.list_box.append(self.spinner_box)
        loading_spinner.start()

        # Add overlay to content box
        content_box.append(self.spinner_overlay)

        # Initialize log viewer variables as None
        self.log_box = None
        self.log_buffer = None
        self.log_view = None

        # Set initial position after window is realized
        self.connect('realize', self.on_window_realize)

        # Load services after window is shown
        GLib.idle_add(self.load_services)

        # Add CSS provider
        css_provider = Gtk.CssProvider()
        css_provider.load_from_data(b"""
            .dark {
                background-color: #303030;
                border-radius: 6px;
                padding: 6px;
            }
            .white {
                color: white;
            }
            .dark-button {
                background: #1a1a1a;
                color: white;
                border: none;
                box-shadow: none;
                text-shadow: none;
                -gtk-icon-shadow: none;
                outline: none;
                border-radius: 4px;
                padding: 8px 12px;
                min-height: 0;
                #min-width: 100px;
                margin: 2px;
            }
            .dark-button:hover {
                background: #2a2a2a;
            }
            .dark-button:active {
                background: #000000;
            }
            row {
                padding: 6px;
            }
            .expander-row {
                padding: 6px;
            }
            .expander-row > box > label {
                font-weight: bold;
                font-size: 1.2em;
            }
            .service-active {
                color: #73d216;
            }
            .service-inactive {
                color: #cc0000;
            }
            .error-text text {
                background-color: #2a0000;
                color: #ff8080;
                padding: 8px;
            }
            .log-text {
                background-color: #000000;
                color: #ffffff;
                border: 1px solid rgba(255, 255, 255, 0.1);
            }
            .log-text text {
                background-color: #000000;
                color: #ffffff;
                padding: 8px;
                font-family: monospace;
            }
            .log-text textview {
                background-color: #000000;
                color: #ffffff;
            }
            .log-text textview text {
                background-color: #000000;
                color: #ffffff;
            }
            paned > separator {
                background-color: rgba(255, 255, 255, 0.1);
                min-height: 6px;
            }
            
            paned > separator:hover {
                background-color: rgba(255, 255, 255, 0.2);
            }
        """)
        
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            css_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        style_manager = Adw.StyleManager.get_default()
        style_manager.set_color_scheme(Adw.ColorScheme.PREFER_DARK)

        # Hide log view by default
        GLib.idle_add(lambda: self.paned.set_position(self.paned.get_allocated_height()))

    @staticmethod
    def is_running_in_flatpak():
        """Check if the application is running inside Flatpak"""
        return os.path.exists("/.flatpak-info")

    def run_host_command(self, cmd):
        """Run a command on the host system, handling Flatpak if needed"""
        if SystemdManagerWindow.is_running_in_flatpak():
            return ["flatpak-spawn", "--host"] + cmd
        return cmd

    def load_services(self):
        """Load systemd services based on current filter"""
        try:
            if self.spinner_box.get_parent():
                self.list_box.remove(self.spinner_box)

            # For user filter, handle separately
            if self.current_filter == "user":
                user_cmd = ["systemctl", "--user", "list-units", "--type=service", "--all", "--no-pager", "--plain"]
                if self.show_only_loaded:
                    user_cmd.extend(["--state=loaded"])
                user_output = subprocess.run(
                    self.run_host_command(user_cmd),
                    capture_output=True,
                    text=True,
                    check=True
                ).stdout
                self.parse_systemctl_output(user_output)
                return

            services = []

            if not self.show_only_loaded:
                # Get all unit files when showing all units
                unit_files_cmd = ["systemctl", "list-unit-files", "--type=service", "--no-pager", "--plain"]
                unit_files_output = subprocess.run(
                    self.run_host_command(unit_files_cmd),
                    capture_output=True,
                    text=True,
                    check=True
                ).stdout

                # Parse unit files to get all services
                for line in unit_files_output.splitlines():
                    if not line.strip() or line.startswith("UNIT FILE"):
                        continue
                    parts = line.split(maxsplit=2)
                    if len(parts) >= 2:
                        unit_name = parts[0]
                        if unit_name.endswith('.service'):
                            services.append({
                                'name': unit_name[:-8],
                                'full_name': unit_name,
                                'load': parts[1],
                                'active': 'inactive',
                                'sub': 'dead',
                                'description': ''
                            })

            # Get active units with their current state
            if self.current_filter == "running":
                cmd = ["systemctl", "list-units", "--type=service", "--state=active", "--no-pager", "--plain"]
            elif self.current_filter == "inactive":
                cmd = ["systemctl", "list-units", "--type=service", "--state=inactive", "--no-pager", "--plain"]
            elif self.current_filter == "failed":
                cmd = ["systemctl", "list-units", "--type=service", "--state=failed", "--no-pager", "--plain"]
            else:
                cmd = ["systemctl", "list-units", "--type=service", "--all", "--no-pager", "--plain"]
                if self.show_only_loaded:
                    cmd.extend(["--state=loaded"])

            units_output = subprocess.run(
                self.run_host_command(cmd),
                capture_output=True,
                text=True,
                check=True
            ).stdout

            # Update or add services from list-units output
            for line in units_output.splitlines():
                if not line.strip() or line.startswith("UNIT"):
                    continue
                parts = line.split(maxsplit=4)
                if len(parts) >= 4:
                    unit_name = parts[0]
                    if unit_name.endswith('.service'):
                        service_data = {
                            'name': unit_name[:-8],
                            'full_name': unit_name,
                            'load': parts[1],
                            'active': parts[2],
                            'sub': parts[3],
                            'description': parts[4] if len(parts) > 4 else ''
                        }
                        # Update existing service or add new one
                        if not self.show_only_loaded:
                            # Update existing service if found
                            for service in services:
                                if service['full_name'] == unit_name:
                                    service.update(service_data)
                                    break
                            else:
                                # Add if not found
                                services.append(service_data)
                        else:
                            services.append(service_data)

            self.all_services = services
            self.refresh_display()
            
        except subprocess.CalledProcessError as e:
            print(f"Error loading services: {e}")
            self.show_error_dialog("Failed to load service information")

    def parse_systemctl_output(self, output):
        """Parse systemctl output"""
        services = []
        for line in output.splitlines():
            if not line.strip() or line.startswith("UNIT") or "not-found" in line:
                continue
                
            parts = line.split(maxsplit=4)
            if len(parts) >= 4:
                unit_name = parts[0]
                if unit_name.endswith('.service'):
                    service_data = {
                        'name': unit_name[:-8],  # Remove '.service' suffix
                        'full_name': unit_name,  # Keep full name for systemctl commands
                        'load': parts[1],
                        'active': parts[2],
                        'sub': parts[3],
                        'description': parts[4] if len(parts) > 4 else ''
                    }
                    services.append(service_data)

        self.all_services = services
        self.refresh_display()

    def create_service_row(self, service_data):
        """Create a row for a service"""
        row = Adw.ExpanderRow(title=service_data['name'])
        
        # Add handler for expand/collapse
        row.connect('notify::expanded', self.on_row_expanded)
        
        # Set the service name as title
        row.set_title(service_data['name'])
        
        # Set the status as subtitle
        status_class = "service-active" if service_data['active'] == "active" else "service-inactive"
        status_text = f"{service_data['active']} ({service_data['sub']})"
        row.set_subtitle(status_text)

        # Details box
        details_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        details_box.set_margin_start(12)
        details_box.set_margin_end(12)
        details_box.set_margin_top(6)
        details_box.set_margin_bottom(6)
        details_box.add_css_class("dark")

        # Add service details
        def create_detail_label(text):
            # Special handling for running state in details
            if "Sub-state: running" in text:
                prefix, _ = text.split("running", 1)
                label = Gtk.Label(xalign=0)
                label.set_markup(f"{prefix}<span foreground='#73d216'>running</span>")
            else:
                label = Gtk.Label(label=text, xalign=0)
            
            label.set_wrap(True)
            label.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
            label.set_hexpand(True)
            label.add_css_class("white")
            return label

        details_box.append(create_detail_label(f"Description: {service_data['description']}"))
        details_box.append(create_detail_label(f"Load: {service_data['load']}"))
        details_box.append(create_detail_label(f"Active: {service_data['active']}"))
        details_box.append(create_detail_label(f"Sub-state: {service_data['sub']}"))
        
        # Get autostart status
        try:
            cmd = ["systemctl", "is-enabled", f"{service_data['name']}.service"]
            if SystemdManagerWindow.is_running_in_flatpak():
                cmd = ["flatpak-spawn", "--host"] + cmd
            autostart = subprocess.run(cmd, capture_output=True, text=True).stdout.strip()
        except:
            autostart = "unknown"
        details_box.append(create_detail_label(f"Autostart: {autostart}"))



        # Add action buttons
        buttons_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        buttons_box.set_margin_top(6)
        
        status_button = Gtk.Button(label="Status")
        status_button.set_tooltip_text("Show detailed service status")
        status_button.connect("clicked", self.on_show_status, service_data['name'])
        status_button.add_css_class("dark-button")
        buttons_box.append(status_button)

        start_button = Gtk.Button(label="Start")
        start_button.connect("clicked", self.on_start_service, service_data['name'])
        start_button.add_css_class("dark-button")
        buttons_box.append(start_button)

        stop_button = Gtk.Button(label="Stop")
        stop_button.connect("clicked", self.on_stop_service, service_data['name'])
        stop_button.add_css_class("dark-button")
        buttons_box.append(stop_button)

        restart_button = Gtk.Button(label="Restart")
        restart_button.connect("clicked", self.on_restart_service, service_data['name'])
        restart_button.add_css_class("dark-button")
        buttons_box.append(restart_button)

        enable_button = Gtk.Button(label="Enable")
        enable_button.connect("clicked", self.on_enable_service, service_data['name'])
        enable_button.add_css_class("dark-button")
        buttons_box.append(enable_button)

        disable_button = Gtk.Button(label="Disable")
        disable_button.connect("clicked", self.on_disable_service, service_data['name'])
        disable_button.add_css_class("dark-button")
        buttons_box.append(disable_button)

        edit_button = Gtk.Button(label="Edit")
        edit_button.set_tooltip_text("Override settings for this unit")
        edit_button.connect("clicked", self.on_edit_service, service_data['name'])
        edit_button.add_css_class("dark-button")
        buttons_box.append(edit_button)

        # Add Log button
        log_button = Gtk.Button(label="Log")
        log_button.set_tooltip_text("Show real-time service logs")
        log_button.connect("clicked", self.on_follow_log, service_data['name'])
        log_button.add_css_class("dark-button")
        buttons_box.append(log_button)

        details_box.append(buttons_box)

        # Add details to row
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.NEVER)
        scrolled.set_child(details_box)
        row.add_row(scrolled)

        # Create button box for actions
        button_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        button_box.set_halign(Gtk.Align.END)
        
        # Add Follow Log button
        follow_log_button = Gtk.Button(label="Follow Log")
        follow_log_button.add_css_class("dark-button")
        follow_log_button.connect("clicked", self.on_follow_log, service_data['name'])
        button_box.append(follow_log_button)

        # Add Log button
        log_button = Gtk.Button(label="Log")
        log_button.add_css_class("dark-button")
        log_button.connect("clicked", self.on_show_log, service_data['name'])
        button_box.append(log_button)

        # Status button
        status_button = Gtk.Button(label="Status")
        status_button.add_css_class("dark-button")
        status_button.connect("clicked", self.on_show_status, service_data['name'])
        button_box.append(status_button)

        return row

    def run_systemctl_command(self, command, service_name):
        """Run a systemctl command with pkexec if needed"""
        try:
            # Get the row and its expanded state
            row = None
            scrolled_window = None
            scroll_value = None
            
            # Find the scrolled window
            for widget in self.main_box:
                if isinstance(widget, Gtk.ScrolledWindow):
                    scrolled_window = widget
                    vadjustment = scrolled_window.get_vadjustment()
                    scroll_value = vadjustment.get_value()
                    break
            
            # Find the row
            for child in self.list_box.observe_children():
                if child.get_title() == service_name:
                    row = child
                    break
            
            was_expanded = row.get_expanded() if row else False
            
            # Check if this is a user service
            is_user_service = self.check_if_user_service(service_name)
            service_name = f"{service_name}.service"
            
            # Build command based on service type
            if is_user_service:
                cmd = ["systemctl", "--user", command, service_name]
            else:
                cmd = ["systemctl", command, service_name]
                if not self.is_root:
                    cmd.insert(0, "pkexec")
            
            # Log the command being executed
            self.log_message(f"Running command: {' '.join(cmd)}")
            
            # Run command and capture output
            result = subprocess.run(
                self.run_host_command(cmd),
                capture_output=True,
                text=True,
                check=True
            )
            
            # Log stdout if any
            if result.stdout:
                self.log_message(f"Output:\n{result.stdout.strip()}")
            
            # Get and log service status
            status_cmd = ["systemctl"]
            if is_user_service:
                status_cmd.append("--user")
            status_cmd.extend(["status", service_name])
            
            status_result = subprocess.run(
                self.run_host_command(status_cmd),
                capture_output=True,
                text=True,
                check=False  # Don't fail on non-zero exit code
            )
            
            # Log status output
            if status_result.stdout:
                self.log_message(f"Status:\n{status_result.stdout.strip()}")
            
            # Use a callback to refresh all services
            def refresh_and_restore():
                self.refresh_data()
                if was_expanded:
                    for child in self.list_box.observe_children():
                        if child.get_title() == service_name[:-8]:
                            child.set_expanded(True)
                            if scrolled_window and scroll_value is not None:
                                GLib.idle_add(lambda: scrolled_window.get_vadjustment().set_value(scroll_value))
                            break
                return False
            
            GLib.timeout_add(1000, refresh_and_restore)
            
        except subprocess.CalledProcessError as e:
            error_msg = f"Command failed: {e}"
            if e.stderr:
                error_msg += f"\nError output:\n{e.stderr.decode()}"
            self.show_error_dialog(error_msg)
            self.log_message(error_msg, "ERROR")

    def on_start_service(self, button, service_name):
        self.run_systemctl_command("start", service_name)

    def on_stop_service(self, button, service_name):
        self.run_systemctl_command("stop", service_name)

    def on_restart_service(self, button, service_name):
        self.run_systemctl_command("restart", service_name)

    def on_enable_service(self, button, service_name):
        self.run_systemctl_command("enable", service_name)

    def on_disable_service(self, button, service_name):
        self.run_systemctl_command("disable", service_name)

    def on_edit_service(self, button, service_name):
        """Open systemctl edit for the service"""
        try:
            # Get the row and its expanded state
            row = None
            for child in self.list_box.observe_children():
                if child.get_title() == service_name:
                    row = child
                    break
            
            was_expanded = row.get_expanded() if row else False
            
            # Check if this is a user service
            is_user_service = self.check_if_user_service(service_name)
            service_name = f"{service_name}.service"
            
            # Build the edit command based on service type
            if is_user_service:
                edit_cmd = f"systemctl --user edit {service_name}; read -p 'Press Enter to close...'"
            else:
                edit_cmd = f"pkexec systemctl edit {service_name}; read -p 'Press Enter to close...'"
            
            terminal = self.get_terminal_command()
            if terminal is None:
                self.show_error_dialog("No suitable terminal emulator found. Please install gnome-terminal, xfce4-terminal, or konsole.")
                return
            
            # Build the complete command
            terminal_cmd = [terminal['binary']]
            terminal_cmd.extend(terminal['args'])
            terminal_cmd.append(edit_cmd)
            
            # Use run_host_command for the complete command
            cmd = self.run_host_command(terminal_cmd)
            
            GLib.spawn_async(
                argv=cmd,
                flags=GLib.SpawnFlags.SEARCH_PATH | GLib.SpawnFlags.DO_NOT_REAP_CHILD,
                child_setup=None,
                user_data=None
            )
            
            # Use a callback to restore expanded state after edit
            def restore_expanded_state():
                if was_expanded:
                    for child in self.list_box.observe_children():
                        if child.get_title() == service_name[:-8]:
                            child.set_expanded(True)
                            break
                return False
            
            GLib.timeout_add(1000, restore_expanded_state)
            
        except GLib.Error as e:
            self.show_error_dialog(f"Failed to edit service: {e.message}")

    def refresh_data(self, *args):
        """Refresh the service data"""
        self.load_services()

    def on_search_toggled(self, button):
        self.search_bar.set_search_mode(button.get_active())

    def on_search_changed(self, entry):
        self.list_box.invalidate_filter()

    def filter_services(self, row):
        """Filter services based on search text and current filter"""
        if not hasattr(row, 'get_title'):
            return True

        # First apply search filter
        show_by_search = True
        if self.search_entry.get_text():
            search_text = self.search_entry.get_text().lower()
            title = row.get_title().lower()
            subtitle = row.get_subtitle().lower()
            show_by_search = search_text in title or search_text in subtitle

        # Then apply status filter
        show_by_status = True
        if self.current_filter != "all":
            subtitle = row.get_subtitle().lower()
            if self.current_filter == "running":
                show_by_status = "running" in subtitle
            elif self.current_filter == "inactive":
                show_by_status = "inactive" in subtitle
            elif self.current_filter == "failed":
                show_by_status = "failed" in subtitle

        return show_by_search and show_by_status

    def show_error_dialog(self, message):
        dialog = Adw.MessageDialog(
            parent=self,
            heading="Error",
            body=message
        )
        dialog.add_response("ok", "_OK")
        dialog.present()

    def refresh_display(self):
        """Update the display with the current service data"""
        while True:
            row = self.list_box.get_first_child()
            if row is None:
                break
            self.list_box.remove(row)

        for service_data in self.all_services:
            row = self.create_service_row(service_data)
            self.list_box.append(row)

    def toggle_search(self, action, param):
        self.search_button.set_active(not self.search_button.get_active())

    def on_filter_changed(self, button, filter_type):
        """Handle filter button toggles"""
        if button.get_active():
            # Deactivate other filter buttons
            for btn_type, btn in self.filter_buttons.items():
                if btn_type != filter_type:
                    btn.set_active(False)
            
            self.current_filter = filter_type
            self.refresh_data()  # Reload with new filter

    def on_daemon_reload(self, button):
        """Reload systemd daemon configuration for both system and user"""
        try:
            self.log_message("Reloading systemd configuration...")
            
            # Reload system daemon
            system_cmd = ["systemctl", "daemon-reload"]
            if not self.is_root:
                system_cmd.insert(0, "pkexec")
            
            self.log_message("Running system daemon reload...")
            result = subprocess.run(
                self.run_host_command(system_cmd),
                capture_output=True,
                text=True,
                check=True
            )
            if result.stdout:
                self.log_message(f"System daemon output:\n{result.stdout.strip()}")
            
            # Reload user daemon
            user_cmd = ["systemctl", "--user", "daemon-reload"]
            self.log_message("Running user daemon reload...")
            result = subprocess.run(
                self.run_host_command(user_cmd),
                capture_output=True,
                text=True,
                check=True
            )
            if result.stdout:
                self.log_message(f"User daemon output:\n{result.stdout.strip()}")
            
            self.log_message("Systemd configuration reload completed successfully")
            self.refresh_data()  # Refresh the service list
            
        except subprocess.CalledProcessError as e:
            error_msg = f"Failed to reload daemon: {e}"
            if e.stderr:
                error_msg += f"\nError output:\n{e.stderr.decode()}"
            self.show_error_dialog(error_msg)
            self.log_message(error_msg, "ERROR")

    def on_show_status(self, button, service_name):
        """Show detailed status of the service"""
        try:
            service_file = f"{service_name}.service"
            is_user_service = self.check_if_user_service(service_name)
            # Simple status command without pkexec, just like running it in terminal
            status_cmd = f"systemctl {'--user ' if is_user_service else ''}status {service_file}; read -p 'Press Enter to close...'"
            
            terminal = self.get_terminal_command()
            if terminal is None:
                self.show_error_dialog("No suitable terminal emulator found. Please install gnome-terminal, xfce4-terminal, or konsole.")
                return
            
            # Build the complete command
            terminal_cmd = [terminal['binary']]
            terminal_cmd.extend(terminal['args'])
            terminal_cmd.append(status_cmd)
            
            # Use run_host_command for the complete command
            cmd = self.run_host_command(terminal_cmd)
            
            GLib.spawn_async(
                argv=cmd,
                flags=GLib.SpawnFlags.SEARCH_PATH | GLib.SpawnFlags.DO_NOT_REAP_CHILD,
                child_setup=None,
                user_data=None
            )
            
        except GLib.Error as e:
            self.show_error_dialog(f"Failed to show service status: {e.message}")

    def check_if_user_service(self, service_name):
        """Helper method to check if a service is a user service"""
        try:
            # Check if service exists in system paths first
            system_paths = [
                "/etc/systemd/system",
                "/usr/lib/systemd/system",
                "/lib/systemd/system",
                "/usr/local/lib/systemd/system"
            ]
            
            service_file = f"{service_name}.service"
            
            # Check system paths first
            for path in system_paths:
                if os.path.exists(os.path.join(path, service_file)):
                    return False  # It's a system service
            
            # If not found in system paths, check user paths
            user_service_path = os.path.expanduser(f"~/.config/systemd/user/{service_file}")
            if os.path.exists(user_service_path):
                return True
            
            # Final check with systemctl --user
            check_cmd = ["systemctl", "--user", "list-unit-files", "--type=service", "--all", "--no-pager", "--plain"]
            result = subprocess.run(
                self.run_host_command(check_cmd),
                capture_output=True,
                text=True,
                check=True
            )
            return any(service_file in line for line in result.stdout.splitlines())
            
        except subprocess.CalledProcessError:
            # If in doubt, assume it's a system service
            return False

    def get_terminal_command(self):
        """Helper function to find an available terminal emulator"""
        terminals = [
            {
                'binary': 'gnome-terminal',
                'args': ['--', 'bash', '-c']
            },
            {
                'binary': 'xfce4-terminal',
                'args': ['-e', 'bash -c']
            },
            {
                'binary': 'konsole',
                'args': ['-e', 'bash -c']
            },
            {
                'binary': 'x-terminal-emulator',
                'args': ['-e', 'bash -c']
            }
        ]
        
        for terminal in terminals:
            try:
                # Use run_host_command to check if terminal exists
                subprocess.run(
                    self.run_host_command(['which', terminal['binary']]),
                    check=True,
                    capture_output=True
                )
                return terminal
            except subprocess.CalledProcessError:
                continue
        return None

    def on_show_log(self, button, service_name):
        """Show service logs in GNOME Logs"""
        try:
            service_file = f"{service_name}.service"
            is_user_service = self.check_if_user_service(service_name)
            
            # Build the command to open GNOME Logs
            if is_user_service:
                # For user services, open user journal
                cmd = ["gnome-logs", "--identifier", service_file]
            else:
                # For system services, might need elevated privileges
                cmd = ["pkexec", "gnome-logs", "--identifier", service_file]
            
            # Use run_host_command for Flatpak compatibility
            cmd = self.run_host_command(cmd)
            
            GLib.spawn_async(
                argv=cmd,
                flags=GLib.SpawnFlags.SEARCH_PATH | GLib.SpawnFlags.DO_NOT_REAP_CHILD,
                child_setup=None,
                user_data=None
            )
            
        except GLib.Error as e:
            self.show_error_dialog(f"Failed to show logs: {e.message}\nPlease make sure GNOME Logs (gnome-logs) is installed.")

    def on_follow_log(self, button, service_name):
        """Show real-time service logs using journalctl -fu"""
        try:
            service_file = f"{service_name}.service"
            is_user_service = self.check_if_user_service(service_name)
            
            # Build the journalctl command
            follow_cmd = f"journalctl {'--user ' if is_user_service else ''}-fu {service_file}"
            
            terminal = self.get_terminal_command()
            if terminal is None:
                self.show_error_dialog("No suitable terminal emulator found. Please install gnome-terminal, xfce4-terminal, or konsole.")
                return
            
            # Build the complete command
            terminal_cmd = [terminal['binary']]
            terminal_cmd.extend(terminal['args'])
            terminal_cmd.append(follow_cmd)
            
            # Use run_host_command for Flatpak compatibility
            cmd = self.run_host_command(terminal_cmd)
            
            GLib.spawn_async(
                argv=cmd,
                flags=GLib.SpawnFlags.SEARCH_PATH | GLib.SpawnFlags.DO_NOT_REAP_CHILD,
                child_setup=None,
                user_data=None
            )
            
        except GLib.Error as e:
            self.show_error_dialog(f"Failed to show service logs: {e.message}")

    def on_show_loaded_changed(self, action, value):
        """Handle show loaded units toggle"""
        action.set_state(value)
        self.show_only_loaded = value.get_boolean()
        
        # Start spinner
        self.spinner.start()
        self.spinner.set_visible(True)
        
        # Use timeout to allow spinner to be shown
        GLib.timeout_add(50, self._delayed_refresh)

    def _delayed_refresh(self):
        """Refresh data and stop spinner"""
        self.refresh_data()
        self.spinner.stop()
        self.spinner.set_visible(False)
        return False

    def on_row_expanded(self, row, param):
        """Handle row expansion to ensure description is visible"""
        if row.get_expanded():
            # Get the scrolled window that contains the list box
            scrolled_window = None
            for widget in self.main_box:
                if isinstance(widget, Gtk.ScrolledWindow):
                    scrolled_window = widget
                    break
            
            if scrolled_window:
                # Get the horizontal adjustment
                hadj = scrolled_window.get_hadjustment()
                # Scroll to the beginning to show the description
                hadj.set_value(0)

    def refresh_and_restore(self, service_name=None):
        """Refresh the service list and restore expanded state"""
        # Store current vertical scroll position
        scrolled = self.list_box.get_parent()
        v_pos = 0
        if isinstance(scrolled, Gtk.ScrolledWindow):
            v_pos = scrolled.get_vadjustment().get_value()
        
        self.load_services()
        
        # Restore vertical position and force scroll to left
        if isinstance(scrolled, Gtk.ScrolledWindow):
            def restore_scroll():
                # Force scroll to left (0 is leftmost position)
                scrolled.get_hadjustment().set_value(0)
                # Restore vertical position
                scrolled.get_vadjustment().set_value(v_pos)
                return False
            # Add a slightly longer delay to ensure content is loaded
            GLib.timeout_add(200, restore_scroll)
        
        if service_name:
            # Wait for the list to be populated
            GLib.timeout_add(200, self._restore_state, service_name)

    def _restore_state(self, service_name):
        """Helper to restore expanded state and scroll position"""
        for row in self.list_box.get_children():
            if isinstance(row, Adw.ExpanderRow) and row.get_title() == service_name:
                row.set_expanded(True)
                # Get the row's parent ScrolledWindow
                scrolled = self.list_box.get_parent()
                if isinstance(scrolled, Gtk.ScrolledWindow):
                    # Scroll to the row vertically
                    adjustment = scrolled.get_vadjustment()
                    row_allocation = row.get_allocation()
                    adjustment.set_value(row_allocation.y)
                    # Ensure horizontal scroll is at left
                    scrolled.get_hadjustment().set_value(0)
                break
        return False

    def clear_log(self, button):
        """Clear the log buffer"""
        self.log_buffer.set_text("")

    def log_message(self, message, level="INFO"):
        """Add a message to the log viewer"""
        # Create log viewer if it doesn't exist
        if self.log_buffer is None:
            self.create_log_viewer()

        end_iter = self.log_buffer.get_end_iter()
        
        # Add timestamp
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        
        # Create tags for different message types if they don't exist
        if not self.log_buffer.get_tag_table().lookup("error"):
            self.log_buffer.create_tag("error", foreground="#ff0000")  # Red for errors
        
        # Insert timestamp
        self.log_buffer.insert(end_iter, f"\n[{timestamp}] ")
        
        # Insert level and message with appropriate color
        if level == "ERROR":
            self.log_buffer.insert_with_tags_by_name(end_iter, f"{level}: ", "error")
            self.log_buffer.insert_with_tags_by_name(end_iter, message, "error")
        else:
            self.log_buffer.insert(end_iter, f"{level}: {message}")
        
        # Add empty lines for better readability
        self.log_buffer.insert(end_iter, "\n\n")
        
        # Get the adjustment for the scrolled window
        scrolled = self.log_view.get_parent()
        if isinstance(scrolled, Gtk.ScrolledWindow):
            adj = scrolled.get_vadjustment()
            # Scroll to bottom by setting value to upper - page_size
            def scroll_to_end():
                adj.set_value(adj.get_upper() - adj.get_page_size())
                return False
            # Use idle_add to ensure scrolling happens after text is rendered
            GLib.idle_add(scroll_to_end)

    def on_window_realize(self, widget):
        """Set initial paned position after window is realized"""
        # Set position to show just the handle (10 pixels from bottom)
        self.paned.set_position(self.get_height() - 10)

    def on_show_log_changed(self, action, value):
        """Handle show log toggle"""
        action.set_state(value)
        show_log = value.get_boolean()
        
        if show_log:
            # Create and show log viewer
            log_box = self.create_log_viewer()
            
            # Set fixed height
            log_box.set_size_request(-1, 120)
            
            # Add to paned
            self.paned.set_end_child(log_box)
            
            # Disable resizing for the log viewer
            self.paned.set_resize_end_child(False)
            self.paned.set_shrink_end_child(False)
            
            # Set position to show exactly 120px
            GLib.idle_add(lambda: self.paned.set_position(self.paned.get_allocated_height() - 120))
        else:
            # Remove log viewer
            self.paned.set_end_child(None)
            
            # Reset paned properties
            self.paned.set_resize_end_child(True)
            self.paned.set_shrink_end_child(True)

    def create_log_viewer(self):
        """Create log viewer on demand"""
        if self.log_box is None:
            # Create log view
            log_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)

            # Add log header
            log_header = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
            log_header.add_css_class("toolbar")

            log_label = Gtk.Label(label="Output Log")
            log_label.add_css_class("heading")
            log_header.append(log_label)

            log_box.append(log_header)

            # Create log text view
            self.log_buffer = GtkSource.Buffer()
            self.log_view = GtkSource.View(buffer=self.log_buffer)
            self.log_view.set_editable(False)
            self.log_view.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
            self.log_view.add_css_class("log-text")

            # Add log view to scrolled window
            log_scroll = Gtk.ScrolledWindow()
            log_scroll.set_vexpand(True)
            log_scroll.set_child(self.log_view)
            log_box.append(log_scroll)

            self.log_box = log_box

        return self.log_box

class ServiceEditor(Gtk.Window):
    def __init__(self, parent):
        super().__init__(title="Create New Service")
        self.set_default_size(800, 600)
        self.set_transient_for(parent)
        self.parent_window = parent  # Store reference to parent window
        
        # Create main box
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.set_child(box)
        
        # Create header bar
        header = Gtk.HeaderBar()
        self.set_titlebar(header)
        
        # Add search button
        search_button = Gtk.ToggleButton(icon_name="system-search-symbolic")
        search_button.set_tooltip_text("Search (Ctrl+F)")
        search_button.connect("toggled", self.on_search_toggled)
        header.pack_end(search_button)
        
        # Add save button
        save_button = Gtk.Button(label="Save")
        save_button.connect("clicked", self.on_save_clicked)
        header.pack_end(save_button)
        
        # Add search bar
        self.search_bar = Gtk.SearchBar()
        self.search_entry = Gtk.SearchEntry()
        self.search_entry.set_hexpand(True)
        self.search_entry.connect("search-changed", self.on_search_changed)
        self.search_bar.set_child(self.search_entry)
        box.append(self.search_bar)
        
        # Create source view
        self.source_view = GtkSource.View()
        self.source_view.set_show_line_numbers(True)
        self.source_view.set_auto_indent(True)
        self.source_view.set_indent_width(2)
        self.source_view.set_insert_spaces_instead_of_tabs(True)
        self.source_view.set_highlight_current_line(True)
        
        # Set up search settings
        self.search_settings = GtkSource.SearchSettings()
        self.search_settings.set_case_sensitive(False)
        self.search_settings.set_wrap_around(True)
        
        self.search_context = GtkSource.SearchContext(
            buffer=self.source_view.get_buffer(),
            settings=self.search_settings
        )
        
        # Set up buffer with service template
        buffer = self.source_view.get_buffer()
        buffer.set_text(self.get_service_template())
        
        # Set up syntax highlighting
        lang_manager = GtkSource.LanguageManager.get_default()
        ini_lang = lang_manager.get_language("ini")
        buffer.set_language(ini_lang)
        
        # Add scrolled window
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        scrolled.set_child(self.source_view)
        box.append(scrolled)
        
        # Set up keyboard shortcuts
        self.search_bar.set_key_capture_widget(self)
        
    def on_search_toggled(self, button):
        """Toggle search bar visibility"""
        self.search_bar.set_search_mode(button.get_active())
        if button.get_active():
            self.search_entry.grab_focus()
    
    def on_search_changed(self, entry):
        """Handle search text changes"""
        search_text = entry.get_text()
        self.search_settings.set_search_text(search_text)
        
        # Highlight first match
        if search_text:
            buffer = self.source_view.get_buffer()
            if not self.search_context.forward(
                buffer.get_start_iter(),
                buffer.get_end_iter(),
                None
            )[0]:
                # No match found
                entry.add_css_class("error")
            else:
                entry.remove_css_class("error")
        else:
            entry.remove_css_class("error")
    
    def get_service_template(self):
        return """[Unit]
Description=My custom service
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/sleep infinity

[Install]
WantedBy=multi-user.target
"""

    def on_save_clicked(self, button):
        buffer = self.source_view.get_buffer()
        text = buffer.get_text(buffer.get_start_iter(), buffer.get_end_iter(), True)
        
        # Create modern file chooser dialog
        dialog = Gtk.FileChooserDialog(
            title="Save Service File",
            transient_for=self,
            action=Gtk.FileChooserAction.SAVE
        )
        
        # Add buttons using modern approach
        dialog.add_button("_Cancel", Gtk.ResponseType.CANCEL)
        dialog.add_button("_Save", Gtk.ResponseType.ACCEPT)
        
        # Set up file filters
        filters = Gtk.FileFilter()
        filters.set_name("Service files")
        filters.add_pattern("*.service")
        dialog.set_filter(filters)
        
        # Set default save location to /etc/systemd/system/
        dialog.set_current_folder(Gio.File.new_for_path("/etc/systemd/system"))
        
        # Suggest default filename
        dialog.set_current_name("myservice.service")
        
        # Show the dialog and handle response
        dialog.connect("response", self._on_save_response, text)
        dialog.present()

    def _on_save_response(self, dialog, response, text):
        if response == Gtk.ResponseType.ACCEPT:
            try:
                file = dialog.get_file()
                if file:
                    file_path = file.get_path()
                    home_dir = os.path.expanduser("~")
                    
                    # Log the save attempt
                    self.parent_window.log_message(f"Creating new service file: {file_path}")
                    
                    # Check if saving to user's home directory or its subdirectories
                    if file_path.startswith(home_dir):
                        # Direct save without pkexec for user directory
                        with open(file_path, 'w') as f:
                            f.write(text)
                        os.chmod(file_path, 0o644)  # Set permissions without pkexec
                        self.parent_window.log_message(f"Service file saved in user directory: {file_path}")
                    else:
                        # Use pkexec for system directories
                        with open('/tmp/temp_service_file', 'w') as temp_file:
                            temp_file.write(text)
                        
                        # Build the mv command
                        cmd = ["pkexec", "mv", "/tmp/temp_service_file", file_path]
                        if SystemdManagerWindow.is_running_in_flatpak():
                            cmd = ["flatpak-spawn", "--host"] + cmd
                        
                        # Execute the command
                        subprocess.run(cmd, check=True)
                        
                        # Set proper permissions
                        chmod_cmd = ["pkexec", "chmod", "644", file_path]
                        if SystemdManagerWindow.is_running_in_flatpak():
                            chmod_cmd = ["flatpak-spawn", "--host"] + chmod_cmd
                        subprocess.run(chmod_cmd, check=True)
                        self.parent_window.log_message(f"Service file saved in system directory: {file_path}")
                    
                    # Show success message with option to start service
                    success_dialog = Adw.MessageDialog(
                        transient_for=self,
                        heading="Success",
                        body=f"Service file saved successfully to {file_path}\nWould you like to start the service now?"
                    )
                    success_dialog.add_response("no", "_No")
                    success_dialog.add_response("yes", "_Yes")
                    success_dialog.set_default_response("no")
                    success_dialog.set_close_response("no")
                    
                    # Store file path for the start service handler
                    self.saved_service_path = file_path
                    success_dialog.connect("response", self._on_start_service_response)
                    success_dialog.present()
                    
            except Exception as e:
                error_dialog = Adw.MessageDialog(
                    transient_for=self,
                    heading="Error",
                    body=f"Error saving file: {str(e)}"
                )
                error_dialog.add_response("ok", "_OK")
                error_dialog.present()
        
        dialog.destroy()

    def _on_start_service_response(self, dialog, response):
        """Handle response to start service question"""
        dialog.destroy()
        
        if response == "yes":
            try:
                # Get service name from path
                service_name = os.path.basename(self.saved_service_path)
                is_user_service = self.saved_service_path.startswith(os.path.expanduser("~"))
                
                # Build start command
                start_cmd = ["systemctl"]
                if is_user_service:
                    start_cmd.append("--user")
                else:
                    start_cmd.insert(0, "pkexec")
                start_cmd.extend(["start", service_name])
                
                # Run the command
                if SystemdManagerWindow.is_running_in_flatpak():
                    start_cmd = ["flatpak-spawn", "--host"] + start_cmd
                subprocess.run(start_cmd, check=True)
                
                # Show success message
                start_success = Adw.MessageDialog(
                    transient_for=self,
                    heading="Success",
                    body=f"Service {service_name} has been started."
                )
                start_success.add_response("ok", "_OK")
                start_success.present()
                
                # Refresh the parent window's service list
                if isinstance(self.parent_window, SystemdManagerWindow):
                    self.parent_window.refresh_data()
                
            except subprocess.CalledProcessError as e:
                error_dialog = Adw.MessageDialog(
                    transient_for=self,
                    heading="Error",
                    body=f"Failed to start service: {e}"
                )
                error_dialog.add_response("ok", "_OK")
                error_dialog.present()
        
        # Close the editor window
        self.destroy()

class SystemdManagerApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id="io.github.mfat.systemdpilot",
                        flags=Gio.ApplicationFlags.FLAGS_NONE)
        self.connect('activate', self.on_activate)
        self.connect('shutdown', self.on_shutdown)
        
        self.set_accels_for_action("win.search", ["<Control>f"])
        self.set_accels_for_action("app.new_service", ["<Control>n"])
        self.set_accels_for_action("app.reload", ["<Control>r"])
        self.set_accels_for_action("win.show_log", ["<Control>l"])
        
        # Add reload action
        reload_action = Gio.SimpleAction.new("reload", None)
        reload_action.connect("activate", self.on_reload_action)
        self.add_action(reload_action)
        
        # Add feedback action
        feedback_action = Gio.SimpleAction.new("feedback", None)
        feedback_action.connect("activate", self.on_feedback_action)
        self.add_action(feedback_action)
        
        about_action = Gio.SimpleAction.new("about", None)
        about_action.connect("activate", self.on_about_action)
        self.add_action(about_action)

        # Add new service action
        new_service_action = Gio.SimpleAction.new("new_service", None)
        new_service_action.connect("activate", self.on_new_service_clicked)
        self.add_action(new_service_action)

    def on_activate(self, app):
        win = SystemdManagerWindow(application=app)
        win.present()

    def on_shutdown(self, app):
        for window in self.get_windows():
            window.close()

    def on_about_action(self, action, param):
        about = Adw.AboutWindow(
            transient_for=self.get_active_window(),
            application_name="systemd Pilot",
            application_icon="system-run",
            developer_name="mFat",
            version=APP_VERSION,
            website="https://github.com/mfat/systemd-pilot",
            license_type=Gtk.License.GPL_3_0,
            developers=["mFat"],
            copyright="© 2024 mFat"
        )
        about.present()

    def on_reload_action(self, action, param):
        """Handle reload action from menu"""
        active_window = self.get_active_window()
        if active_window:
            active_window.on_daemon_reload(None)

    def on_feedback_action(self, action, param):
        """Open feedback URL in default browser"""
        Gtk.show_uri(
            self.get_active_window(),
            "https://github.com/mfat/systemd-pilot/issues",
            Gdk.CURRENT_TIME
        )

    def on_new_service_clicked(self, action, param):
        editor = ServiceEditor(self.get_active_window())
        editor.present()

app = SystemdManagerApp()
app.run(None)
