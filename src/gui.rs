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

use std::io;
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

    // Status update interval
    let update_interval = Duration::from_millis(500);
    let mut last_update = Instant::now();

    loop {
        // Check if we should exit
        if unsafe { crate::TERMINATE_SIGNAL } {
            break;
        }

        // Update the screen at the specified interval
        if last_update.elapsed() >= update_interval {
            last_update = Instant::now();

            // Clear the screen
            execute!(
                io::stdout(),
                terminal::Clear(ClearType::All),
                cursor::MoveTo(0, 2)
            )
            .unwrap();

            // Draw the header
            draw_header();

            // Print the status
            println!("\n Wiping Status:");
            println!(" -------------");

            for (i, context) in contexts.iter().enumerate() {
                // Get the spinner character
                let spinner = SPINNER_CHARS[context.spinner_idx % SPINNER_CHARS.len()];

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

                // Print device status
                println!(
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

                // Print throughput
                let throughput_mb = context.throughput / (1024 * 1024);
                println!(
                    "   Throughput: {} MB/s",
                    throughput_mb
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
                "Q: Quit",
                width = width as usize
            );
            println!("{}", footer);

            // Reset colors
            execute!(
                io::stdout(),
                style::ResetColor
            )
            .unwrap();
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
