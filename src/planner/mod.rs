use crate::core::{Action, Command, Draft};
mod provision;

pub fn plan(action: Action) -> Result<Draft, String> {
    match action.command {
        Command::Provision { pvs, vg_name, pe_size, lvs } => {
            let calls = plan_provision(pvs, vg_name, pe_size, lvs)?;
            Ok(Draft { draft_type: "provision".to_string(), draft: calls, })
        }
    }
}

