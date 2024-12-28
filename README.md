# systemd Pilot

systemd Pilot is a desktop application for managing systemd services on GNU/linux systems. it can be described as a gui for systemd or systemctrl. 

![image](https://github.com/user-attachments/assets/85ee68be-aa3e-4291-8435-ef9ee7b8b72f)


## Features
- List system services
- Quickly deploy a new service using a template
- Start, Stop, Restart services, show status
- Easy search. Just start typing and the app will find relevant services
- Lightweight and easy on system resources (just a single Python script)
  
## Download
- Download from the [releases](https://github.com/mfat/systemd-pilot/releases) section the package for your distribution. Alternatively, you can download the flatpak or the executables  `systemd-pilot` or `systemd-pilot-legacy`(for older systems).

- For older systems the Flatpak and the executable are recommended. 

## Requirements
If you prefer to run the python script directly, here are the requirements:

Python modules:
- PyGObject>=3.42.0
- paramiko>=3.0.0
- keyring>=24.0.0
- rich>=13.0.0
- PyYAML

Debian packages:
- `sudo apt install \
    python3-gi \
    python3-gi-cairo \
    gir1.2-gtk-3.0 \
    gir1.2-gtksource-4 \
    python3-paramiko \
    python3-yaml \
    python3-keyring`

  


## Support development
Bitcoin:`bc1qqtsyf0ft85zshsnw25jgsxnqy45rfa867zqk4t`

Doge:`DRzNb8DycFD65H6oHNLuzyTzY1S5avPHHx`

  
