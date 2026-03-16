use std::process::Command;
use crate::core::{Call, Draft, Exec, Instruction, PercentTarget, SizeUnit, Filesystem};

pub fn exec_provision(draft: Draft) -> Result<Exec, String> {
    let mut instructions = Vec::new();
    let has_fstab_calls = draft.draft.iter().any(|c| matches!(c, Call::Fstab { .. }));

    // 1. Initial fstab backup
    if has_fstab_calls {
        let shell_string = "cp -p /etc/fstab /etc/fstab.bak".to_string();
        let mut command_call = Command::new("cp");
        command_call.arg("-p").arg("/etc/fstab").arg("/etc/fstab.bak");
        
        instructions.push(Instruction { shell_string, command_call });
    }

    for call in &draft.draft {
        match call {
            Call::PvCreate(path) => {
                let path_str = path.to_string_lossy();
                let shell_string = format!("pvcreate -y \"{}\"", path_str);
                let mut command_call = Command::new("pvcreate");
                command_call.arg("-y").arg(path);
                
                instructions.push(Instruction { shell_string, command_call });
            }

            Call::VgCreate { name, pvs, pe_size } => {
                let pe_bytes = pe_size.to_bytes()?;
                let pvs_str: Vec<String> = pvs.iter().map(|p| format!("\"{}\"", p.to_string_lossy())).collect();
                let shell_string = format!("vgcreate -s {}B {} {}", pe_bytes, name, pvs_str.join(" "));
                
                let mut command_call = Command::new("vgcreate");
                command_call.arg("-s").arg(format!("{}B", pe_bytes)).arg(name).args(pvs);
                
                instructions.push(Instruction { shell_string, command_call });
            }

            Call::LvCreate { vg, name, size } => {
                let mut command_call = Command::new("lvcreate");
                command_call.arg("-y").arg("-n").arg(name);

                let shell_string = match size {
                    SizeUnit::Percentage(pct, target) => {
                        let t = match target {
                            PercentTarget::Free => "FREE",
                            PercentTarget::Vg => "VG",
                            PercentTarget::Pvs => "PVS",
                        };
                        let val = format!("{}%{}", pct.get(), t);
                        command_call.arg("-l").arg(&val).arg(vg);
                        format!("lvcreate -y -n {} -l {} {}", name, val, vg)
                    }
                    SizeUnit::Extents(e) => {
                        command_call.arg("-l").arg(e.to_string()).arg(vg);
                        format!("lvcreate -y -n {} -l {} {}", name, e, vg)
                    }
                    _ => {
                        let bytes = format!("{}B", size.to_bytes()?);
                        command_call.arg("-L").arg(&bytes).arg(vg);
                        format!("lvcreate -y -n {} -L {} {}", name, bytes, vg)
                    }
                };
                instructions.push(Instruction { shell_string, command_call });
            }

            Call::Mkfs { device, fs } => {
                let fs_name = format!("{}", fs);
                let dev_str = device.to_string_lossy();
                let shell_string = format!("mkfs -t {} \"{}\"", fs_name, dev_str);
                
                let mut command_call = Command::new("mkfs");
                command_call.arg("-t").arg(fs_name).arg(device);
                
                instructions.push(Instruction { shell_string, command_call });
            }

            Call::Mkdir(path) => {
                let path_str = path.to_string_lossy();
                let shell_string = format!("mkdir -p \"{}\"", path_str);
                
                let mut command_call = Command::new("mkdir");
                command_call.arg("-p").arg(path);
                
                instructions.push(Instruction { shell_string, command_call });
            }

            Call::Mount { device, path } => {
                let dev_str = device.to_string_lossy();
                let path_str = path.to_string_lossy();
                let shell_string = format!("mount \"{}\" \"{}\"", dev_str, path_str);
                
                let mut command_call = Command::new("mount");
                command_call.arg(device).arg(path);
                
                instructions.push(Instruction { shell_string, command_call });
            }

            Call::Fstab { device, path, fs } => {
                let fs_name = format!("{}", fs);
                let dev_str = device.to_string_lossy();
                let path_str = path.to_string_lossy();
                let is_swap = matches!(fs, Filesystem::Swap);
                let opts = if is_swap { "sw" } else { "defaults" };
                let pass = if is_swap { "0" } else { "2" };
                let mnt_point = if is_swap { "none" } else { &path_str };

                // 1. Temp file creation
                let cp_str = format!("cp -p /etc/fstab /etc/fstab.{}.tmp", fs_name);
                let mut cp_cmd = Command::new("cp");
                cp_cmd.arg("-p").arg("/etc/fstab").arg(format!("/etc/fstab.{}.tmp", fs_name));
                instructions.push(Instruction { shell_string: cp_str, command_call: cp_cmd });

                // 2. The logic string (The only one that uses 'sh -c' because of the redirections/logic)
                let logic_shell = format!(
                    "ID=$(blkid -s UUID -o value {dev}); if [ -z \"$ID\" ]; then ID={dev}; else ID=\"UUID=$ID\"; fi; echo \"$ID {mnt} {fs} {opts} 0 {pass}\" >> /etc/fstab.{fs}.tmp",
                    dev = dev_str, mnt = mnt_point, fs = fs_name, opts = opts, pass = pass
                );
                let mut logic_cmd = Command::new("sh");
                logic_cmd.arg("-c").arg(&logic_shell);
                instructions.push(Instruction { shell_string: logic_shell, command_call: logic_cmd });

                // 3. Atomic rename
                let mv_str = format!("mv /etc/fstab.{}.tmp /etc/fstab", fs_name);
                let mut mv_cmd = Command::new("mv");
                mv_cmd.arg(format!("/etc/fstab.{}.tmp", fs_name)).arg("/etc/fstab");
                instructions.push(Instruction { shell_string: mv_str, command_call: mv_cmd });
            }

            Call::MkSwap(device) => {
                let dev_str = device.to_string_lossy();
                
                // mkswap
                let mut mk_cmd = Command::new("mkswap");
                mk_cmd.arg(device);
                instructions.push(Instruction { shell_string: format!("mkswap \"{}\"", dev_str), command_call: mk_cmd });
                
                // swapon
                let mut on_cmd = Command::new("swapon");
                on_cmd.arg(device);
                instructions.push(Instruction { shell_string: format!("swapon \"{}\"", dev_str), command_call: on_cmd });
            }
        }
    }

    if has_fstab_calls {
        let mut reload_cmd = Command::new("systemctl");
        reload_cmd.arg("daemon-reload");
        instructions.push(Instruction { 
            shell_string: "systemctl daemon-reload".to_string(), 
            command_call: reload_cmd 
        });
    }

    Ok(Exec {
        list: instructions,
        auto_confirm: draft.auto_confirm,
        is_allowed: false,
        warnings: draft.warnings,
    })
}
