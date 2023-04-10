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

/// This is for logging of sensitive information that you usually don't want to
/// log in production, regardless of log level. It won't log anything unless
/// `sensitive` is set to true.  
/// In this crate, it's used to log clipboard contents.
macro_rules! sensitive {
	($log_macro:path, $($arg:tt)*) => {
		if *crate::log::log_sensitive_information::get() {
			$log_macro!("Sensitive: {}", format!($($arg)*))
		}
	};
}
pub(crate) use sensitive;

#[allow(unused)]
macro_rules! trace {
	($($arg:tt)*) => {
		if *crate::log::level::get() >= crate::log::Level::Trace {
			crate::log::_log!(println, "TRACE", $($arg)*)
		}
	};
}
#[allow(unused)]
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

pub fn concise_numbers(ns: &[u8]) -> String {
    if ns.is_empty() {
        return "[]".to_string();
    }
    if ns.len() == 1 {
        return format!("[{}]", ns[0]);
    }
    let mut range_size = 1;
    let mut strings = vec![];
    for nn in ns.windows(2) {
        let [n1, n2]: [u8] = *nn else {unreachable!()};
        if n1 + 1 == n2 {
            range_size += 1;
        } else {
            range_size = 1;
        }
        if range_size < 3 {
            strings.push(format!("{n1}"));
        }
        if range_size == 3 {
            strings.push("..".to_string());
        }
    }
    strings.push(format!("{}", ns[ns.len() - 1]));
    strings.push("..".to_owned());
    let mut full_strings = vec![];
    for ss in strings.windows(2) {
        let [s1, s2] = ss else {unreachable!()};
        full_strings.push(s1.to_owned());
        if s1 != ".." && s2 != ".." {
            full_strings.push(", ".to_owned());
        }
    }
    format!("[{}]", full_strings.join(""))
}

#[test]
fn test() {
    assert_eq!("[]", concise_numbers(&[]));
    assert_eq!("[123]", concise_numbers(&[123]));
    assert_eq!("[1..5]", concise_numbers(&[1, 2, 3, 4, 5]));
    assert_eq!(
        "[0, 2..4, 6..8, 10, 11]",
        concise_numbers(&[0, 2, 3, 4, 6, 7, 8, 10, 11])
    );
    assert_eq!(
        "[0, 2..4, 6..8, 10..12]",
        concise_numbers(&[0, 2, 3, 4, 6, 7, 8, 10, 11, 12])
    );
    assert_eq!(
        "[0, 2..4, 6..8, 10]",
        concise_numbers(&[0, 2, 3, 4, 6, 7, 8, 10])
    );
    assert_eq!(
        "[0..4, 6..8, 10]",
        concise_numbers(&[0, 1, 2, 3, 4, 6, 7, 8, 10])
    );
    assert_eq!(
        "[0, 1, 3, 4, 6..8, 10]",
        concise_numbers(&[0, 1, 3, 4, 6, 7, 8, 10])
    );
}
