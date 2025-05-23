# nwipe - Secure Disk Eraser (Rust Edition)

A secure disk wiping utility implemented in Rust, providing multiple wiping methods to ensure your data cannot be recovered.

**Maintained by:** Sebastiaan Koetsier (2025)

**Original concept:** Darik Horn (nwipe/dwipe) and Andy Beverley (modifications)

## Overview

nwipe is a powerful command-line utility that securely erases disks using various methods to ensure data cannot be recovered. This Rust implementation provides the same functionality as the original C version but with the benefits of Rust's memory safety and modern programming features.

## Features

- Multiple wiping methods:
  - OPS-II (DoD 5220.22-M)
  - DoD 5220.22-M
  - Gutmann (35 passes)
  - Random data
  - Zero fill
- Multiple PRNG implementations:
  - ISAAC
  - MT19937 (Mersenne Twister)
  - Standard library RNG
- Verification of wiped data
- Two user interface options:
  - Modern graphical user interface (GUI)
  - Traditional terminal user interface (TUI)
- Detailed logging
- Support for multiple rounds of wiping

## Installation

### Prerequisites

- Rust and Cargo (1.70.0 or newer recommended)
- Linux/Unix-based operating system (for disk access)
- Root privileges (for disk wiping operations)

### Installing Rust and Cargo

If you don't have Rust installed, the easiest way to install it is using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the on-screen instructions to complete the installation. Once installed, make sure to source the environment:

```bash
source "$HOME/.cargo/env"
```

### Installing nwipe

#### Option 1: From source (recommended)

1. Clone the repository:
   ```bash
   git clone https://github.com/sebastiaankoetsier/nwipe.git
   cd nwipe
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Install the binary (optional):
   ```bash
   sudo cp target/release/nwipe /usr/local/bin/
   ```

#### Option 2: Using Cargo

```bash
cargo install --git https://github.com/sebastiaankoetsier/nwipe.git
```

#### Option 3: Download pre-built binary

Check the releases page for pre-built binaries for common platforms.

## Usage

nwipe requires root privileges to access and wipe disks:

```
sudo nwipe [OPTIONS] [DEVICE...]
```

### Options

- `-a, --autonuke`: Automatically wipe all devices, bypassing the GUI
- `-e, --exclude-mounted`: Exclude mounted partitions
- `-g, --nogui`: Run without any GUI (must be used with --autonuke)
- `-t, --traditional-ui`: Use the traditional terminal UI instead of the modern GUI
- `-h, --nowait`: Don't wait for a key before exiting
- `-l, --nosignals`: Don't install signal handlers
- `-p, --autopoweroff`: Power off system when wipe completed
- `-v, --verbose`: Verbose output
- `-P, --prng <PRNG>`: The PRNG algorithm to use (default: "isaac")
- `-m, --method <METHOD>`: The wipe method to use (default: "ops2")
- `-r, --rounds <ROUNDS>`: The number of times to run the method (default: 1)
- `-V, --verify`: Verify the wipe

### Examples

Wipe a specific device (uses modern GUI by default):

```bash
sudo nwipe /dev/sdb
```

Use the traditional terminal UI instead of the modern GUI:

```bash
sudo nwipe --traditional-ui /dev/sdb
```

Automatically wipe all devices:

```bash
sudo nwipe --autonuke
```

Use a specific wiping method:

```bash
sudo nwipe --method dod /dev/sdb
```

Run without any UI in automated environments:

```bash
sudo nwipe --autonuke --nogui --method zero
```

Run with modern GUI and specific options:

```bash
sudo nwipe --method gutmann --rounds 2 --verify
```

## User Interfaces

### Modern GUI

The modern graphical user interface provides an intuitive way to interact with nwipe. It features:

- Device selection with detailed information
- Progress monitoring with visual indicators
- Method and PRNG selection dropdowns
- Configuration options
- Real-time logging display
- Confirmation dialogs for safety

This is the default interface when running nwipe without any UI-related options.

### Traditional TUI

The traditional terminal user interface provides a lightweight, ncurses-based interface that works well in terminal environments. Use this interface with the `--traditional-ui` option.

## Wiping Methods

### OPS-II (DoD 5220.22-M)

This method performs three rounds of wiping:
1. Write zeros
2. Write ones
3. Write random data

### DoD 5220.22-M

This method performs a single round of:
1. Write zeros
2. Write ones
3. Write random data

### Gutmann

This method performs 35 passes with various patterns, as described by Peter Gutmann.

### Random

This method performs a single pass of random data.

### Zero

This method performs a single pass of zeros.

## Safety Warnings

- **IMPORTANT**: nwipe will permanently destroy all data on the selected disks. There is NO RECOVERY possible after wiping.
- Always double-check device names before wiping.
- Never wipe your system disk while the system is running from it.
- Consider disconnecting important drives before running nwipe to prevent accidental data loss.

## Troubleshooting

### Common Issues

- **Permission denied**: Make sure you're running nwipe with sudo or as root.
- **Device not found**: Verify the device path is correct and the device is connected.
- **Cannot wipe mounted device**: Unmount the device first or use the --exclude-mounted option.

### Logs

nwipe logs are stored in `/var/log/nwipe.log` and can be useful for diagnosing issues.

## License

This program is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
