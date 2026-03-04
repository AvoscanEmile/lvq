use std::path::PathBuf;
use std::str::FromStr;
use crate::core::{Command, SizeUnit, LvRequest};

pub fn parse_provision(args: &[String]) -> Result<Command, String> {

    let mut pvs = Vec::new();
    let mut vg_name = String::new();
    let mut pe_size = SizeUnit::Megabytes(4); // Default per your requirement
    let mut lvs = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--pv" => {
                if let Some(val) = args.get(i + 1) {
                    if !val.starts_with('-') {
                        for path in val.split(':').filter(|s| !s.is_empty()) {
                            pvs.push(PathBuf::from(path));
                        }
                        i += 2;
                    } else {
                        return Err(format!("Expected value after {}, found flag '{}'", args[i], val))
                    }
                } else {
                    return Err(format!("Missing value for {}", args[i]));
                }
            }
            "--vg" => {
                if !vg_name.is_empty() {
                    return Err("Volume Group can only be specified once".to_string());
                }
                if let Some(val) = args.get(i + 1) {
                    if !val.starts_with('-') {
                        let parts: Vec<&str> = val.split(':').collect();
                        vg_name = parts[0].to_string();
                        if parts.len() > 1 && !parts[1].is_empty() {
                            pe_size = SizeUnit::from_str(parts[1])?;
                        }
                        i += 2;
                    } else {
                        return Err(format!("Expected value after {}, found flag '{}'", args[i], val))
                    }
                } else {
                    return Err(format!("Missing value for {}", args[i]));
                }
            }
            "--lv" => {
                if let Some(val) = args.get(i + 1) {
                    if !val.starts_with('-') {
                        lvs.push(LvRequest::from_str(val)?);
                        i += 2;
                    } else {
                        return Err(format!("Expected value after {}, found flag '{}'", args[i], val))
                    }
                } else {
                     return Err(format!("Missing value for {}", args[i]));
                }
            }
            "-y" | "--auto-confirm" | "provision" => { i += 1 }
            _ => return Err(format!("Unknown argument: {}", args[i])),
        }
    }

    if vg_name.is_empty() || pvs.is_empty() || lvs.is_empty() {
        return Err("Provisioning requires at least one --pv, a --vg name, and a valid --lv structure".to_string());
    }

    let parsed_command = Command::Provision { pvs, vg_name, pe_size, lvs };
    return Ok(parsed_command) 

}
