/*
 *  device.rs: Device scanning and management for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion: 2023
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use nix::libc;
use nix::sys::stat::SFlag;

use crate::context::{DeviceIdentity, NwipeContext};
use crate::logging::{nwipe_log, LogLevel};

/// Scan for block devices and populate the contexts vector.
pub fn device_scan(contexts: &mut Vec<NwipeContext>) -> Result<usize, io::Error> {
    // Clear the contexts vector
    contexts.clear();

    // Scan for devices in /dev
    scan_devices_in_directory("/dev", contexts)?;

    // Return the number of devices found
    Ok(contexts.len())
}

/// Get devices from a list of device names.
pub fn device_get(contexts: &mut Vec<NwipeContext>, device_names: &[String]) -> Result<usize, io::Error> {
    // Clear the contexts vector
    contexts.clear();

    // Process each device name
    for name in device_names {
        // Create a context for the device
        let mut context = NwipeContext::new(name);

        // Check if the device exists
        if !Path::new(name).exists() {
            nwipe_log(LogLevel::Warning, &format!("Device '{}' not found.", name));
            continue;
        }

        // Get device information
        if let Err(e) = get_device_info(&mut context) {
            nwipe_log(LogLevel::Warning, &format!("Failed to get info for device '{}': {}", name, e));
            continue;
        }

        // Add the context to the vector
        contexts.push(context);
    }

    // Return the number of devices found
    Ok(contexts.len())
}

/// Scan for block devices in a directory.
fn scan_devices_in_directory(dir_path: &str, contexts: &mut Vec<NwipeContext>) -> Result<(), io::Error> {
    // Read the directory entries
    let entries = fs::read_dir(dir_path)?;

    // Process each entry
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip non-block devices
        let metadata = fs::metadata(&path)?;
        if !metadata.file_type().is_block_device() {
            continue;
        }

        // Get the device name
        let device_name = path.to_string_lossy().to_string();

        // Create a context for the device
        let mut context = NwipeContext::new(&device_name);

        // Get device information
        if let Err(e) = get_device_info(&mut context) {
            nwipe_log(LogLevel::Warning, &format!("Failed to get info for device '{}': {}", device_name, e));
            continue;
        }

        // Add the context to the vector
        contexts.push(context);
    }

    Ok(())
}

/// Get information about a device.
fn get_device_info(context: &mut NwipeContext) -> Result<(), io::Error> {
    // Open the device
    let file = File::options().read(true).write(true).open(&context.device_name)?;

    // Get the file descriptor
    let fd = file.as_raw_fd();

    // Get device identity information
    get_device_identity(fd, context)?;

    // Get device size
    get_device_size(fd, context)?;

    // Get device sector and block size
    get_device_sector_block_size(fd, context)?;

    Ok(())
}

/// Get device identity information.
fn get_device_identity(fd: i32, context: &mut NwipeContext) -> Result<(), io::Error> {
    // In a real implementation, we would use ioctl calls to get device identity information
    // For now, we'll just set some placeholder values

    // Try to extract device model and serial from sysfs
    if let Some(device_info) = extract_device_info_from_sysfs(&context.device_name) {
        context.identity = device_info;
    } else {
        // Set default values if sysfs info not available
        context.identity.model_no = "Unknown Model".to_string();
        context.identity.serial_no = "Unknown Serial".to_string();
        context.identity.firmware_rev = "Unknown Firmware".to_string();
    }

    Ok(())
}

/// Extract device information from sysfs.
fn extract_device_info_from_sysfs(device_name: &str) -> Option<DeviceIdentity> {
    // Extract the device name without the /dev/ prefix
    let dev_name = Path::new(device_name)
        .file_name()?
        .to_str()?;

    // Construct the sysfs path
    let sysfs_path = PathBuf::from(format!("/sys/block/{}", dev_name));

    if !sysfs_path.exists() {
        return None;
    }

    let mut identity = DeviceIdentity::default();

    // Try to read model
    if let Ok(model) = fs::read_to_string(sysfs_path.join("device/model")) {
        identity.model_no = model.trim().to_string();
    }

    // Try to read serial
    if let Ok(serial) = fs::read_to_string(sysfs_path.join("device/serial")) {
        identity.serial_no = serial.trim().to_string();
    }

    // Try to read firmware revision
    if let Ok(firmware) = fs::read_to_string(sysfs_path.join("device/firmware_rev")) {
        identity.firmware_rev = firmware.trim().to_string();
    }

    Some(identity)
}

/// Get device size.
fn get_device_size(fd: i32, context: &mut NwipeContext) -> Result<(), io::Error> {
    // In a real implementation, we would use ioctl calls to get device size
    // For now, we'll use a placeholder implementation

    // Try to get size using BLKGETSIZE64 ioctl
    let mut size: u64 = 0;

    // This is a placeholder for the actual ioctl call
    // In real code, we would use something like:
    // unsafe {
    //     let result = libc::ioctl(fd, libc::BLKGETSIZE64, &mut size);
    //     if result != 0 {
    //         return Err(io::Error::last_os_error());
    //     }
    // }

    // For now, just set a placeholder size
    context.device_size = size;

    // If we couldn't get the size, try to use lseek
    if context.device_size == 0 {
        // This is a placeholder for the actual lseek call
        // In real code, we would use something like:
        // let size = unsafe { libc::lseek64(fd, 0, libc::SEEK_END) };
        // if size != -1 {
        //     context.device_size = size as u64;
        // }
    }

    // For demonstration purposes, set a reasonable size
    if context.device_size == 0 {
        context.device_size = 1024 * 1024 * 1024; // 1 GB
    }

    Ok(())
}

/// Get device sector and block size.
fn get_device_sector_block_size(fd: i32, context: &mut NwipeContext) -> Result<(), io::Error> {
    // In a real implementation, we would use ioctl calls to get sector and block size
    // For now, we'll use placeholder values

    // Try to get sector size using BLKSSZGET ioctl
    let mut sector_size: u64 = 0;

    // This is a placeholder for the actual ioctl call
    // In real code, we would use something like:
    // unsafe {
    //     let result = libc::ioctl(fd, libc::BLKSSZGET, &mut sector_size);
    //     if result != 0 {
    //         return Err(io::Error::last_os_error());
    //     }
    // }

    // For now, just set a placeholder sector size
    context.device_sector_size = 512;

    // Try to get block size using BLKBSZGET ioctl
    let mut block_size: i32 = 0;

    // This is a placeholder for the actual ioctl call
    // In real code, we would use something like:
    // unsafe {
    //     let result = libc::ioctl(fd, libc::BLKBSZGET, &mut block_size);
    //     if result != 0 {
    //         return Err(io::Error::last_os_error());
    //     }
    // }

    // For now, just set a placeholder block size
    context.device_block_size = 4096;

    Ok(())
}

/// Check if a device is mounted.
pub fn device_is_mounted(device_name: &str) -> bool {
    // Read /proc/mounts to check if the device is mounted
    if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == device_name {
                return true;
            }
        }
    }

    false
}
