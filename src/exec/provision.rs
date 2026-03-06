use crate::core::{Call, Draft, SizeUnit, PercentTarget, Exec};

pub fn exec_provision(draft: Draft) -> Result<Exec, String> {
    let mut command_list = Vec::new();
    let has_fstab_calls = draft.draft.iter().any(|c| matches!(c, Call::Fstab { .. }));

    if has_fstab_calls {
        command_list.push(
            "cp -p /etc/fstab /etc/fstab.bak".to_string()
        );
    } 

    for call in &draft.draft {

        let cmd_string = match call {
            Call::PvCreate(path) => {
                format!("pvcreate -y {:?}", path)
            }
            Call::VgCreate { name, pvs, pe_size } => {
                let pvs_str: Vec<String> = pvs.iter().map(|p| format!("{:?}", p)).collect();
                // PE size is converted to bytes for precision in the shell command
                format!("vgcreate -s {}B {} {}", pe_size.to_bytes()?, name, pvs_str.join(" "))
            }
            Call::LvCreate { vg, name, size } => {
                match size {
                    SizeUnit::Percentage(pct, target) => {
                        let t = match target {
                            PercentTarget::Free => "FREE",
                            PercentTarget::Vg => "VG",
                            PercentTarget::Pvs => "PVS",
                        };
                        format!("lvcreate -y -n {} -l {}%{} {}", name, pct.get(), t, vg)
                    }
                    SizeUnit::Extents(e) => {
                        format!("lvcreate -y -n {} -l {} {}", name, e, vg)
                    }
                    _ => {
                        format!("lvcreate -y -n {} -L {}B {}", name, size.to_bytes()?, vg)
                    }
                }
            }
            Call::Mkfs { device, fs } => {
                let fs_name = format!("{:?}", fs).to_lowercase();
                format!("mkfs -t {} {:?}", fs_name, device)
            }
            Call::Mkdir(path) => format!("mkdir -p {:?}", path),
            Call::Mount { device, path } => format!("mount {:?} {:?}", device, path),
            Call::Fstab { device, path, fs } => {
                let fs_name = format!("{:?}", fs).to_lowercase();
                let is_swap = matches!(fs, crate::core::Filesystem::Swap);

                let dev_str = device.to_str().unwrap_or_default();
                let path_str = path.to_str().unwrap_or_default();
                let opts = if is_swap { "sw" } else { "defaults" };
                let pass = if is_swap { "0" } else { "2" };

                command_list.push(format!("cp -p /etc/fstab /etc/fstab.{}.tmp", fs_name));

                command_list.push(format!("ID=$(blkid -s UUID -o value {dev}); if [ -z \"$ID\" ]; then ID={dev}; else ID=\"UUID=$ID\"; fi; echo \"$ID {mnt} {fs} {opts} 0 {pass}\" >> /etc/fstab.{fs}.tmp", dev = dev_str, mnt = path_str, fs = fs_name, opts = opts, pass = pass ));

                command_list.push(format!("mv /etc/fstab.{}.tmp /etc/fstab", fs_name));
                continue; 
            }
            Call::MkSwap(device) => {
                let dev_str = device.to_str().unwrap_or_default();
                command_list.push(format!("mkswap {}", dev_str));
                command_list.push(format!("swapon {}", dev_str));
                continue;
            }
        };
        command_list.push(cmd_string);
    }

    if has_fstab_calls { command_list.push("systemctl daemon-reload".to_string()); }

    Ok(Exec {
        list: command_list,
        auto_confirm: draft.auto_confirm,
        is_allowed: false,
        warnings: draft.warnings,
    })
}
