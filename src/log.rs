mustatex! {
    pub(crate) level: Level = Level::Debug;
    pub(crate) timestamp: bool = true;
    pub(crate) log_sensitive_information: bool = false;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Level {
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for Level {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        return Level::Debug;
        #[cfg(not(debug_assertions))]
        Level::Info
    }
}

/// This is for debug logging of sensitive information that you usually don't
/// want to log in production, even if debug logging is enabled. It won't log
/// anything unless `sensitive` is set to true.  
/// In this crate, it's used to log clipboard contents.
macro_rules! sensitive {
	($($arg:tt)*) => {
		if *crate::log::level::get() >= crate::log::Level::Debug
		&& *crate::log::log_sensitive_information::get() {
			crate::log::_log!(println, "SENSI", $($arg)*)
		}
	};
}
pub(crate) use sensitive;

macro_rules! trace {
	($($arg:tt)*) => {
		if *crate::log::level::get() >= crate::log::Level::Trace {
			crate::log::_log!(println, "TRACE", $($arg)*)
		}
	};
}
pub(crate) use trace;

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

macro_rules! fatal {
	($($arg:tt)*) => {
		if *crate::log::level::get() >= crate::log::Level::Fatal {
			crate::log::_log!(eprintln, "FATAL", $($arg)*)
		}
	};
}
pub(crate) use fatal;

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
