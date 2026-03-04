use std::collections::{HashSet, HashMap};
use std::path::PathBuf;
use crate::core::Draft;

pub struct VgSim {
    pub name: String,
    pub total_extents: u64,
    pub free_extents: u64,
    pub pe_size_kb: u64,
}

pub struct ProvisionState {
    pub pending_pvs: HashSet<PathBuf>,
    pub pending_vgs: HashMap<String, VgSim>,
    pub pending_lvs: HashSet<String>,
    pub pending_mounts: HashSet<PathBuf>,
}

/// Worker: Simulates the provisioning draft against the system state.
pub fn verify_provision(draft: Draft) -> Result<Draft, String> {
    // We initialize the state here once logic implementation begins
    // let mut state = ProvisionState { ... };

    // TODO: Iterate through draft.draft calls, perform targeted 
    // system queries, and update ProvisionState.

    Ok(draft)
}
