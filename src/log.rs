mustatex! {
    pub(crate) level: Level = Level::Debug;
    pub(crate) timestamp: bool = true;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Default for Level {	
    fn default() -> Self {
        #[cfg(debug_assertions)] return Level::Debug;
		#[cfg(not(debug_assertions))] Level::Info
    }
}

macro_rules! debug {
	($($arg:tt)*) => {
		if *crate::log::level::get() >= crate::log::Level::Debug {
			crate::log::_log!(println, "DEBUG", $($arg)*)
		}
	};
}
pub(crate) use debug;

macro_rules! info {
	($($arg:tt)*) => {
		if *crate::log::level::get() >= crate::log::Level::Info {
			crate::log::_log!(println, " INFO", $($arg)*)
		}
	};
}
pub(crate) use info;

macro_rules! warning {
	($($arg:tt)*) => {
		if *crate::log::level::get() >= crate::log::Level::Warn {
			crate::log::_log!(eprintln, " WARN", $($arg)*)
		}
	};
}
pub(crate) use warning;

macro_rules! error {
	($($arg:tt)*) => {
		if *crate::log::level::get() >= crate::log::Level::Error {
			crate::log::_log!(eprintln, "ERROR", $($arg)*)
		}
	};
}
pub(crate) use error;

macro_rules! _log {
	($print:ident, $level:literal, $($arg:tt)*) => {{
		let s = format!($($arg)*);
		if *crate::log::timestamp::get() {
			$print!("{} - {} - {}", Local::now().format("%Y-%m-%d %H:%M:%S"), $level, s);
		} else {
			$print!("{} - {}", $level, s);
		}
	}};
}
pub(crate) use _log;

#[allow(unused)]
macro_rules! truncate_to_debug {
	($logger:path, $($arg:tt)*) => {
		let s = format!($($arg)*);
		let s = if s.len() > 254 && !crate::log::DEBUG {
			format!("{}{}", crate::log::truncate(&s, 200), "... message truncated, enable debug for entire message")
		} else {
			s
		};
		$logger!("{}", s);
	};
}
#[allow(unused)]
pub(crate) use truncate_to_debug;

use crate::mustatex::mustatex;

#[allow(unused)]
pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
