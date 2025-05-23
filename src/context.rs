/*
 *  context.rs: Context structure for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion: 2023
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

use std::os::unix::io::RawFd;

/// The status of a device selection.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectStatus {
    /// Device is not selected.
    False,
    /// Device is selected.
    True,
    /// Device is selected by inference.
    TrueParent,
    /// Device is disabled.
    Disabled,
}

/// The type of the current pass.
#[derive(Debug, Clone, PartialEq)]
pub enum PassType {
    /// Not running any pass.
    None,
    /// Writing a pattern.
    Write,
    /// Verifying a pattern.
    Verify,
    /// The final pass.
    FinalBlank,
    /// The final OPS-II pass.
    FinalOps2,
}

/// Device identity information.
#[derive(Debug, Clone)]
pub struct DeviceIdentity {
    /// The device model.
    pub model_no: String,
    /// The device serial number.
    pub serial_no: String,
    /// The firmware revision.
    pub firmware_rev: String,
}

impl Default for DeviceIdentity {
    fn default() -> Self {
        Self {
            model_no: String::new(),
            serial_no: String::new(),
            firmware_rev: String::new(),
        }
    }
}

/// PRNG seed structure.
#[derive(Debug, Clone)]
pub struct PrngSeed {
    /// The length of the seed.
    pub length: usize,
    /// The seed data.
    pub s: Vec<u8>,
}

impl Default for PrngSeed {
    fn default() -> Self {
        Self {
            length: 0,
            s: Vec::new(),
        }
    }
}

/// The main context structure for nwipe.
#[derive(Debug, Clone)]
pub struct NwipeContext {
    /// The device name.
    pub device_name: String,
    /// The device file descriptor.
    pub device_fd: RawFd,
    /// The device size in bytes.
    pub device_size: u64,
    /// The device sector size in bytes.
    pub device_sector_size: u64,
    /// The device block size in bytes.
    pub device_block_size: i32,
    /// The device identity information.
    pub identity: DeviceIdentity,
    /// The entropy source file descriptor.
    pub entropy_fd: RawFd,
    /// The PRNG implementation.
    pub prng: String,
    /// The PRNG seed.
    pub prng_seed: PrngSeed,
    /// The PRNG state.
    pub prng_state: usize,
    /// The selection status of this device.
    pub select: SelectStatus,
    /// The number of patterns that will be written to the device.
    pub round_count: i32,
    /// The current pattern that is being written to the device.
    pub round_working: i32,
    /// The percentage complete of the current round.
    pub round_percent: f64,
    /// The number of passes per round.
    pub pass_count: i32,
    /// The current pass.
    pub pass_working: i32,
    /// The type of the current pass.
    pub pass_type: PassType,
    /// The percentage complete of the current pass.
    pub pass_percent: f64,
    /// The estimated time remaining in seconds.
    pub eta: i64,
    /// The throughput in bytes per second.
    pub throughput: u64,
    /// The number of bytes that have been written to the device.
    pub bytes_written: u64,
    /// The number of bytes that have been verified.
    pub bytes_verified: u64,
    /// The combined number of bytes that have been read and written.
    pub bytes_total: u64,
    /// The error status of the most recent operation.
    pub result: i32,
    /// The signal that caused the process to exit.
    pub signal: i32,
    /// The spinner character index.
    pub spinner_idx: usize,
    /// The start time of the wipe.
    pub start_time: i64,
    /// The end time of the wipe.
    pub end_time: i64,
    /// The wipe status flag.
    pub wipe_status: i32,
    /// The sync status flag.
    pub sync_status: bool,
    /// Whether to verify the wipe.
    pub verify: bool,
}

impl Default for NwipeContext {
    fn default() -> Self {
        Self {
            device_name: String::new(),
            device_fd: -1,
            device_size: 0,
            device_sector_size: 0,
            device_block_size: 0,
            identity: DeviceIdentity::default(),
            entropy_fd: -1,
            prng: String::new(),
            prng_seed: PrngSeed::default(),
            prng_state: 0,
            select: SelectStatus::False,
            round_count: 0,
            round_working: 0,
            round_percent: 0.0,
            pass_count: 0,
            pass_working: 0,
            pass_type: PassType::None,
            pass_percent: 0.0,
            eta: 0,
            throughput: 0,
            bytes_written: 0,
            bytes_verified: 0,
            bytes_total: 0,
            result: 0,
            signal: 0,
            spinner_idx: 0,
            start_time: 0,
            end_time: 0,
            wipe_status: 0,
            sync_status: false,
            verify: true,  // Default to verifying the wipe
        }
    }
}

impl NwipeContext {
    /// Create a new context for a device.
    pub fn new(device_name: &str) -> Self {
        let mut context = Self::default();
        context.device_name = device_name.to_string();
        context
    }
}
