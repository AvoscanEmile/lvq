use std::path::{Path};
use std::process::Command;
use crate::core::{Call, Draft, DraftStatus};

fn probe_pv_exists(path: &Path) -> bool {
    Command::new("pvs")
        .args(["--reportformat", "json", path.to_str().unwrap_or_default()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn probe_vg_exists(name: &str) -> bool {
    Command::new("vgs")
        .args(["--reportformat", "json", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn probe_lv_exists(vg: &str, name: &str) -> bool {
    let lv_path = format!("{}/{}", vg, name);
    Command::new("lvs")
        .args(["--reportformat", "json", &lv_path])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn probe_fs_exists(path: &Path) -> bool {
    Command::new("blkid")
        .arg(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn probe_mount_exists(target_path: &Path) -> bool {
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        let path_str = target_path.to_str().unwrap_or_default();
        mounts.lines().any(|line| line.contains(path_str))
    } else {
        false
    }
}

fn probe_swap_active(path: &Path) -> bool {
    if let Ok(swaps) = std::fs::read_to_string("/proc/swaps") {
        let path_str = path.to_str().unwrap_or_default();
        swaps.lines().any(|line| line.contains(path_str))
    } else {
        false
    }
}

fn probe_fstab_exists(device: &Path, mount_path: &Path) -> bool {
    let fstab = std::fs::read_to_string("/etc/fstab").unwrap_or_default();
    
    let mnt_str = mount_path.to_str().unwrap_or_default();
    if !mnt_str.is_empty() && mnt_str != "none" {
        if fstab.lines().any(|l| !l.starts_with('#') && l.contains(mnt_str)) {
            return true;
        }
    }

    if let Ok(output) = Command::new("blkid").args(["-s", "UUID", "-o", "value", device.to_str().unwrap_or_default()]).output() {
        let uuid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !uuid.is_empty() && fstab.lines().any(|l| !l.starts_with('#') && l.contains(&uuid)) {
            return true;
        }
    }

    let dev_str = device.to_str().unwrap_or_default();
    fstab.lines().any(|l| !l.starts_with('#') && l.contains(dev_str))
}

fn probe_block_device_size(path: &Path) -> Result<u64, String> {
    let output = Command::new("lsblk")
        .args(["-b", "-n", "-o", "SIZE", path.to_str().unwrap_or_default()])
        .output()
        .map_err(|e| format!("Failed to execute lsblk: {}", e))?;

    if !output.status.success() {
        return Err(format!("lsblk failed for device {:?}", path));
    }

    let size_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    size_str.parse::<u64>().map_err(|_| format!("Failed to parse size: {}", size_str))
}

fn probe_is_full_disk(path: &Path) -> bool {
    Command::new("lsblk")
        .args(["-n", "-d", "-o", "TYPE", path.to_str().unwrap_or_default()])
        .output()
        .map(|o| {
            let out = String::from_utf8_lossy(&o.stdout);
            let dev_type = out.trim();
            !dev_type.is_empty() && dev_type != "part"
        })
        .unwrap_or(false)
}

fn verify_done(draft: &mut Draft) -> Result<(), String> {
    let mut matched_calls = 0;
    let mut total_calls = draft.draft.len();

    for call in &draft.draft {
        match call {
            Call::PvCreate(path) => if probe_pv_exists(path) { matched_calls += 1; },
            Call::VgCreate { name, .. } => if probe_vg_exists(name) { matched_calls += 1; },
            Call::LvCreate { vg, name, .. } => if probe_lv_exists(vg, name) { matched_calls += 1; },
            Call::Mount { path, .. } => if probe_mount_exists(path) { matched_calls += 1; },
            Call::Fstab { device, path, .. } => if probe_fstab_exists(device, path) { matched_calls += 1; },
            Call::MkSwap(device) => if probe_swap_active(device) { matched_calls += 1; }
            Call::Mkfs { device, .. } => if probe_fs_exists(device) { total_calls -= 1; }
            Call::Mkdir(path) => if path.exists() { total_calls -= 1; }
        };
    }

    if matched_calls == total_calls {
        draft.status = DraftStatus::Done;
    } else if matched_calls == 0 {
        draft.status = DraftStatus::Clean;
    } else {
        draft.status = DraftStatus::Dirty;
        return Err("Draft is in a dirty/partial state.".to_string());
    }

    Ok(())
}

fn verify_possible(draft: &mut Draft) -> Result<(), String> {
    if draft.status != DraftStatus::Clean {
        return Err("Cannot run capability check on a non-clean draft.".to_string());
    }

    for call in &draft.draft {
        if let Call::PvCreate(path) = call {
            if probe_is_full_disk(path) {
                draft.warnings.push(format!("Targeting full disk {:?} (not a partition). This could cause problems in the future. It is recommended to partition first, then call the program on the target partition instead.", path));
            }

            if probe_fs_exists(path) {
                if probe_fstab_exists(path, Path::new("")) {
                    draft.status = DraftStatus::Dirty;
                    return Err(format!("CRITICAL: Device {:?} has a filesystem signature that is actively referenced in /etc/fstab! Wiping it would break system boot.", path));
                } else {
                    draft.warnings.push(format!("Device {:?} contains an existing signature. PV creation will wipe this signature automatically. Make sure no crucial data will be lost", path));
                }
            }
        }
    }

    let mut total_usable_extents: u128 = 0;
    let mut total_required_extents: u128 = 0;
    let mut pe_size_bytes: u128 = 0;

    for call in &draft.draft {
        if let Call::VgCreate { pvs, pe_size, .. } = call {
            pe_size_bytes = pe_size.to_bytes()?; 
            
            for pv in pvs {
                if !pv.exists() {
                    draft.status = DraftStatus::Invalid;
                    return Err(format!("Hardware failure: Path {:?} does not exist.", pv));
                }

                let raw_size = probe_block_device_size(pv)? as u128;
                let metadata_overhead: u128 = 1048576; // 1MB overhead
                
                if raw_size <= metadata_overhead {
                    draft.status = DraftStatus::Invalid;
                    return Err(format!("Device {:?} is too small for LVM metadata.", pv));
                }

                let usable_bytes = raw_size - metadata_overhead;
                total_usable_extents += usable_bytes / pe_size_bytes; 
            }
        }
    }

    for call in &draft.draft {
        if let Call::LvCreate { size, .. } = call {
            let required_extents = match size {
                crate::core::SizeUnit::Extents(e) => *e as u128,
                
                crate::core::SizeUnit::Percentage(pct, target) => {
                    let p = pct.get() as u128;
                    match target {
                        crate::core::PercentTarget::Vg | crate::core::PercentTarget::Pvs => {
                            (total_usable_extents * p) / 100
                        },
                        crate::core::PercentTarget::Free => {
                            let free_extents = total_usable_extents.saturating_sub(total_required_extents);
                            (free_extents * p) / 100
                        }
                    }
                },
                
                _ => {
                    let lv_bytes = size.to_bytes()?;
                    (lv_bytes + pe_size_bytes - 1) / pe_size_bytes
                }
            };

            total_required_extents += required_extents;
        }
    }

    if total_usable_extents >= total_required_extents {
        draft.status = DraftStatus::Ready;
        Ok(())
    } else {
        draft.status = DraftStatus::Invalid;
        Err(format!(
            "Validation Failure: Insufficient disk space. Required {} extents, but only {} available.",
            total_required_extents, total_usable_extents
        ))
    }
}

pub fn verify_provision(mut draft: Draft) -> Result<Draft, String> {
    verify_done(&mut draft)?;

    match draft.status {
        DraftStatus::Done => return Ok(draft), // Main exits 0
        DraftStatus::Dirty => return Err("System is in a Dirty state. Manual intervention required.".into()), // Main exits 4
        DraftStatus::Clean => {
            verify_possible(&mut draft)?;

            match draft.status {
                DraftStatus::Ready => Ok(draft), // Main proceeds to Confirmation
                DraftStatus::Invalid => Err("System cannot fulfill this plan. Invalid hardware or math.".into()), // Main exits 1
                _ => Err("Architectural Error: Unexpected state after Pass 2.".into()),
            }
        }
        _ => Err("Architectural Error: Unexpected state after Pass 1.".into()),
    }
}
