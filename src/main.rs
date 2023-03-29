use chrono::Local;
use clap::Parser;
use error::{Generify, MyResult, Standardize};
use nix::sys::signal::{self, Signal};
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use nix::unistd::{fork, Pid};
use nix::{sys::wait::waitpid, unistd::ForkResult};
use std::thread;
use std::time::SystemTime;
use std::{thread::sleep, time::Duration};

mod clipboard;
mod error;
mod log;
mod mustatex;
mod sync;

fn main() {
    let args = Args::parse();
    configure_logging(&args);
    if args.run_forked {
        run_forked()
    } else {
        run()
    }
}

/// cli arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, max_term_width = 120)]
struct Args {
    /// granularity to log
    #[arg(long, value_enum, default_value_t = log::Level::default())]
    log_level: log::Level,

    /// whether to include timestamps in the logs (systemd already includes
    /// timestamps so you'll want to enable this for systemd)
    #[arg(long)]
    hide_timestamp: bool,

    /// whether to run the sync forked so the state can be cleaned up
    /// periodically. typically, this should be true.
    #[arg(long)]
    #[cfg_attr(not(debug_assertions), arg(default_value_t = true))]
    run_forked: bool,

    /// when debug logging is enabled, it won't show clipboard contents, because
    /// clipboard contents are sensitive user information. but if you set this
    /// to true, in addition to enabling debug logging, then it will log the
    /// clipboard contents.
    #[arg(long)]
    log_clipboard_contents: bool,
}

fn configure_logging(args: &Args) {
    log::level::set(args.log_level);
    log::timestamp::set(!args.hide_timestamp);
    log::log_sensitive_information::set(args.log_clipboard_contents);
}

#[allow(dead_code)]
fn run_forked() {
    log::info!("started clipboard sync manager");
    loop {
        match unsafe { fork() }.expect("Failed to fork") {
            ForkResult::Parent { child } => {
                log::debug!("child process {child} successfully initialized.");
                kill_after(child, 600);
                let status = waitpid(Some(child), None)
                    .expect("there was a problem managing the child process, so the service is exiting. check that pid {child} is not running before restarting this service");
                log::debug!("child process {child} completed with: {:?}", status);
                sleep(Duration::from_secs(1));
            }
            ForkResult::Child => run(),
        }
    }
}

fn run() {
    log::info!("starting clipboard sync");
    loop_with_error_pain_management(
        sync::get_clipboards().unwrap(),
        |cb| sync::keep_synced(cb),
        |_| sync::get_clipboards().unwrap(),
    )
    .unwrap();
}

pub fn kill_after(pid: Pid, seconds: u64) {
    thread::spawn(move || {
        log::debug!("waiting {seconds} seconds and then killing {pid}.");
        thread::sleep(Duration::from_secs(seconds));
        match waitpid(Some(pid), Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => log::debug!("child {pid} is still alive, as expected."),
            Ok(ok) => {
                log::warning!("expected child process {pid} to be StillAlive but got: {ok:?}")
            }
            Err(e) => log::error!("error getting status of child process {pid}: {e}"),
        }
        log::debug!("routinely attempting to kill child process {pid}.");
        if let Err(e) = signal::kill(pid, Signal::SIGTERM) {
            log::error!("error killing child process {pid}: {e}")
        }
    });
}

/// Execute an action with a sophisticated retry mechanism
/// If the action fails:
/// - 1. run a recovery step to manipulate the input
/// - 2. attempt to execute the action again
/// If the action fails too frequently, exit
fn loop_with_error_pain_management<
    Input,
    Return,
    Action: Fn(&Input) -> MyResult<Return>,
    Recovery: Fn(Input) -> Input,
>(
    // data passed into action and reset by recovery
    initial_input: Input,
    // action to attempt on every iteration
    action: Action,
    // action to attempt on every error - errors here are not yet handled. you can panic if necessary
    recovery: Recovery,
) -> MyResult<Return> {
    let mut input = initial_input;
    let mut errorcount: u64 = 0;
    let mut first_error: SystemTime = SystemTime::UNIX_EPOCH;
    let mut last_error: SystemTime = SystemTime::UNIX_EPOCH;
    loop {
        match action(&input) {
            Ok(ret) => return Ok(ret),
            Err(err) => {
                log::error!("action exited with error: {:?}", err);
                let now = SystemTime::now();
                input = recovery(input);
                if errorcount == 0 {
                    first_error = now;
                } else if SystemTime::now().duration_since(last_error).unwrap()
                    > Duration::from_secs(10)
                {
                    errorcount = 0;
                } else {
                    let error_session_seconds = SystemTime::now()
                        .duration_since(first_error)
                        .unwrap()
                        .as_secs();
                    let error_session_rate_scaled = errorcount
                        .checked_mul(100)
                        .unwrap_or(u64::MAX)
                        .checked_div(error_session_seconds)
                        .unwrap_or(u64::MAX);
                    let error_pain = error_session_rate_scaled
                        .checked_add(error_session_seconds)
                        .unwrap_or(u64::MAX);
                    if error_pain > 100 {
                        Err("too many errors, exiting".to_string())
                            .standardize()
                            .generify()?;
                    }
                }
                last_error = now;
                errorcount += 1;
                sleep(Duration::from_millis(1000));
            }
        }
        log::info!("retrying");
    }
}
