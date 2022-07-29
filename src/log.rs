pub const DEBUG: bool = false;

macro_rules! debug {
	($($arg:tt)*) => {
		if crate::log::DEBUG {
			crate::log::_log!(println, "DEBUG", $($arg)*)
		}
	};
}
pub(crate) use debug;

macro_rules! info {
	($($arg:tt)*) => {
		crate::log::_log!(println, " INFO", $($arg)*)
	};
}
pub(crate) use info;

macro_rules! error {
	($($arg:tt)*) => {
		crate::log::_log!(eprintln, "ERROR", $($arg)*)
	};
}
pub(crate) use error;

macro_rules! _log {
	($print:ident, $level:literal, $($arg:tt)*) => {{
		let s = format!($($arg)*);
		$print!("{} - {} - {}", Local::now().format("%Y-%m-%d %H:%M:%S"), $level, s);
	}};
}
pub(crate) use _log;

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
pub(crate) use truncate_to_debug;

pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}