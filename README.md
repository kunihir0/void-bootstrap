# void-bootstrap

[![Build Status](https://github.com/kunihir0/void-bootstrap/actions/workflows/build.yml/badge.svg)](https://github.com/kunihir0/void-bootstrap/actions/workflows/build.yml)

## About
`void-bootstrap` is an automated, interactive command-line installer for Void Linux. It provides a guided installation process from partitioning the disk to a fully bootable system.

## What it does
- Guides the user through disk partitioning and formatting.
- Mounts necessary partitions (Root, EFI, Btrfs subvolumes if applicable).
- Installs the base Void Linux system and essential packages (including GPU drivers) using `xbps-install`.
- Configures the chroot environment (hostname, timezone, locale, fstab).
- Installs and configures the GRUB bootloader.
- Sets up the root password, primary user account, and standard groups/services.


## Build Instructions
Ensure you have the Rust toolchain installed.

1. Clone the repository:
   ```sh
   git clone https://github.com/kunihir0/void-bootstrap.git
   cd void-bootstrap
   ```
2. Build the project in release mode:
   ```sh
   cargo build --release
   ```
3. The compiled binary will be located at `target/release/void_bootstrap`.

## Contributing
Contributions are welcome. Please submit a pull request or open an issue to discuss proposed changes.
