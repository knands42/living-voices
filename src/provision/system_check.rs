use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "linux")]
const DRM_ROOT: &str = "/sys/class/drm";

const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsCard {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gpu {
    pub card: GraphicsCard,
    pub vram_mb: Option<u64>,
}

impl Gpu {
    pub const UNKNOWN: Gpu = Gpu {
        card: GraphicsCard::Unknown,
        vram_mb: None,
    };

    #[cfg(target_os = "linux")]
    pub fn detect() -> Gpu {
        scan(Path::new(DRM_ROOT))
            .iter()
            .map(Device::probe)
            .fold(Gpu::UNKNOWN, Gpu::prefer_more_vram)
    }

    #[cfg(target_os = "windows")]
    pub fn detect() -> Gpu {
        if let Some(vram_mb) = nvidia_smi_mb() {
            return Gpu {
                card: GraphicsCard::Nvidia,
                vram_mb: Some(vram_mb),
            };
        }

        let name = powershell(
            "(Get-CimInstance Win32_VideoController | Select-Object -First 1 -ExpandProperty Name)",
        )
        .unwrap_or_default();

        Gpu {
            card: vendor_from_name(&name),
            vram_mb: adapter_memory_mb(),
        }
    }

    /// On a hybrid machine the discrete card answers for ComfyUI, not the iGPU
    /// that reports no VRAM of its own.
    #[cfg(target_os = "linux")]
    fn prefer_more_vram(best: Gpu, candidate: Gpu) -> Gpu {
        if candidate.vram_mb.unwrap_or(0) > best.vram_mb.unwrap_or(0) {
            candidate
        } else {
            best
        }
    }
}

#[cfg(target_os = "linux")]
struct Device {
    sysfs: PathBuf,
}

#[cfg(target_os = "linux")]
impl Device {
    fn probe(&self) -> Gpu {
        let card = self.vendor();

        Gpu {
            card,
            vram_mb: self.memory_mb(card),
        }
    }

    fn vendor(&self) -> GraphicsCard {
        fs::read_to_string(self.sysfs.join("vendor"))
            .map(|id| vendor_from_id(&id))
            .unwrap_or(GraphicsCard::Unknown)
    }

    fn memory_mb(&self, card: GraphicsCard) -> Option<u64> {
        match card {
            GraphicsCard::Amd => sysfs_mib(&self.sysfs.join("mem_info_vram_total")),
            GraphicsCard::Nvidia => nvidia_smi_mb(),
            GraphicsCard::Intel | GraphicsCard::Unknown => None,
        }
    }
}

#[cfg(any(target_os = "linux", test))]
fn scan(root: &Path) -> Vec<Device> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut devices: Vec<Device> = entries
        .flatten()
        .filter(|entry| is_card(&entry.file_name()))
        .map(|entry| Device {
            sysfs: entry.path().join("device"),
        })
        .collect();

    devices.sort_by(|left, right| left.sysfs.cmp(&right.sysfs));
    devices
}

#[cfg(target_os = "linux")]
fn sysfs_mib(path: &Path) -> Option<u64> {
    let bytes = fs::read_to_string(path).ok()?;
    let bytes = bytes.trim().parse::<u64>().ok()?;

    (bytes > 0).then(|| bytes / MIB)
}

#[cfg(any(target_os = "linux", test))]
fn is_card(name: &OsStr) -> bool {
    name.to_str()
        .and_then(|name| name.strip_prefix("card"))
        .is_some_and(|index| !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(any(target_os = "linux", test))]
fn vendor_from_id(id: &str) -> GraphicsCard {
    match id.trim().to_ascii_lowercase().as_str() {
        "0x10de" => GraphicsCard::Nvidia,
        "0x1002" => GraphicsCard::Amd,
        "0x8086" => GraphicsCard::Intel,
        _ => GraphicsCard::Unknown,
    }
}

fn nvidia_smi_mb() -> Option<u64> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_mib_lines(&String::from_utf8_lossy(&output.stdout))
}

fn parse_mib_lines(stdout: &str) -> Option<u64> {
    stdout
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .max()
}

#[cfg(target_os = "windows")]
fn powershell(script: &str) -> Option<String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `Win32_VideoController.AdapterRAM` is a u32 and wraps past 4 GiB, so the
/// registry QWORD is the only value that survives a modern card.
#[cfg(target_os = "windows")]
fn adapter_memory_mb() -> Option<u64> {
    const ADAPTER_CLASS: &str =
        r"HKLM\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000";

    let output = Command::new("reg")
        .args([
            "query",
            ADAPTER_CLASS,
            "/v",
            "HardwareInformation.qwMemorySize",
        ])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let hex = text
        .lines()
        .find(|line| line.contains("qwMemorySize"))?
        .split_whitespace()
        .last()?;

    let bytes = u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok()?;

    (bytes > 0).then(|| bytes / MIB)
}

#[cfg(target_os = "windows")]
fn vendor_from_name(name: &str) -> GraphicsCard {
    let name = name.to_ascii_lowercase();

    if name.contains("nvidia") {
        GraphicsCard::Nvidia
    } else if name.contains("amd") || name.contains("radeon") {
        GraphicsCard::Amd
    } else if name.contains("intel") {
        GraphicsCard::Intel
    } else {
        GraphicsCard::Unknown
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn connectors_are_not_cards() {
        // given the names /sys/class/drm actually lists
        // when
        // then
        assert!(is_card(OsStr::new("card0")));
        assert!(is_card(OsStr::new("card12")));
        assert!(!is_card(OsStr::new("card0-DP-1")));
        assert!(!is_card(OsStr::new("renderD128")));
        assert!(!is_card(OsStr::new("card")));
        assert!(!is_card(OsStr::new("version")));
    }

    #[test]
    fn vendor_ids_map_to_their_card() {
        // given sysfs writes the id with a trailing newline
        // when
        // then
        assert_eq!(vendor_from_id("0x10de\n"), GraphicsCard::Nvidia);
        assert_eq!(vendor_from_id("0x1002\n"), GraphicsCard::Amd);
        assert_eq!(vendor_from_id("0x8086\n"), GraphicsCard::Intel);
        assert_eq!(vendor_from_id("0x1234\n"), GraphicsCard::Unknown);
    }

    #[test]
    fn smi_reports_the_largest_card_and_ignores_noise() {
        // given one line per GPU, largest last, plus the shapes that mean "no answer"
        // when
        // then
        assert_eq!(parse_mib_lines("8192\n24564\n"), Some(24564));
        assert_eq!(parse_mib_lines("[N/A]\n"), None);
        assert_eq!(parse_mib_lines(""), None);
    }

    #[test]
    fn amd_vram_comes_from_sysfs_and_connectors_are_skipped() {
        // given a fake /sys/class/drm holding one AMD card and one connector
        let root = std::env::temp_dir().join(format!("lv_drm_{}", uuid::Uuid::new_v4()));
        let sysfs = root.join("card0").join("device");
        fs::create_dir_all(&sysfs).expect("create card");
        fs::create_dir_all(root.join("card0-DP-1")).expect("create connector");
        fs::write(sysfs.join("vendor"), "0x1002\n").expect("write vendor");
        fs::write(sysfs.join("mem_info_vram_total"), "17179869184\n").expect("write vram");

        // when
        let gpus: Vec<Gpu> = scan(&root).iter().map(Device::probe).collect();
        fs::remove_dir_all(&root).expect("clean up");

        // then
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].card, GraphicsCard::Amd);
        assert_eq!(gpus[0].vram_mb, Some(16384));
    }
}
