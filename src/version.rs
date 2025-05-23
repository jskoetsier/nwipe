/*
 *  version.rs: Version information for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion and maintenance by Sebastiaan Koetsier (2025)
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

/// The version string of the program.
pub const VERSION: &str = "0.3.0";

/// The version string with additional information.
pub const VERSION_STRING: &str = "nwipe 0.3.0 (Rust Edition)";

/// The copyright string.
pub const COPYRIGHT: &str = "Copyright (C) 2025 Sebastiaan Koetsier, based on work by Darik Horn and Andy Beverley";

/// Get the version string.
pub fn version_string() -> String {
    VERSION_STRING.to_string()
}

/// Get the copyright string.
pub fn copyright_string() -> String {
    COPYRIGHT.to_string()
}

/// Get the full version information.
pub fn version_info() -> String {
    format!("{}\n{}", VERSION_STRING, COPYRIGHT)
}
