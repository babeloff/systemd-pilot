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
        self.current_filter = "all"  # Track current filter

        # Set up search action
        search_action = Gio.SimpleAction.new("search", None)
        search_action.connect("activate", self.toggle_search)
        self.add_action(search_action)

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
        all_button = Gtk.ToggleButton(label="All")
        all_button.set_active(True)
        all_button.connect("toggled", self.on_filter_changed, "all")
        filter_box.append(all_button)

        # Running services filter
        running_button = Gtk.ToggleButton(label="Running")
        running_button.connect("toggled", self.on_filter_changed, "running")
        filter_box.append(running_button)

        # Inactive services filter
        inactive_button = Gtk.ToggleButton(label="Inactive")
        inactive_button.connect("toggled", self.on_filter_changed, "inactive")
        filter_box.append(inactive_button)

        # Failed services filter
        failed_button = Gtk.ToggleButton(label="Failed")
        failed_button.connect("toggled", self.on_filter_changed, "failed")
        filter_box.append(failed_button)

        # User services filter (moved to end)
        user_button = Gtk.ToggleButton(label="User")
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

        # Create scrolled window
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        self.main_box.append(scrolled)

        # Create list box for services
        self.list_box = Gtk.ListBox()
        self.list_box.set_selection_mode(Gtk.SelectionMode.NONE)
        self.list_box.add_css_class("boxed-list")
        self.list_box.set_filter_func(self.filter_services)
        scrolled.set_child(self.list_box)

        # Add loading spinner
        self.spinner_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        self.spinner_box.set_valign(Gtk.Align.CENTER)
        self.spinner_box.set_vexpand(True)
        
        self.spinner = Gtk.Spinner()
        self.spinner.set_size_request(32, 32)
        self.spinner_box.append(self.spinner)
        
        loading_label = Gtk.Label(label="Loading services...")
        self.spinner_box.append(loading_label)
        
        self.list_box.append(self.spinner_box)
        self.spinner.start()

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
        """)
        
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(),
            css_provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        style_manager = Adw.StyleManager.get_default()
        style_manager.set_color_scheme(Adw.ColorScheme.PREFER_DARK)

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
                user_output = subprocess.run(
                    self.run_host_command(user_cmd),
                    capture_output=True,
                    text=True,
                    check=True
                ).stdout
                self.parse_systemctl_output(user_output)
                return

            services_dict = {}  # Use dictionary to avoid duplicates

            # First get all installed unit files
            unit_files_cmd = ["systemctl", "list-unit-files", "--type=service", "--no-pager", "--plain"]
            unit_files_output = subprocess.run(
                self.run_host_command(unit_files_cmd),
                capture_output=True,
                text=True,
                check=True
            ).stdout

            # Then get active units with their current state
            units_cmd = ["systemctl", "list-units", "--type=service", "--all", "--no-pager", "--plain"]
            if self.current_filter != "all":
                if self.current_filter == "running":
                    units_cmd.extend(["--state=active"])
                elif self.current_filter == "inactive":
                    units_cmd.extend(["--state=inactive"])
                elif self.current_filter == "failed":
                    units_cmd.extend(["--state=failed"])

            units_output = subprocess.run(
                self.run_host_command(units_cmd),
                capture_output=True,
                text=True,
                check=True
            ).stdout

            # Parse unit files first (to get all installed services)
            for line in unit_files_output.splitlines():
                if not line.strip() or line.startswith("UNIT FILE"):
                    continue
                parts = line.split(maxsplit=2)
                if len(parts) >= 2:
                    unit_name = parts[0]
                    if unit_name.endswith('.service'):
                        # Get service description using systemctl show
                        show_cmd = ["systemctl", "show", unit_name, "--property=Description"]
                        try:
                            desc_output = subprocess.run(
                                self.run_host_command(show_cmd),
                                capture_output=True,
                                text=True,
                                check=True
                            ).stdout
                            description = desc_output.strip().split("=", 1)[1] if "=" in desc_output else ""
                        except subprocess.CalledProcessError:
                            description = ""

                        services_dict[unit_name] = {
                            'name': unit_name[:-8],
                            'full_name': unit_name,
                            'load': 'loaded',
                            'active': 'inactive',
                            'sub': 'dead',
                            'description': description
                        }

            # Update with current state from list-units
            for line in units_output.splitlines():
                if not line.strip() or line.startswith("UNIT"):
                    continue
                parts = line.split(maxsplit=4)
                if len(parts) >= 4:
                    unit_name = parts[0]
                    if unit_name.endswith('.service'):
                        description = parts[4] if len(parts) > 4 else services_dict.get(unit_name, {}).get('description', '')
                        services_dict[unit_name] = {
                            'name': unit_name[:-8],
                            'full_name': unit_name,
                            'load': parts[1],
                            'active': parts[2],
                            'sub': parts[3],
                            'description': description
                        }

            # Convert to list and apply filters
            services = list(services_dict.values())
            
            if self.current_filter != "all":
                filtered_services = []
                for service in services:
                    if self.current_filter == "running" and service['active'] == "active":
                        filtered_services.append(service)
                    elif self.current_filter == "inactive" and service['active'] == "inactive":
                        filtered_services.append(service)
                    elif self.current_filter == "failed" and service['sub'] == "failed":
                        filtered_services.append(service)
                services = filtered_services

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
        row = Adw.ExpanderRow()
        
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
            
            # Check if this is a user service by listing all user services
            is_user_service = self.check_if_user_service(service_name)
            
            service_name = f"{service_name}.service"  # Add .service suffix
            
            # Build command based on service type
            if is_user_service:
                cmd = ["systemctl", "--user", command, service_name]
            else:
                cmd = ["systemctl", command, service_name]
                if not self.is_root:
                    cmd.insert(0, "pkexec")
            
            # Run command with Flatpak handling
            subprocess.run(self.run_host_command(cmd), check=True)
            
            # Use a callback to refresh all services but keep the current row expanded and scroll position
            def refresh_and_restore():
                self.refresh_data()
                # Find the row again after refresh and expand it if it was expanded
                if was_expanded:
                    for child in self.list_box.observe_children():
                        if child.get_title() == service_name[:-8]:  # Remove .service suffix
                            child.set_expanded(True)
                            # Restore scroll position
                            if scrolled_window and scroll_value is not None:
                                GLib.idle_add(lambda: scrolled_window.get_vadjustment().set_value(scroll_value))
                            break
                return False
            
            GLib.timeout_add(1000, refresh_and_restore)
            
        except subprocess.CalledProcessError as e:
            self.show_error_dialog(f"Failed to {command} service: {e}")

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
        """Reload systemd daemon configuration"""
        try:
            cmd = ["systemctl", "daemon-reload"]
            if not self.is_root:
                cmd.insert(0, "pkexec")
            
            subprocess.run(cmd, check=True)
            self.refresh_data()  # Refresh the service list
            
        except subprocess.CalledProcessError as e:
            self.show_error_dialog(f"Failed to reload daemon: {e}")

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

class ServiceEditor(Gtk.Window):
    def __init__(self, parent):
        super().__init__(title="Create New Service")
        self.set_default_size(800, 600)
        self.set_transient_for(parent)
        
        # Create main box
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        self.set_child(box)
        
        # Create header bar
        header = Gtk.HeaderBar()
        self.set_titlebar(header)
        
        # Add save button
        save_button = Gtk.Button(label="Save")
        save_button.connect("clicked", self.on_save_clicked)
        header.pack_end(save_button)
        
        # Create source view
        source_view = GtkSource.View()
        source_view.set_show_line_numbers(True)
        source_view.set_auto_indent(True)
        source_view.set_indent_width(2)
        source_view.set_insert_spaces_instead_of_tabs(True)
        source_view.set_highlight_current_line(True)
        
        # Set up buffer with service template
        buffer = source_view.get_buffer()
        buffer.set_text(self.get_service_template())
        
        # Set up syntax highlighting
        lang_manager = GtkSource.LanguageManager.get_default()
        ini_lang = lang_manager.get_language("ini")
        buffer.set_language(ini_lang)
        
        # Add scrolled window
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        scrolled.set_child(source_view)
        box.append(scrolled)
        
        self.source_view = source_view
    
    def get_service_template(self):
        return """[Unit]
Description=My Service Description
# Documentation URL for this service
Documentation=https://example.com/docs

# Dependencies
After=network.target                   # Start after network is up
#Requires=docker.service              # Hard dependency, if needed
#Wants=network-online.target          # Soft dependency
#PartOf=some.target                   # Groups services together
#Conflicts=conflicting.service        # Services that can't run together

[Service]
# Service type (simple, forking, oneshot, notify, dbus, idle)
Type=simple

# User and group to run as (comment out to run as root)
User=username
Group=groupname

# Working directory
#WorkingDirectory=/path/to/working/dir

# Environment variables
#Environment=VAR1=value1
#Environment=VAR2=value2
#EnvironmentFile=/etc/myapp/env

# Main process
ExecStartPre=/bin/mkdir -p /var/run/myapp     # Run before start (optional)
ExecStart=/usr/bin/myapp --option1 --option2   # Main service command
#ExecStop=/usr/bin/myapp --stop               # Custom stop command (optional)

# Restart configuration
Restart=always                        # always, on-failure, on-abnormal, no
RestartSec=3                         # Time to wait before restart
#StartLimitIntervalSec=60            # Time window for start limit
#StartLimitBurst=3                   # Number of restarts allowed in the interval

# Runtime configuration
#Nice=0                              # Process priority (-20 to 19)
#LimitNOFILE=65535                   # File descriptor limit
#TimeoutStartSec=30                  # Start timeout
#TimeoutStopSec=30                   # Stop timeout

# Security
#NoNewPrivileges=true                # Prevent privilege escalation
#ProtectSystem=full                  # Protect system directories
#ProtectHome=true                    # Protect home directories
#ReadOnlyDirectories=/etc           # Make directories read-only
#ReadWriteDirectories=/var/lib/myapp # Allow write access to specific dirs

[Install]
# Where to install the service (common targets: multi-user.target, graphical.target)
WantedBy=multi-user.target

# For user services, use:
#WantedBy=default.target
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
                    
                    # Check if saving to user's home directory or its subdirectories
                    if file_path.startswith(home_dir):
                        # Direct save without pkexec for user directory
                        with open(file_path, 'w') as f:
                            f.write(text)
                        os.chmod(file_path, 0o644)  # Set permissions without pkexec
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
                    
                    # Show success message
                    success_dialog = Adw.MessageDialog(
                        transient_for=self,
                        heading="Success",
                        body=f"Service file saved successfully to {file_path}\nDon't forget to reload systemd daemon to apply changes."
                    )
                    success_dialog.add_response("ok", "_OK")
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

class SystemdManagerApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id="io.github.mfat.systemdpilot",
                        flags=Gio.ApplicationFlags.FLAGS_NONE)
        self.connect('activate', self.on_activate)
        self.connect('shutdown', self.on_shutdown)
        
        self.set_accels_for_action("win.search", ["<Control>f"])
        self.set_accels_for_action("app.new_service", ["<Control>n"])
        self.set_accels_for_action("app.reload", ["<Control>r"])
        
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
