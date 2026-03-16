use proptest::prelude::*;
use std::path::PathBuf;
use crate::core::*; 
use crate::exec::{apply_execution, confirm_execution};
use crate::exec::provision::exec_provision;

/// Generates a ValidPercentage between 1 and 100
pub fn arb_valid_percentage() -> impl Strategy<Value = ValidPercentage> {
    (1u8..=100u8).prop_map(|v| ValidPercentage::new(v).unwrap())
}

/// Generates any PercentTarget variant
pub fn arb_percent_target() -> impl Strategy<Value = PercentTarget> {
    prop_oneof![
        Just(PercentTarget::Free),
        Just(PercentTarget::Vg),
        Just(PercentTarget::Pvs),
    ]
}

/// Generates all variants of SizeUnit
/// We use a mix of small and large u64 values to test boundary conditions
pub fn arb_size_unit() -> impl Strategy<Value = SizeUnit> {
    prop_oneof![
        any::<u64>().prop_map(SizeUnit::Bytes),
        any::<u64>().prop_map(SizeUnit::Kilobytes),
        any::<u64>().prop_map(SizeUnit::Megabytes),
        any::<u64>().prop_map(SizeUnit::Gigabytes),
        any::<u64>().prop_map(SizeUnit::Terabytes),
        any::<u64>().prop_map(SizeUnit::Petabytes),
        // Limit Exabytes to prevent u128 overflow in to_bytes() if necessary,
        // though u128 can hold ~1.8e38, and 18EB is only ~1.8e19.
        any::<u64>().prop_map(SizeUnit::Exabytes),
        any::<u64>().prop_map(SizeUnit::Sectors),
        any::<u64>().prop_map(SizeUnit::Extents),
        (arb_valid_percentage(), arb_percent_target())
            .prop_map(|(p, t)| SizeUnit::Percentage(p, t)),
    ]
}

/// Generates all supported Filesystems
pub fn arb_filesystem() -> impl Strategy<Value = Filesystem> {
    prop_oneof![
        Just(Filesystem::Xfs),
        Just(Filesystem::Ext4),
        Just(Filesystem::Btrfs),
        Just(Filesystem::Vfat),
        Just(Filesystem::Swap),
        Just(Filesystem::F2FS),
        Just(Filesystem::Ntfs),
        Just(Filesystem::Exfat),
    ]
}

/// Generates a simple alphanumeric PathBuf to avoid shell-injection 
/// complexities during initial property testing
pub fn arb_path() -> impl Strategy<Value = PathBuf> {
    r"[a-z0-9/]{1,20}".prop_map(PathBuf::from)
}

/// Generates a Call variant
pub fn arb_call() -> impl Strategy<Value = Call> {
    prop_oneof![
        arb_path().prop_map(Call::PvCreate),
        (any::<String>(), prop::collection::vec(arb_path(), 1..5), arb_size_unit()).prop_map(|(name, pvs, pe_size)| Call::VgCreate { name, pvs, pe_size }),
        (any::<String>(), any::<String>(), arb_size_unit()).prop_map(|(vg, name, size)| Call::LvCreate { vg, name, size }),
        (arb_path(), arb_filesystem()).prop_map(|(device, fs)| Call::Mkfs { device, fs }),
        arb_path().prop_map(Call::MkSwap),
        arb_path().prop_map(Call::Mkdir),
        (arb_path(), arb_path()).prop_map(|(device, path)| Call::Mount { device, path }),
        (arb_path(), arb_path(), arb_filesystem()).prop_map(|(device, path, fs)| Call::Fstab { device, path, fs }),
    ]
}

// Ensuring arb_draft is available to compose the collection of calls
fn arb_draft() -> impl Strategy<Value = Draft> {
    (
        any::<bool>(),
        any::<String>(),
        prop::collection::vec(arb_call(), 0..20),
        prop::collection::vec(any::<String>(), 0..5),
    ).prop_map(|(auto_confirm, draft_type, draft, warnings)| Draft {
        auto_confirm,
        draft_type,
        draft,
        status: DraftStatus::Pending,
        warnings,
    })
}

proptest! {
    #[test]
    fn test_fstab_lifecycle_invariants(draft in arb_draft()) {
        let has_fstab = draft.draft.iter().any(|c| matches!(c, Call::Fstab { .. }));
        
        if let Ok(exec) = exec_provision(draft) {
            if has_fstab {
                // 1. Check start of transaction
                prop_assert_eq!(
                    &exec.list.first().unwrap().shell_string, 
                    "cp -p /etc/fstab /etc/fstab.bak",
                    "Fstab calls present but no backup command found at start."
                );

                // 2. Check end of transaction
                prop_assert_eq!(
                    &exec.list.last().unwrap().shell_string, 
                    "systemctl daemon-reload",
                    "Fstab calls present but daemon-reload missing from end."
                );

                // 3. Check the "Triple Pattern" for every fstab modification
                for (i, inst) in exec.list.iter().enumerate() {
                    // Identify the 'logic' step (the sh -c block)
                    if inst.shell_string.contains("blkid") && inst.shell_string.contains(">> /etc/fstab") {
                        // Invariant: The previous step MUST be creating the temp file
                        prop_assert!(
                            exec.list[i-1].shell_string.contains("cp -p /etc/fstab /etc/fstab."), 
                            "Fstab logic step [{}] missing preceding temp copy.", i
                        );
                        // Invariant: The next step MUST be the atomic move
                        prop_assert!(
                            exec.list[i+1].shell_string.starts_with("mv /etc/fstab."), 
                            "Fstab logic step [{}] missing following move command.", i
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_swap_atomicity_invariants(draft in arb_draft()) {
        if let Ok(exec) = exec_provision(draft) {
            for (i, inst) in exec.list.iter().enumerate() {
                if inst.shell_string.starts_with("mkswap ") {
                    // 1. Extract the device part (including quotes if present)
                    let device_part = &inst.shell_string[7..];
                    
                    // 2. Ensure we aren't at the end of the list
                    prop_assert!(
                        i + 1 < exec.list.len(), 
                        "mkswap was the last command, missing swapon for {}", 
                        device_part
                    );

                    // 3. Compare against the next instruction's shell_string
                    let next_inst = &exec.list[i + 1];
                    let expected_swapon = format!("swapon {}", device_part);
                    
                    prop_assert_eq!(
                        &next_inst.shell_string, 
                        &expected_swapon,
                        "mkswap must be immediately followed by matching swapon"
                    );
                }
            }
        }
    }

    #[test]
    fn test_vg_create_precision_invariants(draft in arb_draft()) {
        let original_draft = draft.clone();
        if let Ok(exec) = exec_provision(draft) {
            for call in original_draft.draft {
                if let Call::VgCreate { name, pvs: _, pe_size } = call {
                    // Invariant: VgCreate must use exact byte values to prevent PE alignment issues
                    let bytes = pe_size.to_bytes().unwrap();
                    let expected_flag = format!("-s {}B", bytes);
                    
                    let found = exec.list.iter().any(|cmd| {
                        cmd.shell_string.contains("vgcreate") && cmd.shell_string.contains(&name) && cmd.shell_string.contains(&expected_flag)
                    });
                    
                    prop_assert!(found, "VG creation command lost precision or was malformed for size: {:?}", pe_size);
                }
            }
        }
    }

    #[test]
    fn test_exec_struct_integrity(draft in arb_draft()) {
        let expected_confirm = draft.auto_confirm;
        let expected_warnings = draft.warnings.clone();
        
        if let Ok(exec) = exec_provision(draft) {
            // Invariant: Meta-properties of the draft must be preserved in the execution plan
            prop_assert_eq!(exec.auto_confirm, expected_confirm);
            prop_assert_eq!(exec.warnings, expected_warnings);
            prop_assert!(!exec.is_allowed, "Exec should never be pre-allowed by the provisioner logic.");
        }
    }
}

#[test]
fn test_exec_provision_error_on_invalid_size() {
    // VgCreate cannot use Percentage units because it needs a concrete byte value for Physical Extents
    let bad_call = Call::VgCreate {
        name: "vg0".to_string(),
        pvs: vec![PathBuf::from("/dev/sda")],
        pe_size: SizeUnit::Percentage(ValidPercentage::new(50).unwrap(), PercentTarget::Free),
    };

    let draft = Draft {
        auto_confirm: false,
        draft_type: "test".to_string(),
        draft: vec![bad_call],
        status: DraftStatus::Pending,
        warnings: vec![],
    };

    let result = exec_provision(draft);
    
    // Assert that it returns an Err because to_bytes() fails for Percentage
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Percentage"));
}

#[test]
fn test_path_escaping_in_commands() {

    let complex_path = PathBuf::from("/mnt/external drive/backup_01");
    let call = Call::Mkdir(complex_path.clone());

    let draft = Draft {
        auto_confirm: false,
        draft_type: "test".to_string(),
        draft: vec![call],
        status: DraftStatus::Pending,
        warnings: vec![],
    };

    let exec = exec_provision(draft).expect("Provisioning should succeed");
    let expected_cmd = format!("mkdir -p {:?}", complex_path);

    assert!(exec.list.iter().any(|inst| inst.shell_string == expected_cmd));
    assert!(exec.list[0].shell_string.contains("\"/mnt/external drive/backup_01\""));

}

#[test]
fn test_empty_draft_yields_empty_exec() {
    let draft = Draft {
        auto_confirm: true,
        draft_type: "empty".to_string(),
        draft: vec![], // No calls
        status: DraftStatus::Pending,
        warnings: vec![],
    };

    let exec = exec_provision(draft).expect("Empty draft should be valid");

    // Ensure no commands are generated, and fstab logic didn't trigger
    assert!(exec.list.is_empty());
    assert!(exec.auto_confirm);
}

#[test]
fn test_confirm_execution_auto_confirm() {
    // Setup an execution plan with auto_confirm set to true
    let mut exec = Exec {
        list: vec![Instruction {
            shell_string: "pvcreate /dev/sdb1".to_string(),
            command_call: std::process::Command::new("true"), // dummy command for test state
        }],
        auto_confirm: true,
        is_allowed: false, // Starts as false
        warnings: vec!["Disk will be wiped".to_string()],
    };

    // Call the function
    let result = confirm_execution(&mut exec);

    // Assertions
    assert!(result.is_ok(), "auto_confirm should return Ok without seeking input");
    assert!(exec.is_allowed, "is_allowed must be toggled to true when auto_confirm is enabled");
}

#[test]
fn test_apply_execution_security_gate() {
    // Create an execution plan that HAS NOT been confirmed
    let exec = Exec {
        list: vec![Instruction { 
            shell_string: "rm -rf /".to_string(), 
            command_call: std::process::Command::new("true"), 
        }], 
        auto_confirm: false,
        is_allowed: false, // The critical flag
        warnings: vec![],
    };

    let result = apply_execution(exec);

    // Assertions
    assert!(result.is_err(), "Apply must fail if is_allowed is false");
    assert!(
        result.unwrap_err().contains("Security Error"),
        "Should return a specific security error message"
    );
}
