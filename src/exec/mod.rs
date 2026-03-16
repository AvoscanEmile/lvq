use std::fs::OpenOptions;
use std::io::{self, Write};
use crate::core::Exec;
pub mod provision;

pub fn confirm_execution(exec: &mut Exec) -> Result<(), String> {
    if exec.auto_confirm {
        exec.is_allowed = true;
        return Ok(());
    }

    if !exec.warnings.is_empty() {
        println!("--- WARNINGS ---");
        for warning in &exec.warnings {
            println!("{}", warning);
        }
    }

    println!("\n--- PENDING SYSTEM CHANGES ---");
    for (i, instruction) in exec.list.iter().enumerate() {
        // We print the shell_string to maintain the "What You See Is What You Get" contract
        println!("{:2}. {}", i + 1, instruction.shell_string);
    }
    println!("------------------------------");
    print!("\nExecute these commands? [Y/n]: ");
    io::stdout().flush().map_err(|e| format!("Terminal error: {e}"))?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| format!("Input error: {e}"))?;
    
    if input.trim() == "Y" {
        exec.is_allowed = true;
        Ok(())
    } else {
        exec.is_allowed = false;
        Err("Execution aborted by user.".to_string())
    }
}

pub fn apply_execution(exec: Exec) -> Result<(), String> {
    if !exec.is_allowed {
        return Err("Security Error: Attempted to apply an unauthorized execution plan.".into());
    }

    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/var/log/lvq")
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    writeln!(log, "\n--- Start of Transaction ---").ok();
    writeln!(log, "Full execution plan:").ok();
    for instruction in &exec.list {
        writeln!(log, "  {}", instruction.shell_string).ok();
    }
    writeln!(log, "------------------------------").ok();
    writeln!(log, "\nExecution Log:").ok();

    // Consume the instructions
    for mut instruction in exec.list {
        let cmd_display = &instruction.shell_string;
        writeln!(log, "INTENT: {}", cmd_display).ok();

        // Execute the native command object built during the provision phase
        let status = instruction.command_call.status()
            .map_err(|e| format!("Process error for [{}]: {}", cmd_display, e))?;

        if status.success() {
            writeln!(log, "SUCCESS: {}", cmd_display).ok();
        } else {
            writeln!(log, "FAILED: {}", cmd_display).ok();
            return Err(format!("Command [{}] failed with exit code: {:?}", cmd_display, status.code()));
        }
    }

    writeln!(log, "--- End of Transaction (SUCCESS) ---\n").ok();
    Ok(())
}

#[cfg(test)]
mod tests;
