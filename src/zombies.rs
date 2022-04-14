use anyhow::Result as AnyResult;
use std::{process::Command, thread, time};

pub fn spawn_reaper() {
    thread::spawn(|| {
        let delay = time::Duration::from_secs(1);
        let my_pid = std::process::id();
        loop {
            reap_children(my_pid).unwrap_or_else(|err| eprintln!("{:?}", err));
            thread::sleep(delay);
        }
    });
}

pub fn reap_children(parent_pid: u32) -> AnyResult<()> {
    for pid in get_children(parent_pid)? {
        let _ = nix::sys::wait::waitpid(
            nix::unistd::Pid::from_raw(pid as i32),
            Some(nix::sys::wait::WaitPidFlag::WNOHANG),
        )?;
    }

    Ok(())
}

pub fn get_children(pid: u32) -> AnyResult<Vec<u32>> {
    let stdout = Command::new("pgrep")
        .arg("-P")
        .arg(pid.to_string())
        .output()?
        .stdout;

    Ok(String::from_utf8_lossy(&stdout)
        .trim()
        .to_string()
        .split("\n")
        .into_iter()
        .map(|s| s.parse::<u32>())
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap())
        .collect::<Vec<u32>>())
}
