/*
 *  gui.rs: User interface for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion: 2023
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{self, Color},
    terminal::{self, ClearType},
};

use crate::context::{NwipeContext, SelectStatus, PassType};
use crate::logging::convert_seconds_to_hours_minutes_seconds;

// Spinner characters for the GUI
const SPINNER_CHARS: [char; 4] = ['|', '/', '-', '\\'];

/// Initialize the GUI.
pub fn gui_init() {
    // Set up the terminal
    terminal::enable_raw_mode().unwrap();
    execute!(
        io::stdout(),
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(ClearType::All)
    )
    .unwrap();

    // Draw the header
    draw_header();
}

/// Free the GUI resources.
pub fn gui_free() {
    // Restore the terminal
    execute!(
        io::stdout(),
        terminal::LeaveAlternateScreen,
        cursor::Show
    )
    .unwrap();
    terminal::disable_raw_mode().unwrap();
}

/// Draw the header of the GUI.
fn draw_header() {
    let (width, _) = terminal::size().unwrap();

    execute!(
        io::stdout(),
        cursor::MoveTo(0, 0),
        style::SetForegroundColor(Color::White),
        style::SetBackgroundColor(Color::Blue)
    )
    .unwrap();

    // Print the header
    let header = format!("{:^width$}", "NWIPE - Secure Disk Eraser", width = width as usize);
    println!("{}", header);

    // Reset colors
    execute!(
        io::stdout(),
        style::ResetColor
    )
    .unwrap();
}

/// Display the options screen.
pub fn gui_options() {
    let (width, height) = terminal::size().unwrap();

    // Clear the screen
    execute!(
        io::stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 2)
    )
    .unwrap();

    // Draw the header
    draw_header();

    // Print the options
    println!("\n Options:");
    println!(" --------");
    println!(" - Wipe method: OPS-II (DoD 5220.22-M)");
    println!(" - Verification: Enabled");
    println!(" - Rounds: 1");
    println!(" - PRNG: ISAAC");

    // Print the footer
    execute!(
        io::stdout(),
        cursor::MoveTo(0, height - 2),
        style::SetForegroundColor(Color::White),
        style::SetBackgroundColor(Color::Blue)
    )
    .unwrap();

    let footer = format!("{:^width$}", "Press any key to start wiping...", width = width as usize);
    println!("{}", footer);

    // Reset colors
    execute!(
        io::stdout(),
        style::ResetColor
    )
    .unwrap();

    // Wait for a key press
    loop {
        if let Ok(true) = event::poll(Duration::from_millis(100)) {
            if let Ok(Event::Key(_)) = event::read() {
                break;
            }
        }
    }
}

/// Display the device selection screen.
pub fn gui_select(count: usize, contexts: &mut Vec<NwipeContext>) {
    let (width, height) = terminal::size().unwrap();

    // Current selection
    let mut selected = 0;

    loop {
        // Clear the screen
        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 2)
        )
        .unwrap();

        // Draw the header
        draw_header();

        // Print the device list
        println!("\n Select devices to wipe:");
        println!(" ---------------------");

        for (i, context) in contexts.iter().enumerate() {
            let marker = if context.select == SelectStatus::True { "[*]" } else { "[ ]" };
            let highlight = if i == selected { "> " } else { "  " };

            // Print device information
            println!(
                " {}{}  {} - {} ({} GB)",
                highlight,
                marker,
                context.device_name,
                context.identity.model_no,
                context.device_size / (1024 * 1024 * 1024)
            );
        }

        // Print the footer
        execute!(
            io::stdout(),
            cursor::MoveTo(0, height - 2),
            style::SetForegroundColor(Color::White),
            style::SetBackgroundColor(Color::Blue)
        )
        .unwrap();

        let footer = format!(
            "{:^width$}",
            "Space: Select/Deselect | Enter: Start Wiping | Q: Quit",
            width = width as usize
        );
        println!("{}", footer);

        // Reset colors
        execute!(
            io::stdout(),
            style::ResetColor
        )
        .unwrap();

        // Handle key presses
        if let Ok(true) = event::poll(Duration::from_millis(100)) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    },
                    KeyCode::Down => {
                        if selected < count - 1 {
                            selected += 1;
                        }
                    },
                    KeyCode::Char(' ') => {
                        // Toggle selection
                        if contexts[selected].select == SelectStatus::True {
                            contexts[selected].select = SelectStatus::False;
                        } else {
                            contexts[selected].select = SelectStatus::True;
                        }
                    },
                    KeyCode::Enter => {
                        // Start wiping
                        break;
                    },
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        // Quit
                        unsafe { crate::USER_ABORT = true; }
                        break;
                    },
                    _ => {},
                }
            }
        }
    }
}

/// Display the status screen.
pub fn gui_status(contexts: &[NwipeContext], _count: usize) {
    let (width, height) = terminal::size().unwrap();

    // Status update interval - increased to reduce flickering
    let update_interval = Duration::from_millis(1000);
    let mut last_update = Instant::now();

    // Store previous values to avoid unnecessary updates
    let mut prev_spinner_indices: Vec<usize> = contexts.iter().map(|_| 0).collect();
    let mut prev_percentages: Vec<f64> = contexts.iter().map(|_| -1.0).collect();
    let mut prev_throughputs: Vec<u64> = contexts.iter().map(|_| 0).collect();
    let mut prev_etas: Vec<i64> = contexts.iter().map(|_| -1).collect();

    // Initial full draw
    execute!(
        io::stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    ).unwrap();

    // Draw the header
    draw_header();

    // Print the static parts of the status screen
    execute!(
        io::stdout(),
        cursor::MoveTo(0, 2)
    ).unwrap();

    println!("\n Wiping Status:");
    println!(" -------------");

    // Print the footer once
    execute!(
        io::stdout(),
        cursor::MoveTo(0, height - 2),
        style::SetForegroundColor(Color::White),
        style::SetBackgroundColor(Color::Blue)
    ).unwrap();

    let footer = format!(
        "{:^width$}",
        "Q: Quit",
        width = width as usize
    );
    println!("{}", footer);

    // Reset colors
    execute!(
        io::stdout(),
        style::ResetColor
    ).unwrap();

    // Main loop
    loop {
        // Check if we should exit
        if unsafe { crate::TERMINATE_SIGNAL } {
            break;
        }

        // Update the screen at the specified interval
        if last_update.elapsed() >= update_interval {
            last_update = Instant::now();

            // Update only the dynamic parts of each device status
            for (idx, context) in contexts.iter().enumerate() {
                let row_position = 5 + idx * 3; // Position for this device's status line

                // Get the spinner character
                let spinner_idx = context.spinner_idx % SPINNER_CHARS.len();
                let spinner = SPINNER_CHARS[spinner_idx];

                // Get the status string
                let status = match context.pass_type {
                    PassType::Write => "Writing",
                    PassType::Verify => "Verifying",
                    PassType::FinalBlank => "Final Blank",
                    PassType::FinalOps2 => "Final OPS-II",
                    PassType::None => "Idle",
                };

                // Format the ETA
                let mut hours = 0;
                let mut minutes = 0;
                let mut seconds = 0;
                convert_seconds_to_hours_minutes_seconds(context.eta, &mut hours, &mut minutes, &mut seconds);

                // Calculate throughput
                let throughput_mb = context.throughput / (1024 * 1024);

                // Only update if values have changed
                let needs_update = spinner_idx != prev_spinner_indices[idx] ||
                                  (context.round_percent - prev_percentages[idx]).abs() > 0.01 ||
                                  throughput_mb != prev_throughputs[idx] ||
                                  context.eta != prev_etas[idx];

                if needs_update {
                    // Update the spinner and status line
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, row_position as u16),
                        terminal::Clear(ClearType::CurrentLine)
                    ).unwrap();

                    // Print device status
                    print!(
                        " {} {} - {:.2}% - Round {}/{}, Pass {}/{} - ETA: {:02}:{:02}:{:02} - {}",
                        spinner,
                        context.device_name,
                        context.round_percent,
                        context.round_working,
                        context.round_count,
                        context.pass_working,
                        context.pass_count,
                        hours,
                        minutes,
                        seconds,
                        status
                    );

                    // Update throughput line
                    execute!(
                        io::stdout(),
                        cursor::MoveTo(0, (row_position + 1) as u16),
                        terminal::Clear(ClearType::CurrentLine)
                    ).unwrap();

                    print!("   Throughput: {} MB/s", throughput_mb);

                    // Store current values for next comparison
                    prev_spinner_indices[idx] = spinner_idx;
                    prev_percentages[idx] = context.round_percent;
                    prev_throughputs[idx] = throughput_mb;
                    prev_etas[idx] = context.eta;
                }
            }

            // Flush stdout to ensure all updates are displayed
            io::stdout().flush().unwrap();
        }

        // Handle key presses
        if let Ok(true) = event::poll(Duration::from_millis(100)) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        // Quit
                        unsafe {
                            crate::TERMINATE_SIGNAL = true;
                            crate::USER_ABORT = true;
                        }
                        break;
                    },
                    _ => {},
                }
            }
        }

        // Sleep to avoid high CPU usage
        thread::sleep(Duration::from_millis(50));
    }
}
