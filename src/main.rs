/*
 *  main.rs:  Darik's Wipe - Rust implementation.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion: 2023
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 *
 *  This program is distributed in the hope that it will be useful, but WITHOUT
 *  ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 *  FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
 *  details.
 *
 *  You should have received a copy of the GNU General Public License along with
 *  this program; if not, write to the Free Software Foundation, Inc.,
 *  51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 *
 */

mod context;
mod device;
mod gui;
mod gui_app;
mod logging;
mod method;
mod options;
mod prng;
mod version;

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use clap::Parser;
use log::{error, info, warn};
use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use nix::unistd::close;
use signal_hook::{consts::*, iterator::Signals};

use crate::context::{NwipeContext, SelectStatus};
use crate::device::device_scan;
use crate::gui::gui_init;
use crate::logging::nwipe_log;
use crate::options::{NwipeOptions, parse_options};

// Global variables
static mut TERMINATE_SIGNAL: bool = false;
static mut USER_ABORT: bool = false;

const NWIPE_KNOB_ENTROPY: &str = "/dev/urandom";
const NWIPE_KNOB_SLEEP: u8 = 1;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line options
    let options = parse_options();

    // Initialize logging
    logging::init_logging(options.verbose);

    // Check if we should use the modern GUI
    if options.modern_gui {
        // Run the modern GUI application
        nwipe_log(logging::LogLevel::Info, "Starting modern GUI interface");
        return match gui_app::run_gui() {
            Ok(_) => Ok(()),
            Err(e) => {
                nwipe_log(logging::LogLevel::Error, &format!("GUI error: {}", e));
                Err(Box::new(e))
            }
        };
    }

    // If we're here, we're using the traditional TUI
    // The array of pointers to enumerated contexts
    let mut contexts = Vec::new();

    // Scan for devices or use provided device names
    let nwipe_enumerated = if options.device_names.is_empty() {
        // Scan for devices
        match device_scan(&mut contexts) {
            Ok(count) => {
                if count == 0 {
                    nwipe_log(logging::LogLevel::Info, "Storage devices not found.");
                    cleanup();
                    return Ok(());
                } else {
                    nwipe_log(logging::LogLevel::Info, &format!("Automatically enumerated {} devices.", count));
                    count
                }
            },
            Err(e) => {
                nwipe_log(logging::LogLevel::Error, &format!("Error scanning devices: {}", e));
                cleanup();
                return Ok(());
            }
        }
    } else {
        // Use provided device names
        match device::device_get(&mut contexts, &options.device_names) {
            Ok(count) => {
                if count == 0 {
                    nwipe_log(logging::LogLevel::Error, "Devices not found. Check you're not excluding drives unnecessarily.");
                    println!("No drives found");
                    cleanup();
                    return Ok(());
                }
                count
            },
            Err(e) => {
                nwipe_log(logging::LogLevel::Error, &format!("Error getting devices: {}", e));
                cleanup();
                return Ok(());
            }
        }
    };

    if unsafe { TERMINATE_SIGNAL } {
        cleanup();
        return Ok(());
    }

    // Log system information
    logging::log_sysinfo();

    // The array of contexts that will actually be wiped
    let mut selected_contexts = Vec::new();

    // Open the entropy source
    let entropy_result = open(NWIPE_KNOB_ENTROPY, OFlag::O_RDONLY, Mode::empty());
    let nwipe_entropy = match entropy_result {
        Ok(fd) => fd,
        Err(e) => {
            nwipe_log(logging::LogLevel::Fatal, &format!("Unable to open entropy source {}: {}", NWIPE_KNOB_ENTROPY, e));
            cleanup();
            return Ok(());
        }
    };

    nwipe_log(logging::LogLevel::Notice, &format!("Opened entropy source '{}'.", NWIPE_KNOB_ENTROPY));

    // Set up signal handling
    let mut signals = Signals::new(&[SIGHUP, SIGTERM, SIGQUIT, SIGINT, SIGUSR1])?;
    let signal_thread = thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGUSR1 => {
                    // Log current status
                    // TODO: Implement status logging
                },
                SIGHUP | SIGINT | SIGQUIT | SIGTERM => {
                    // Set termination flag
                    unsafe {
                        TERMINATE_SIGNAL = true;
                        USER_ABORT = true;
                    }
                    break;
                },
                _ => {},
            }
        }
    });

    // Set specific nwipe options for each device
    for context in &mut contexts {
        // Set the entropy source
        context.entropy_fd = nwipe_entropy;

        // Set selection status based on autonuke option
        if options.autonuke {
            context.select = SelectStatus::True;
        } else {
            context.select = SelectStatus::False;
        }

        // Set the PRNG implementation
        context.prng = options.prng.clone();
        // Initialize PRNG state and seed
        // TODO: Implement PRNG initialization
    }

    // Start the UI interface if not in nogui mode
    if !options.nogui {
        gui_init();
    }

    // Handle device selection
    if options.autonuke {
        if !options.nogui {
            gui::gui_options();
        }
    } else {
        if options.nogui {
            println!("--nogui option must be used with autonuke option");
            cleanup();
            return Ok(());
        } else {
            gui::gui_select(nwipe_enumerated, &mut contexts);
        }
    }

    // Count and collect selected contexts
    let mut nwipe_selected = 0;
    for context in &contexts {
        if context.select == SelectStatus::True {
            selected_contexts.push(context.clone());
            nwipe_selected += 1;
        }
    }

    // Start wiping threads if user hasn't aborted
    let mut wipe_threads_started = false;
    let mut thread_handles = Vec::new();

    if !unsafe { USER_ABORT } {
        for context in &mut selected_contexts {
            // Initialize context for wiping
            context.spinner_idx = 0;
            context.start_time = 0;
            context.end_time = 0;
            context.wipe_status = -1;

            // Open the device for reads and writes
            match File::options().read(true).write(true).open(&context.device_name) {
                Ok(file) => {
                    context.device_fd = file.as_raw_fd();

                    // Get device information
                    // TODO: Implement device stat and size retrieval

                    // Print serial number if available
                    if !context.identity.serial_no.is_empty() {
                        nwipe_log(logging::LogLevel::Notice,
                                 &format!("{} has serial number {}", context.device_name, context.identity.serial_no));
                    }

                    // Get device sector and block size
                    // TODO: Implement ioctl calls for device information

                    // Get device size
                    // TODO: Implement device size retrieval

                    // Start wiping thread
                    let context_clone = context.clone();
                    let handle = thread::spawn(move || {
                        // Call the selected wiping method
                        method::run_method(&context_clone);
                    });

                    thread_handles.push(handle);
                    wipe_threads_started = true;
                },
                Err(e) => {
                    nwipe_log(logging::LogLevel::Warning,
                             &format!("Unable to open device '{}': {}", context.device_name, e));
                    context.select = SelectStatus::Disabled;
                    continue;
                }
            }
        }
    }

    // Start GUI status thread if not in nogui mode
    let gui_thread = if !options.nogui {
        let selected_contexts_clone = selected_contexts.clone();
        Some(thread::spawn(move || {
            gui::gui_status(&selected_contexts_clone, nwipe_selected);
        }))
    } else {
        None
    };

    // Wait for all wiping threads to finish
    let mut i = 0;
    while i < nwipe_selected && !unsafe { TERMINATE_SIGNAL } {
        if i == nwipe_selected {
            break;
        }

        if selected_contexts[i].wipe_status != 0 {
            i = 0;
        } else {
            i += 1;
            continue;
        }
        thread::sleep(Duration::from_secs(1));
    }

    // Wait for user input if not in nowait mode and not set to autopoweroff
    if !unsafe { TERMINATE_SIGNAL } && !options.nowait && !options.autopoweroff {
        loop {
            if unsafe { TERMINATE_SIGNAL } {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    if options.verbose {
        nwipe_log(logging::LogLevel::Info, "Exit in progress");
    }

    // Request cancellation of wipe threads
    for (i, handle) in thread_handles.iter().enumerate() {
        if options.verbose {
            nwipe_log(logging::LogLevel::Info,
                     &format!("Requesting wipe thread cancellation for {}", selected_contexts[i].device_name));
            nwipe_log(logging::LogLevel::Info, "Please wait..");
        }
        // TODO: Implement thread cancellation
    }

    // Kill the GUI thread
    if let Some(handle) = gui_thread {
        if options.verbose {
            nwipe_log(logging::LogLevel::Info, "Cancelling the GUI thread.");
        }

        // Wait for GUI thread to finish
        if let Err(e) = handle.join() {
            nwipe_log(logging::LogLevel::Warning, "Error when waiting for GUI thread to cancel.");
        }

        if options.verbose {
            nwipe_log(logging::LogLevel::Info, "GUI compute_stats thread has been cancelled");
        }
    }

    // Release the GUI
    if !options.nogui {
        gui::gui_free();
    }

    // Wait for wipe threads to finish
    for (i, handle) in thread_handles.iter().enumerate() {
        if let Err(e) = handle.join() {
            nwipe_log(logging::LogLevel::Warning, "Error when waiting for wipe thread to cancel.");
        }

        if options.verbose {
            nwipe_log(logging::LogLevel::Info,
                     &format!("Wipe thread for device {} has been cancelled", selected_contexts[i].device_name));
        }

        // Close device file descriptor
        close(selected_contexts[i].device_fd).unwrap_or_else(|e| {
            nwipe_log(logging::LogLevel::Warning,
                     &format!("Error closing device {}: {}", selected_contexts[i].device_name, e));
        });
    }

    // Check for errors and set return status
    let mut return_status = 0;

    if !wipe_threads_started {
        // Zero each selected drive result flag if no wipes were started
        for context in &mut selected_contexts {
            context.result = 0;
        }
    } else {
        // Check for non-fatal errors
        for context in &selected_contexts {
            if context.result > 0 {
                nwipe_log(logging::LogLevel::Fatal,
                         &format!("Nwipe exited with non fatal errors on device = {}", context.device_name));
                return_status = 1;
            }
        }

        // Check for fatal errors
        for context in &selected_contexts {
            if context.result < 0 {
                nwipe_log(logging::LogLevel::Error,
                         &format!("Nwipe exited with fatal errors on device = {}", context.device_name));
                return_status = -1;
            }
        }
    }

    // Generate and send the drive status summary to the log
    logging::log_summary(&selected_contexts, nwipe_selected);

    if return_status == 0 {
        nwipe_log(logging::LogLevel::Info, "Nwipe successfully exited.");
    }

    cleanup();

    check_for_autopoweroff(&options);

    Ok(())
}

fn cleanup() -> i32 {
    // TODO: Implement cleanup functionality
    // Print logs held in memory
    // Deallocate memory used by logging

    0
}

fn check_for_autopoweroff(options: &NwipeOptions) {
    if options.autopoweroff {
        let cmd = "shutdown -P +1 \"System going down in one minute\"";
        match Command::new("sh").arg("-c").arg(cmd).output() {
            Ok(_) => {},
            Err(_) => {
                nwipe_log(logging::LogLevel::Info, &format!("Failed to autopoweroff with command: {}", cmd));
            }
        }
    }
}
