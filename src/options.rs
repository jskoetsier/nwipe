/*
 *  options.rs: Command line options processing for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion: 2023
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Nwipe options structure
#[derive(Debug, Clone)]
pub struct NwipeOptions {
    /// Automatically wipe all devices, bypassing the GUI.
    pub autonuke: bool,

    /// Exclude mounted partitions.
    pub exclude_mounted: bool,

    /// Run without a GUI.
    pub nogui: bool,

    /// Use the modern GUI interface.
    pub modern_gui: bool,

    /// Don't wait for a key before exiting.
    pub nowait: bool,

    /// Don't install signal handlers.
    pub nosignals: bool,

    /// Power off system when wipe completed.
    pub autopoweroff: bool,

    /// Verbose output.
    pub verbose: bool,

    /// The PRNG algorithm to use.
    pub prng: String,

    /// The wipe method to use.
    pub method: String,

    /// The number of times to run the method.
    pub rounds: i32,

    /// Verify the wipe.
    pub verify: bool,

    /// Device names to wipe.
    pub device_names: Vec<String>,
}

impl Default for NwipeOptions {
    fn default() -> Self {
        Self {
            autonuke: false,
            exclude_mounted: false,
            nogui: false,
            modern_gui: true,  // Default to modern GUI
            nowait: false,
            nosignals: false,
            autopoweroff: false,
            verbose: false,
            prng: "isaac".to_string(),
            method: "ops2".to_string(),
            rounds: 1,
            verify: true,
            device_names: Vec::new(),
        }
    }
}

/// Command line arguments for nwipe
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Automatically wipe all devices, bypassing the GUI
    #[clap(short = 'a', long)]
    autonuke: bool,

    /// Exclude mounted partitions
    #[clap(short = 'e', long)]
    exclude_mounted: bool,

    /// Run without a GUI
    #[clap(short = 'g', long)]
    nogui: bool,

    /// Use the traditional terminal UI instead of the modern GUI
    #[clap(short = 't', long)]
    traditional_ui: bool,

    /// Don't wait for a key before exiting
    #[clap(short = 'h', long)]
    nowait: bool,

    /// Don't install signal handlers
    #[clap(short = 'l', long)]
    nosignals: bool,

    /// Power off system when wipe completed
    #[clap(short = 'p', long)]
    autopoweroff: bool,

    /// Verbose output
    #[clap(short = 'v', long)]
    verbose: bool,

    /// The PRNG algorithm to use
    #[clap(short = 'P', long, default_value = "isaac")]
    prng: String,

    /// The wipe method to use
    #[clap(short = 'm', long, default_value = "ops2")]
    method: String,

    /// The number of times to run the method
    #[clap(short = 'r', long, default_value_t = 1)]
    rounds: i32,

    /// Verify the wipe
    #[clap(short = 'V', long)]
    verify: bool,

    /// Device names to wipe
    #[clap(value_name = "DEVICE")]
    device_names: Vec<String>,
}

/// Parse command line options
pub fn parse_options() -> NwipeOptions {
    let args = Args::parse();

    NwipeOptions {
        autonuke: args.autonuke,
        exclude_mounted: args.exclude_mounted,
        nogui: args.nogui,
        modern_gui: !args.traditional_ui && !args.nogui, // Use modern GUI if not traditional UI and not nogui
        nowait: args.nowait,
        nosignals: args.nosignals,
        autopoweroff: args.autopoweroff,
        verbose: args.verbose,
        prng: args.prng,
        method: args.method,
        rounds: args.rounds,
        verify: args.verify,
        device_names: args.device_names,
    }
}
