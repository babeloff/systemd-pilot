# systemd Pilot

systemd Pilot is a desktop application for managing systemd services on GNU/linux systems. it can be described as a gui for systemd or systemctrl. 

![image](https://github.com/user-attachments/assets/85ee68be-aa3e-4291-8435-ef9ee7b8b72f)


## Features
- List system and user services
- Quickly deploy a service using a template
- Filter by running state
- Start, Stop, Restart services, show status
- Create override configuration for any unit file using the edit button
- Easy search. Just start typing and the app will find relevant services
- Lightweight and easy on system resources (just a single Python script)
- Available as deb, rpm, flatpak and AppImage
- Full integration into GNOME desktop (libadwaita)
  
## Download
- Download from the [releases](https://github.com/mfat/systemd-pilot/releases) section the package for your distribution. Alternatively, you can download the flatpak or the executables  `systemd-pilot` or `systemd-pilot-legacy`(for older systems).

- For older systems the Flatpak and the executable are recommended. 

## Requirements

Python modules:
`PyGObject`

In Debian/Ubuntu, you need these packages:
`sudo apt install python3 libgtk-4-1 libadwaita-1-0 libgtksourceview-5-0 gnome-terminal gnome-logs python3-gi python3-gi-cairo gir1.2-gtk-4.0 gir1.2-adw-1 gir1.2-gtksource-5`

For Fedora/RHEL/etc.
`sudo dnf install python3 gtk4 libadwaita gtksourceview5 gnome-terminal gnome-logs`

For Arch linux:
`sudo pacman -S python gtk4 libadwaita gtksourceview5 gnome-terminal gnome-logs`


## Support development
Bitcoin:`bc1qqtsyf0ft85zshsnw25jgsxnqy45rfa867zqk4t`

Doge:`DRzNb8DycFD65H6oHNLuzyTzY1S5avPHHx`

  
