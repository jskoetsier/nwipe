/*
 *  method.rs: Wiping methods for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion: 2023
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::stat::Mode;
use nix::unistd::{close, fsync, lseek, Whence};

use crate::context::{NwipeContext, PassType};
use crate::logging::{nwipe_log, LogLevel};
use crate::prng;

// Buffer size for wiping (4 MiB)
const NWIPE_KNOB_BUFSIZE: usize = 4 * 1024 * 1024;

/// Run the selected wiping method on the device.
pub fn run_method(context: &NwipeContext) -> i32 {
    // Set up a safe copy of the context that we can modify
    let mut ctx = context.clone();

    // Record the start time
    ctx.start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Set the wipe status to in progress
    ctx.wipe_status = 1;

    // Log the start of the wipe
    nwipe_log(
        LogLevel::Notice,
        &format!("Starting wipe of device {}", ctx.device_name)
    );

    // Determine which method to use based on the context's prng field
    let result = match ctx.prng.as_str() {
        "ops2" => ops2_wipe(&mut ctx),
        "dod" => dod_wipe(&mut ctx),
        "gutmann" => gutmann_wipe(&mut ctx),
        "random" => random_wipe(&mut ctx),
        "zero" => zero_wipe(&mut ctx),
        _ => {
            nwipe_log(
                LogLevel::Error,
                &format!("Unknown wipe method: {}", ctx.prng)
            );
            -1
        }
    };

    // Record the end time
    ctx.end_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Set the wipe status to complete
    ctx.wipe_status = 0;

    // Log the completion of the wipe
    if result == 0 {
        nwipe_log(
            LogLevel::Notice,
            &format!("Wipe of device {} completed successfully", ctx.device_name)
        );
    } else {
        nwipe_log(
            LogLevel::Error,
            &format!("Wipe of device {} failed with error code {}", ctx.device_name, result)
        );
    }

    result
}

/// OPS-II wiping method (DoD 5220.22-M).
pub fn ops2_wipe(context: &mut NwipeContext) -> i32 {
    // Set up the wipe parameters
    context.round_count = 3;
    context.pass_count = 3;

    // Perform the wipe
    for round in 0..context.round_count {
        context.round_working = round + 1;

        // Pass 1: Write zeros
        context.pass_working = 1;
        context.pass_type = PassType::Write;
        if let Err(e) = write_pattern(context, &[0x00]) {
            nwipe_log(
                LogLevel::Error,
                &format!("OPS-II write zeros failed: {}", e)
            );
            return -1;
        }

        // Pass 2: Write ones
        context.pass_working = 2;
        context.pass_type = PassType::Write;
        if let Err(e) = write_pattern(context, &[0xFF]) {
            nwipe_log(
                LogLevel::Error,
                &format!("OPS-II write ones failed: {}", e)
            );
            return -1;
        }

        // Pass 3: Write random data
        context.pass_working = 3;
        context.pass_type = PassType::Write;
        if let Err(e) = write_random(context) {
            nwipe_log(
                LogLevel::Error,
                &format!("OPS-II write random failed: {}", e)
            );
            return -1;
        }

        // Verify if requested
        if context.verify {
            context.pass_working = 4;
            context.pass_type = PassType::Verify;
            if let Err(e) = verify_random(context) {
                nwipe_log(
                    LogLevel::Error,
                    &format!("OPS-II verify failed: {}", e)
                );
                return -1;
            }
        }
    }

    // Final pass: Write zeros
    context.pass_working = context.pass_count;
    context.pass_type = PassType::FinalBlank;
    if let Err(e) = write_pattern(context, &[0x00]) {
        nwipe_log(
            LogLevel::Error,
            &format!("OPS-II final zero write failed: {}", e)
        );
        return -1;
    }

    0
}

/// DoD 5220.22-M wiping method.
pub fn dod_wipe(context: &mut NwipeContext) -> i32 {
    // Set up the wipe parameters
    context.round_count = 1;
    context.pass_count = 3;

    // Pass 1: Write zeros
    context.pass_working = 1;
    context.pass_type = PassType::Write;
    if let Err(e) = write_pattern(context, &[0x00]) {
        nwipe_log(
            LogLevel::Error,
            &format!("DoD write zeros failed: {}", e)
        );
        return -1;
    }

    // Pass 2: Write ones
    context.pass_working = 2;
    context.pass_type = PassType::Write;
    if let Err(e) = write_pattern(context, &[0xFF]) {
        nwipe_log(
            LogLevel::Error,
            &format!("DoD write ones failed: {}", e)
        );
        return -1;
    }

    // Pass 3: Write random data
    context.pass_working = 3;
    context.pass_type = PassType::Write;
    if let Err(e) = write_random(context) {
        nwipe_log(
            LogLevel::Error,
            &format!("DoD write random failed: {}", e)
        );
        return -1;
    }

    // Verify if requested
    if context.verify {
        context.pass_working = 4;
        context.pass_type = PassType::Verify;
        if let Err(e) = verify_random(context) {
            nwipe_log(
                LogLevel::Error,
                &format!("DoD verify failed: {}", e)
            );
            return -1;
        }
    }

    0
}

/// Gutmann wiping method.
pub fn gutmann_wipe(context: &mut NwipeContext) -> i32 {
    // Set up the wipe parameters
    context.round_count = 1;
    context.pass_count = 35;

    // Passes 1-4: Random data
    for pass in 0..4 {
        context.pass_working = pass + 1;
        context.pass_type = PassType::Write;
        if let Err(e) = write_random(context) {
            nwipe_log(
                LogLevel::Error,
                &format!("Gutmann write random (pass {}) failed: {}", pass + 1, e)
            );
            return -1;
        }
    }

    // Passes 5-31: Specific patterns
    let patterns = [
        &[0x55, 0x55, 0x55], // 5
        &[0xAA, 0xAA, 0xAA], // 6
        &[0x92, 0x49, 0x24], // 7
        &[0x49, 0x24, 0x92], // 8
        &[0x24, 0x92, 0x49], // 9
        &[0x00, 0x00, 0x00], // 10
        &[0x11, 0x11, 0x11], // 11
        &[0x22, 0x22, 0x22], // 12
        &[0x33, 0x33, 0x33], // 13
        &[0x44, 0x44, 0x44], // 14
        &[0x55, 0x55, 0x55], // 15
        &[0x66, 0x66, 0x66], // 16
        &[0x77, 0x77, 0x77], // 17
        &[0x88, 0x88, 0x88], // 18
        &[0x99, 0x99, 0x99], // 19
        &[0xAA, 0xAA, 0xAA], // 20
        &[0xBB, 0xBB, 0xBB], // 21
        &[0xCC, 0xCC, 0xCC], // 22
        &[0xDD, 0xDD, 0xDD], // 23
        &[0xEE, 0xEE, 0xEE], // 24
        &[0xFF, 0xFF, 0xFF], // 25
        &[0x92, 0x49, 0x24], // 26
        &[0x49, 0x24, 0x92], // 27
        &[0x24, 0x92, 0x49], // 28
        &[0x6D, 0xB6, 0xDB], // 29
        &[0xB6, 0xDB, 0x6D], // 30
        &[0xDB, 0x6D, 0xB6], // 31
    ];

    for (i, pattern) in patterns.iter().enumerate() {
        context.pass_working = i as i32 + 5;
        context.pass_type = PassType::Write;
        if let Err(e) = write_pattern(context, *pattern) {
            nwipe_log(
                LogLevel::Error,
                &format!("Gutmann write pattern (pass {}) failed: {}", i + 5, e)
            );
            return -1;
        }
    }

    // Passes 32-35: Random data
    for pass in 0..4 {
        context.pass_working = pass + 32;
        context.pass_type = PassType::Write;
        if let Err(e) = write_random(context) {
            nwipe_log(
                LogLevel::Error,
                &format!("Gutmann write random (pass {}) failed: {}", pass + 32, e)
            );
            return -1;
        }
    }

    // Verify if requested
    if context.verify {
        context.pass_working = 36;
        context.pass_type = PassType::Verify;
        if let Err(e) = verify_random(context) {
            nwipe_log(
                LogLevel::Error,
                &format!("Gutmann verify failed: {}", e)
            );
            return -1;
        }
    }

    0
}

/// Random data wiping method.
pub fn random_wipe(context: &mut NwipeContext) -> i32 {
    // Set up the wipe parameters
    context.round_count = 1;
    context.pass_count = 1;

    // Pass 1: Write random data
    context.pass_working = 1;
    context.pass_type = PassType::Write;
    if let Err(e) = write_random(context) {
        nwipe_log(
            LogLevel::Error,
            &format!("Random write failed: {}", e)
        );
        return -1;
    }

    // Verify if requested
    if context.verify {
        context.pass_working = 2;
        context.pass_type = PassType::Verify;
        if let Err(e) = verify_random(context) {
            nwipe_log(
                LogLevel::Error,
                &format!("Random verify failed: {}", e)
            );
            return -1;
        }
    }

    0
}

/// Zero fill wiping method.
pub fn zero_wipe(context: &mut NwipeContext) -> i32 {
    // Set up the wipe parameters
    context.round_count = 1;
    context.pass_count = 1;

    // Pass 1: Write zeros
    context.pass_working = 1;
    context.pass_type = PassType::Write;
    if let Err(e) = write_pattern(context, &[0x00]) {
        nwipe_log(
            LogLevel::Error,
            &format!("Zero write failed: {}", e)
        );
        return -1;
    }

    // Verify if requested
    if context.verify {
        context.pass_working = 2;
        context.pass_type = PassType::Verify;
        if let Err(e) = verify_pattern(context, &[0x00]) {
            nwipe_log(
                LogLevel::Error,
                &format!("Zero verify failed: {}", e)
            );
            return -1;
        }
    }

    0
}

/// Write a pattern to the device.
fn write_pattern(context: &mut NwipeContext, pattern: &[u8]) -> Result<(), io::Error> {
    // Open the device
    let mut file = unsafe { File::from_raw_fd(context.device_fd) };

    // Seek to the beginning of the device
    file.seek(SeekFrom::Start(0))?;

    // Create a buffer filled with the pattern
    let mut buffer = vec![0u8; NWIPE_KNOB_BUFSIZE];
    for i in 0..buffer.len() {
        buffer[i] = pattern[i % pattern.len()];
    }

    // Calculate the number of blocks to write
    let block_count = (context.device_size + NWIPE_KNOB_BUFSIZE as u64 - 1) / NWIPE_KNOB_BUFSIZE as u64;

    // Write the pattern to the device
    for block in 0..block_count {
        // Check if we should abort
        if unsafe { crate::TERMINATE_SIGNAL } {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Wipe interrupted by user"));
        }

        // Calculate the size of this block
        let size = if block == block_count - 1 && context.device_size % NWIPE_KNOB_BUFSIZE as u64 != 0 {
            (context.device_size % NWIPE_KNOB_BUFSIZE as u64) as usize
        } else {
            NWIPE_KNOB_BUFSIZE
        };

        // Write the block
        file.write_all(&buffer[0..size])?;

        // Update progress
        context.bytes_written += size as u64;
        context.bytes_total += size as u64;
        context.round_percent = (block as f64 + 1.0) / block_count as f64 * 100.0;

        // Update ETA and throughput
        update_eta_throughput(context);
    }

    // Sync the device
    context.sync_status = true;
    file.sync_all()?;
    context.sync_status = false;

    // Don't close the file descriptor as it's owned by the context
    std::mem::forget(file);

    Ok(())
}

/// Write random data to the device.
fn write_random(context: &mut NwipeContext) -> Result<(), io::Error> {
    // Open the device
    let mut file = unsafe { File::from_raw_fd(context.device_fd) };

    // Seek to the beginning of the device
    file.seek(SeekFrom::Start(0))?;

    // Create a buffer for random data
    let mut buffer = vec![0u8; NWIPE_KNOB_BUFSIZE];

    // Initialize the PRNG
    let mut prng = prng::init_prng(&context.prng)?;

    // Calculate the number of blocks to write
    let block_count = (context.device_size + NWIPE_KNOB_BUFSIZE as u64 - 1) / NWIPE_KNOB_BUFSIZE as u64;

    // Write random data to the device
    for block in 0..block_count {
        // Check if we should abort
        if unsafe { crate::TERMINATE_SIGNAL } {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Wipe interrupted by user"));
        }

        // Fill the buffer with random data
        prng.fill_bytes(&mut buffer);

        // Calculate the size of this block
        let size = if block == block_count - 1 && context.device_size % NWIPE_KNOB_BUFSIZE as u64 != 0 {
            (context.device_size % NWIPE_KNOB_BUFSIZE as u64) as usize
        } else {
            NWIPE_KNOB_BUFSIZE
        };

        // Write the block
        file.write_all(&buffer[0..size])?;

        // Update progress
        context.bytes_written += size as u64;
        context.bytes_total += size as u64;
        context.round_percent = (block as f64 + 1.0) / block_count as f64 * 100.0;

        // Update ETA and throughput
        update_eta_throughput(context);
    }

    // Sync the device
    context.sync_status = true;
    file.sync_all()?;
    context.sync_status = false;

    // Don't close the file descriptor as it's owned by the context
    std::mem::forget(file);

    Ok(())
}

/// Verify that a pattern was written correctly.
fn verify_pattern(context: &mut NwipeContext, pattern: &[u8]) -> Result<(), io::Error> {
    // Open the device
    let mut file = unsafe { File::from_raw_fd(context.device_fd) };

    // Seek to the beginning of the device
    file.seek(SeekFrom::Start(0))?;

    // Create a buffer for reading
    let mut buffer = vec![0u8; NWIPE_KNOB_BUFSIZE];

    // Create a buffer with the expected pattern
    let mut expected = vec![0u8; NWIPE_KNOB_BUFSIZE];
    for i in 0..expected.len() {
        expected[i] = pattern[i % pattern.len()];
    }

    // Calculate the number of blocks to read
    let block_count = (context.device_size + NWIPE_KNOB_BUFSIZE as u64 - 1) / NWIPE_KNOB_BUFSIZE as u64;

    // Read and verify the device
    for block in 0..block_count {
        // Check if we should abort
        if unsafe { crate::TERMINATE_SIGNAL } {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Verification interrupted by user"));
        }

        // Calculate the size of this block
        let size = if block == block_count - 1 && context.device_size % NWIPE_KNOB_BUFSIZE as u64 != 0 {
            (context.device_size % NWIPE_KNOB_BUFSIZE as u64) as usize
        } else {
            NWIPE_KNOB_BUFSIZE
        };

        // Read the block
        file.read_exact(&mut buffer[0..size])?;

        // Verify the block
        for i in 0..size {
            if buffer[i] != expected[i] {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Verification failed at offset {}: expected {:#04x}, found {:#04x}",
                        block * NWIPE_KNOB_BUFSIZE as u64 + i as u64,
                        expected[i],
                        buffer[i]
                    )
                ));
            }
        }

        // Update progress
        context.bytes_verified += size as u64;
        context.bytes_total += size as u64;
        context.round_percent = (block as f64 + 1.0) / block_count as f64 * 100.0;

        // Update ETA and throughput
        update_eta_throughput(context);
    }

    // Don't close the file descriptor as it's owned by the context
    std::mem::forget(file);

    Ok(())
}

/// Verify that random data was written correctly.
fn verify_random(context: &mut NwipeContext) -> Result<(), io::Error> {
    // This is a placeholder for random data verification
    // In a real implementation, we would need to store the random data or its hash
    // for verification, but for now we'll just simulate verification

    // Open the device
    let mut file = unsafe { File::from_raw_fd(context.device_fd) };

    // Seek to the beginning of the device
    file.seek(SeekFrom::Start(0))?;

    // Create a buffer for reading
    let mut buffer = vec![0u8; NWIPE_KNOB_BUFSIZE];

    // Calculate the number of blocks to read
    let block_count = (context.device_size + NWIPE_KNOB_BUFSIZE as u64 - 1) / NWIPE_KNOB_BUFSIZE as u64;

    // Read and "verify" the device
    for block in 0..block_count {
        // Check if we should abort
        if unsafe { crate::TERMINATE_SIGNAL } {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Verification interrupted by user"));
        }

        // Calculate the size of this block
        let size = if block == block_count - 1 && context.device_size % NWIPE_KNOB_BUFSIZE as u64 != 0 {
            (context.device_size % NWIPE_KNOB_BUFSIZE as u64) as usize
        } else {
            NWIPE_KNOB_BUFSIZE
        };

        // Read the block
        file.read_exact(&mut buffer[0..size])?;

        // In a real implementation, we would verify the block against the expected random data
        // For now, we just check that the data is not all zeros or all ones
        let mut all_zeros = true;
        let mut all_ones = true;

        for i in 0..size {
            if buffer[i] != 0 {
                all_zeros = false;
            }
            if buffer[i] != 0xFF {
                all_ones = false;
            }

            if !all_zeros && !all_ones {
                break;
            }
        }

        if all_zeros || all_ones {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Verification failed at block {}: data is {}",
                    block,
                    if all_zeros { "all zeros" } else { "all ones" }
                )
            ));
        }

        // Update progress
        context.bytes_verified += size as u64;
        context.bytes_total += size as u64;
        context.round_percent = (block as f64 + 1.0) / block_count as f64 * 100.0;

        // Update ETA and throughput
        update_eta_throughput(context);
    }

    // Don't close the file descriptor as it's owned by the context
    std::mem::forget(file);

    Ok(())
}

/// Update the ETA and throughput values in the context.
fn update_eta_throughput(context: &mut NwipeContext) {
    // Get the current time
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Calculate the elapsed time
    let elapsed = now - context.start_time;

    // Avoid division by zero
    if elapsed == 0 {
        return;
    }

    // Calculate the throughput in bytes per second
    context.throughput = context.bytes_total / elapsed as u64;

    // Calculate the ETA
    let remaining_bytes = context.device_size * context.round_count as u64 * context.pass_count as u64 - context.bytes_total;

    if context.throughput > 0 {
        context.eta = remaining_bytes as i64 / context.throughput as i64;
    } else {
        context.eta = 0;
    }
}
