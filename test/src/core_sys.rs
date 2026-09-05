use ggok_core::sys::{pid_cmdline, pid_is_alive};

#[test]
fn current_process_is_alive() {
    let pid = std::process::id();
    assert!(pid_is_alive(pid));
    assert!(!pid_is_alive(u32::MAX));
}

#[test]
fn cmdline_of_self_is_some() {
    let pid = std::process::id();
    let cmd = pid_cmdline(pid);
    assert!(!cmd.is_empty(), "expected cmdline for pid {pid}");
}
