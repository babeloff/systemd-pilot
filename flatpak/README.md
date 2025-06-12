# Flatpak Packaging for systemd Pilot

This directory contains all the files necessary to package systemd Pilot as a Flatpak application.

## Files Overview

- **`io.github.mfat.systemdpilot.yml`** - Main Flatpak manifest defining the build process
- **`flathub.json`** - Flathub-specific configuration and metadata
- **`generated-sources.json`** - Auto-generated Cargo dependency sources for offline builds
- **`README.md`** - This documentation file

The manifest references files from the `../data/` directory for desktop integration.

## Building the Flatpak

### Prerequisites

Install the required tools:

```bash
# Install flatpak-builder
sudo apt install flatpak-builder

# Add Flathub repository
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# Install required runtime and SDK
flatpak install flathub org.gnome.Platform//47 org.gnome.Sdk//47
```

### Build Commands

From the project root directory:

```bash
# Quick build check (no actual building)
pixi run flatpak-check

# Build the application
pixi run flatpak-build

# Build and install locally
pixi run flatpak-install

# Run the Flatpak version
pixi run flatpak-run

# Create a distributable bundle
pixi run flatpak-bundle
```

### Manual Build Process

If you prefer to build manually:

```bash
# Build the Flatpak
flatpak-builder build flatpak/io.github.mfat.systemdpilot.yml

# Install locally
flatpak-builder --user --install --force-clean build flatpak/io.github.mfat.systemdpilot.yml

# Run the installed application
flatpak run io.github.mfat.systemdpilot
```

## Development Workflow

### Testing Changes

1. Make your changes to the Rust code
2. Test locally with `cargo run`
3. Build and test the Flatpak version:
   ```bash
   pixi run flatpak-build
   pixi run flatpak-run
   ```

### Updating Dependencies

When Cargo dependencies change:

1. Update `Cargo.toml` as needed
2. Regenerate the sources file:
   ```bash
   pixi run flatpak-generate-sources
   ```
3. Test the build:
   ```bash
   pixi run flatpak-build
   ```

## Flatpak Manifest Structure

The manifest (`io.github.mfat.systemdpilot.yml`) defines:

- **Runtime**: GNOME Platform 47
- **SDK**: GNOME SDK 47 with Rust extension
- **Permissions**: System access for systemd management, SSH for remote hosts
- **Build Process**: Rust/Cargo-based build with offline dependency fetching
- **Data Files**: Desktop integration files installed from `../data/` directory

### Key Permissions

- `--system-talk-name=org.freedesktop.systemd1` - Access to systemd
- `--talk-name=org.freedesktop.secrets` - Secure credential storage
- `--filesystem=home/.ssh:ro` - Read SSH keys
- `--share=network` - Network access for SSH connections

## Submission to Flathub

### Initial Submission

1. Fork the [Flathub repository](https://github.com/flathub/flathub)
2. Create a new repository for the application
3. Copy the files from this directory
4. Submit a pull request

### Updates

For application updates:

1. Update the manifest with new version information
2. Update `generated-sources.json` if dependencies changed
3. Update desktop integration files in `../data/` if needed
4. Test the build locally
5. Submit pull request to the Flathub app repository

## Troubleshooting

### Common Issues

**Build fails with missing dependencies:**
- Ensure `generated-sources.json` is up to date
- Check that all system dependencies are listed in the manifest

**Runtime errors:**
- Verify permissions in the manifest
- Check that all required files are installed correctly

**SSH/systemd access issues:**
- Ensure the proper D-Bus permissions are granted
- Verify file system access permissions

### Debug Build

To debug issues:

```bash
# Build with verbose output
flatpak-builder --verbose build flatpak/io.github.mfat.systemdpilot.yml

# Shell into the build environment
flatpak-builder --run build flatpak/io.github.mfat.systemdpilot.yml bash
```

## CI/CD Integration

The pre-push git hook automatically validates the Flatpak manifest. For CI systems:

```yaml
# Example GitHub Actions step
- name: Validate Flatpak
  run: |
    sudo apt-get install flatpak-builder
    flatpak-builder --dry-run --disable-download build flatpak/io.github.mfat.systemdpilot.yml
```

## Resources

- [Flatpak Documentation](https://docs.flatpak.org/)
- [Flathub Submission Guidelines](https://github.com/flathub/flathub/wiki)
- [GNOME Runtime Documentation](https://docs.flatpak.org/en/latest/available-runtimes.html#gnome)
- [Rust Applications on Flatpak](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo)