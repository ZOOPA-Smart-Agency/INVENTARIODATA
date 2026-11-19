//! Shared by the desktop app and the command-line tool: reads this computer's details (`collect`),
//! talks to the inventory server (`net`), remembers the last server and trusted certificates (`prefs`),
//! and implements the command-line mode (`cli`).

pub mod cli;
pub mod collect;
pub mod facts;
pub mod net;
pub mod prefs;
pub mod rename;
pub mod selfupdate;

/// This program's version, sent with every report.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
