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
    let parent = pid_ppid(pid).expect("parent");
    assert_ne!(parent, 0);
    assert!(pid_is_alive(parent));
    assert!(
        pid_children(parent).contains(&pid),
        "parent {parent} should list child {pid}"
    );
}
