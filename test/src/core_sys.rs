use ggok_core::sys::{pid_children, pid_cmdline, pid_is_alive, pid_ppid};

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

#[test]
fn pid_ppid_of_self_is_alive() {
    let pid = std::process::id();
    let ppid = pid_ppid(pid).expect("ppid");
    assert_ne!(ppid, 0);
    assert!(pid_is_alive(ppid));
    assert!(
        pid_children(ppid).contains(&pid),
        "parent {ppid} should list child {pid}"
    );
}
