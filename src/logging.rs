/*
 *  logging.rs: Logging functionality for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion: 2023
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fmt;

use crate::context::NwipeContext;

// Global log storage
lazy_static::lazy_static! {
    static ref LOG_LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);
}

/// Log levels for nwipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Fatal error messages.
    Fatal,
    /// Error messages.
    Error,
    /// Warning messages.
    Warning,
    /// Notice messages.
    Notice,
    /// Informational messages.
    Info,
    /// Debug messages.
    Debug,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Fatal => write!(f, "FATAL"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Notice => write!(f, "NOTICE"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
        }
    }
}

/// Initialize the logging system.
pub fn init_logging(_verbose: bool) {
    // Set up the log file
    let log_path = "/var/log/nwipe.log";
    let file_result = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path);

    match file_result {
        Ok(file) => {
            let mut log_file = LOG_FILE.lock().unwrap();
            *log_file = Some(file);
        },
        Err(e) => {
            eprintln!("Warning: Unable to open log file '{}': {}", log_path, e);
        }
    }

    // Initialize the log lines vector
    let mut log_lines = LOG_LINES.lock().unwrap();
    log_lines.clear();

    // Log the start of the program
    drop(log_lines); // Release the lock before calling nwipe_log
    nwipe_log(LogLevel::Notice, "Nwipe Rust version started");
}

/// Log a message to the nwipe log.
pub fn nwipe_log(level: LogLevel, message: &str) {
    // Get the current time
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Format the log message
    let log_message = format!("{} {} {}", now, level, message);

    // Print to stdout
    println!("{}", log_message);

    // Store in memory
    let mut log_lines = LOG_LINES.lock().unwrap();
    log_lines.push(log_message.clone());

    // Write to log file if available
    if let Ok(log_file) = LOG_FILE.lock() {
        if let Some(mut file) = log_file.as_ref() {
            let _ = writeln!(file, "{}", log_message);
            let _ = file.flush();
        }
    }
}

/// Log system information.
pub fn log_sysinfo() {
    // Get system information
    let os_info = os_info::get();

    nwipe_log(LogLevel::Info, &format!("Operating System: {} {}", os_info.os_type(), os_info.version()));

    // Log CPU information
    if let Ok(cpus) = sys_info::cpu_num() {
        nwipe_log(LogLevel::Info, &format!("CPU Count: {}", cpus));
    }

    // Log memory information
    if let Ok(mem) = sys_info::mem_info() {
        let total_mb = mem.total / 1024;
        nwipe_log(LogLevel::Info, &format!("Memory: {} MB", total_mb));
    }

    // Log kernel information
    if let Ok(kernel) = sys_info::os_release() {
        nwipe_log(LogLevel::Info, &format!("Kernel: {}", kernel));
    }
}

/// Log a summary of the wipe results.
pub fn log_summary(contexts: &[NwipeContext], count: usize) {
    nwipe_log(LogLevel::Info, "***********************************************************");
    nwipe_log(LogLevel::Info, "                        Wipe Summary                        ");
    nwipe_log(LogLevel::Info, "***********************************************************");

    for i in 0..count {
        let context = &contexts[i];

        // Format the result message
        let result_msg = if context.result == 0 {
            "Wipe completed successfully".to_string()
        } else if context.signal > 0 {
            format!("Wipe interrupted by signal {}", context.signal)
        } else {
            format!("Wipe failed with error code {}", context.result)
        };

        // Log the device result
        nwipe_log(
            LogLevel::Info,
            &format!(
                "Device: {} - {}",
                context.device_name,
                result_msg
            )
        );

        // Log additional information if available
        if !context.identity.serial_no.is_empty() {
            nwipe_log(
                LogLevel::Info,
                &format!(
                    "  Serial Number: {}",
                    context.identity.serial_no
                )
            );
        }

        if !context.identity.model_no.is_empty() {
            nwipe_log(
                LogLevel::Info,
                &format!(
                    "  Model: {}",
                    context.identity.model_no
                )
            );
        }

        // Log wipe statistics
        if context.start_time > 0 && context.end_time > 0 {
            let duration = context.end_time - context.start_time;
            let hours = duration / 3600;
            let minutes = (duration % 3600) / 60;
            let seconds = duration % 60;

            nwipe_log(
                LogLevel::Info,
                &format!(
                    "  Duration: {:02}:{:02}:{:02}",
                    hours, minutes, seconds
                )
            );
        }

        if context.bytes_total > 0 {
            // Convert to MB for display
            let mb_total = context.bytes_total / (1024 * 1024);
            nwipe_log(
                LogLevel::Info,
                &format!(
                    "  Total bytes processed: {} MB",
                    mb_total
                )
            );
        }
    }

    nwipe_log(LogLevel::Info, "***********************************************************");
}

/// Log an error message with errno information.
pub fn nwipe_perror(errno: i32, function: &str, message: &str) {
    let error_string = std::io::Error::from_raw_os_error(errno).to_string();
    nwipe_log(
        LogLevel::Error,
        &format!(
            "{}(): {}: {}",
            function, message, error_string
        )
    );
}

/// Convert seconds to hours, minutes, and seconds.
pub fn convert_seconds_to_hours_minutes_seconds(seconds: i64, hours: &mut i32, minutes: &mut i32, secs: &mut i32) {
    *hours = (seconds / 3600) as i32;
    *minutes = ((seconds % 3600) / 60) as i32;
    *secs = (seconds % 60) as i32;
}
