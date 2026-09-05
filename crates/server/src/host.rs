use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static WAN: Mutex<WanCache> = Mutex::new(WanCache {
    ip: None,
    tried: false,
});

static CPU: Mutex<Option<CpuSample>> = Mutex::new(None);

struct WanCache {
    ip: Option<String>,
    tried: bool,
}

struct CpuSample {
    idle: u64,
    total: u64,
    at: Instant,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostStatus {
    pub user: String,
    pub hostname: String,
    pub ipv4_lan: Option<String>,
    pub ipv4_wan: Option<String>,
    pub cpu: CpuStatus,
    pub memory: MemStatus,
    pub disks: Vec<DiskStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuStatus {
    pub load1: f64,
    pub load5: f64,
    pub load15: f64,
    pub percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemStatus {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskStatus {
    pub path: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

pub fn snapshot(grok_home: &Path) -> HostStatus {
    HostStatus {
        user: current_user(),
        hostname: hostname(),
        ipv4_lan: lan_ipv4(),
        ipv4_wan: wan_ipv4(),
        cpu: cpu_status(),
        memory: mem_status(),
        disks: disk_status(grok_home),
    }
}

pub fn current_user() -> String {
    if let Ok(u) = std::env::var("USER")
        && !u.is_empty()
    {
        return u;
    }
    if let Ok(raw) = fs::read_to_string("/proc/self/status") {
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                let uid = rest.split_whitespace().next().unwrap_or("");
                if let Ok(passwd) = fs::read_to_string("/etc/passwd") {
                    for row in passwd.lines() {
                        let parts: Vec<&str> = row.split(':').collect();
                        if parts.len() > 2 && parts[2] == uid.trim() {
                            return parts[0].to_string();
                        }
                    }
                }
                return uid.trim().to_string();
            }
        }
    }
    if let Ok(out) = std::process::Command::new("id").arg("-un").output()
        && let Ok(s) = String::from_utf8(out.stdout)
    {
        let name = s.trim();
        if !name.is_empty() {
            return name.to_string();
        }
    }
    String::from("—")
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| String::from("—"))
}

fn lan_ipv4() -> Option<String> {
    if let Ok(out) = std::process::Command::new("hostname").arg("-I").output()
        && let Ok(s) = String::from_utf8(out.stdout)
    {
        for part in s.split_whitespace() {
            if part.parse::<std::net::Ipv4Addr>().is_ok() && !part.starts_with("127.") {
                return Some(part.to_string());
            }
        }
    }
    lan_ipv4_compat()
}

fn wan_ipv4() -> Option<String> {
    if let Ok(cache) = WAN.lock()
        && cache.tried
    {
        return cache.ip.clone();
    }
    let ip = fetch_wan();
    if let Ok(mut cache) = WAN.lock() {
        cache.ip.clone_from(&ip);
        cache.tried = true;
    }
    ip
}

fn fetch_wan() -> Option<String> {
    std::thread::spawn(|| {
        let addr = "ipv4.icanhazip.com:80".to_socket_addrs().ok()?.next()?;
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).ok()?;
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .ok()?;
        stream
            .write_all(b"GET / HTTP/1.0\r\nHost: ipv4.icanhazip.com\r\nConnection: close\r\n\r\n")
            .ok()?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).ok()?;
        let text = String::from_utf8_lossy(&buf);
        let body = text.split("\r\n\r\n").nth(1)?.trim();
        let ip = body.lines().next()?.trim();
        ip.parse::<std::net::Ipv4Addr>().ok()?;
        Some(ip.to_string())
    })
    .join()
    .ok()
    .flatten()
}

use std::net::ToSocketAddrs;

fn cpu_status() -> CpuStatus {
    let (one, five, fifteen) = loadavg();
    CpuStatus {
        load1: one,
        load5: five,
        load15: fifteen,
        percent: cpu_percent(),
    }
}

fn loadavg() -> (f64, f64, f64) {
    let Ok(raw) = fs::read_to_string("/proc/loadavg") else {
        return loadavg_compat();
    };
    let mut it = raw.split_whitespace();
    let a = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let b = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let c = it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    (a, b, c)
}

fn cpu_percent() -> Option<f64> {
    if std::fs::metadata("/proc/stat").is_err() {
        return cpu_percent_compat();
    }
    let (idle, total) = read_proc_stat()?;
    let now = Instant::now();
    let mut slot = CPU.lock().ok()?;
    let percent = if let Some(prev) = slot.as_ref() {
        let dt = now.saturating_duration_since(prev.at).as_secs_f64();
        if dt <= 0.0 {
            None
        } else {
            cpu_used_percent(
                idle.saturating_sub(prev.idle),
                total.saturating_sub(prev.total),
            )
        }
    } else {
        None
    };
    *slot = Some(CpuSample {
        idle,
        total,
        at: now,
    });
    percent
}

fn read_proc_stat() -> Option<(u64, u64)> {
    let raw = fs::read_to_string("/proc/stat").ok()?;
    let line = raw.lines().next()?;
    if !line.starts_with("cpu ") {
        return None;
    }
    let nums: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() < 4 {
        return None;
    }
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total = nums.iter().sum();
    Some((idle, total))
}

fn mem_status() -> MemStatus {
    let Ok(raw) = fs::read_to_string("/proc/meminfo") else {
        return mem_status_compat();
    };
    let mut total_kb = 0_u64;
    let mut avail_kb = 0_u64;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb(v);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail_kb = parse_kb(v);
        }
    }
    let total = total_kb.saturating_mul(1024);
    let used = total.saturating_sub(avail_kb.saturating_mul(1024));
    MemStatus {
        used_bytes: used,
        total_bytes: total,
    }
}

fn parse_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn disk_status(grok_home: &Path) -> Vec<DiskStatus> {
    let mut disks = Vec::new();
    if let Some(root) = df_one("/") {
        disks.push(root);
    }
    if let Some(home) = df_one(&grok_home.to_string_lossy()) {
        if disks
            .first()
            .is_none_or(|d| d.total_bytes != home.total_bytes || d.used_bytes != home.used_bytes)
        {
            disks.push(DiskStatus {
                path: grok_home.to_string_lossy().into_owned(),
                used_bytes: home.used_bytes,
                total_bytes: home.total_bytes,
            });
        }
    }
    disks
}

fn cpu_used_percent(didle: u64, dtotal: u64) -> Option<f64> {
    if dtotal == 0 {
        return None;
    }
    let used = dtotal.saturating_sub(didle);
    let hundredths = (u128::from(used) * 10_000 / u128::from(dtotal)).min(10_000);
    let hundredths = u16::try_from(hundredths).unwrap_or(10_000);
    Some(f64::from(hundredths) / 100.0)
}

fn df_one(path: &str) -> Option<DiskStatus> {
    parse_df(path, &["-B1", "-P"]).or_else(|| parse_df_k(path))
}

fn parse_df(path: &str, args: &[&str]) -> Option<DiskStatus> {
    let mut cmd = std::process::Command::new("df");
    cmd.args(args).arg(path);
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let line = text.lines().nth(1)?;
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 6 {
        return None;
    }
    let total: u64 = cols[1].parse().ok()?;
    let used: u64 = cols[2].parse().ok()?;
    Some(DiskStatus {
        path: cols[5].to_string(),
        used_bytes: used,
        total_bytes: total,
    })
}

fn parse_df_k(path: &str) -> Option<DiskStatus> {
    let mut st = parse_df(path, &["-kP"])?;
    st.used_bytes = st.used_bytes.saturating_mul(1024);
    st.total_bytes = st.total_bytes.saturating_mul(1024);
    Some(st)
}

fn lan_ipv4_compat() -> Option<String> {
    for iface in ["en0", "en1", "en2", "en3", "en4", "en5", "eth0"] {
        if let Ok(out) = std::process::Command::new("ipconfig")
            .args(["getifaddr", iface])
            .output()
            && out.status.success()
            && let Ok(s) = String::from_utf8(out.stdout)
        {
            let ip = s.trim();
            if ip.parse::<std::net::Ipv4Addr>().is_ok() && !ip.starts_with("127.") {
                return Some(ip.to_string());
            }
        }
    }
    let out = std::process::Command::new("ifconfig").arg("-a").output().ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    for part in s.split_whitespace() {
        if part.parse::<std::net::Ipv4Addr>().is_ok()
            && !part.starts_with("127.")
            && !part.starts_with("169.254.")
        {
            return Some(part.to_string());
        }
    }
    None
}

fn loadavg_compat() -> (f64, f64, f64) {
    let Ok(out) = std::process::Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
    else {
        return (0.0, 0.0, 0.0);
    };
    let Ok(s) = String::from_utf8(out.stdout) else {
        return (0.0, 0.0, 0.0);
    };
    let nums: Vec<f64> = s
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter(|x| !x.is_empty())
        .filter_map(|x| x.parse().ok())
        .collect();
    if nums.len() >= 3 {
        (nums[0], nums[1], nums[2])
    } else {
        (0.0, 0.0, 0.0)
    }
}

fn cpu_percent_compat() -> Option<f64> {
    let out = std::process::Command::new("top")
        .args(["-l", "1", "-n", "0", "-s", "0"])
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if !lower.contains("cpu") || !lower.contains("idle") {
            continue;
        }
        let idle = line
            .split_whitespace()
            .filter_map(|w| w.strip_suffix('%'))
            .filter_map(|w| w.parse::<f64>().ok())
            .next_back()?;
        return Some((100.0 - idle).clamp(0.0, 100.0));
    }
    None
}

fn mem_status_compat() -> MemStatus {
    let total = sysctl_u64("hw.memsize").unwrap_or(0);
    let page = sysctl_u64("hw.pagesize").unwrap_or(4096);
    let free = sysctl_u64("vm.page_free_count").unwrap_or(0);
    let spec = sysctl_u64("vm.page_speculative_count").unwrap_or(0);
    let purge = sysctl_u64("vm.page_purgeable_count").unwrap_or(0);
    let reusable = sysctl_u64("vm.page_reusable_count").unwrap_or(0);
    let avail = free
        .saturating_add(spec)
        .saturating_add(purge)
        .saturating_add(reusable)
        .saturating_mul(page)
        .min(total);
    MemStatus {
        used_bytes: total.saturating_sub(avail),
        total_bytes: total,
    }
}

fn sysctl_u64(name: &str) -> Option<u64> {
    let out = std::process::Command::new("sysctl")
        .args(["-n", name])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()?.trim().parse().ok()
}
