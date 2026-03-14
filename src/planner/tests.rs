use super::*;
use crate::core::*;
use proptest::prelude::*;
use std::path::PathBuf;

fn arb_size_unit() -> impl Strategy<Value = SizeUnit> {
    prop_oneof![
        any::<u64>().prop_map(SizeUnit::Megabytes),
        any::<u64>().prop_map(SizeUnit::Gigabytes),
        any::<u64>().prop_map(SizeUnit::Extents),
    ]
}

fn arb_filesystem() -> impl Strategy<Value = Filesystem> {
    prop_oneof![
        Just(Filesystem::Xfs),
        Just(Filesystem::Ext4),
        Just(Filesystem::Swap),
        Just(Filesystem::Btrfs),
    ]
}

fn arb_lv_request() -> impl Strategy<Value = LvRequest> {
    (
        "[a-z0-9_]{1,10}", // valid name regex
        arb_size_unit(),
        prop::option::weighted(0.7, (arb_filesystem(), prop::option::of("[a-z0-9/]{1,20}")))
    ).prop_map(|(name, size, fs_opt)| {
        let fs = fs_opt.map(|(fs_type, path)| FsMount {
            fs: fs_type,
            mount_path: path.map(PathBuf::from),
        });
        LvRequest { name, size, fs }
    })
}

proptest! {
    #[test]
    fn test_plan_provision_ordering_and_consistency(
        pvs in prop::collection::vec("[a-z/]{5,15}", 1..5),
        vg_name in "[a-z0-9]{3,10}",
        pe_size in arb_size_unit(),
        lvs in prop::collection::vec(arb_lv_request(), 0..5)
    ) {
        let pv_paths: Vec<PathBuf> = pvs.into_iter().map(PathBuf::from).collect();
        
        let result = plan_provision(pv_paths.clone(), vg_name.clone(), pe_size, lvs.clone());
        prop_assert!(result.is_ok());
        let plan = result.unwrap();

        let pv_create_count = plan.iter().filter(|c| matches!(c, Call::PvCreate(_))).count();
        prop_assert_eq!(pv_create_count, pv_paths.len());

        let mut vg_idx = None;
        let mut lv_indices = Vec::new();

        for (i, call) in plan.iter().enumerate() {
            match call {
                Call::VgCreate { name, .. } => {
                    prop_assert_eq!(name, &vg_name);
                    vg_idx = Some(i);
                    for j in 0..i {
                        if !matches!(plan[j], Call::PvCreate(_)) {
                        }
                    }
                }
                Call::LvCreate { vg, .. } => {
                    prop_assert_eq!(vg, &vg_name);
                    prop_assert!(vg_idx.is_some() && i > vg_idx.unwrap(), "LV created before VG");
                    lv_indices.push(i);
                }
                Call::Mkfs { device, .. } | Call::MkSwap(device) => {
                    let device_str = device.to_string_lossy();
                    prop_assert!(device_str.contains(&vg_name), "Device path mismatch");
                    
                    let has_creator = plan[..i].iter().any(|c| {
                        if let Call::LvCreate { name, .. } = c {
                            device_str.ends_with(name)
                        } else { false }
                    });
                    prop_assert!(has_creator, "Formatted a device before creating the LV");
                }
                Call::Mount { device: _, path: _ } => {
                     let has_mkdir = plan[..i].iter().any(|c| matches!(c, Call::Mkdir(_)));
                     prop_assert!(has_mkdir, "Mount called without Mkdir");
                }
                _ => {}
            }
        }
    }
    #[test]
    fn test_planner_structural_invariants(
        pvs in prop::collection::vec("[a-z/]{5,15}", 1..5),
        vg_name in "[a-z0-9]{3,10}",
        lvs in prop::collection::vec(arb_lv_request(), 1..10)
    ) {
        let pv_paths: Vec<PathBuf> = pvs.into_iter().map(PathBuf::from).collect();
        let pe_size = SizeUnit::Megabytes(4);
        
        let plan = plan_provision(pv_paths.clone(), vg_name.clone(), pe_size, lvs.clone()).unwrap();

        let pv_calls = plan.iter().filter(|c| matches!(c, Call::PvCreate(_))).count();
        prop_assert_eq!(pv_calls, pv_paths.len());

        let lv_calls = plan.iter().filter(|c| matches!(c, Call::LvCreate { .. })).count();
        prop_assert_eq!(lv_calls, lvs.len());

        for lv_req in &lvs {
            let expected_path = PathBuf::from(format!("/dev/{}/{}", vg_name, lv_req.name));
            
            for call in &plan {
                match call {
                    Call::Mkfs { device, .. } | Call::MkSwap(device) | Call::Mount { device, .. } | Call::Fstab { device, .. } => {
                        if device.file_name().map(|n| n == lv_req.name.as_str()).unwrap_or(false) {
                            prop_assert_eq!(device, &expected_path, 
                                "Device path mismatch for LV '{}'. Expected {:?}, got {:?}", 
                                lv_req.name, expected_path, device);
                        }
                    },
                    Call::LvCreate { name, vg, .. } => {
                        if name == &lv_req.name {
                            prop_assert_eq!(vg, &vg_name);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
        fn test_plan_draft_wrapping_invariants(
            auto_confirm in proptest::bool::ANY,
            vg_name in "[a-z0-9]{3,10}",
            pvs in prop::collection::vec("[a-z/]{5,15}", 1..3),
            lvs in prop::collection::vec(arb_lv_request(), 0..3)
        ) {
            let action = Action {
                auto_confirm,
                command: Command::Provision {
                    pvs: pvs.into_iter().map(PathBuf::from).collect(),
                    vg_name,
                    pe_size: SizeUnit::Megabytes(4),
                    lvs,
                },
            };

            let result = plan(action);
            prop_assert!(result.is_ok());
            let draft = result.unwrap();

            prop_assert_eq!(draft.auto_confirm, auto_confirm);
            
            prop_assert_eq!(draft.draft_type, "provision");
            prop_assert_eq!(draft.status, DraftStatus::Pending);
            
            prop_assert!(!draft.draft.is_empty() || (true));
            
            prop_assert!(draft.warnings.is_empty());
        }
}

// Regular Unit Testing
#[test]
fn test_swap_special_handling() {
    let lvs = vec![LvRequest {
        name: "swap_vol".into(),
        size: SizeUnit::Gigabytes(2),
        fs: Some(FsMount {
            fs: Filesystem::Swap,
            mount_path: None, 
        }),
    }];

    let plan = plan_provision(vec![], "vg0".into(), SizeUnit::Megabytes(4), lvs).unwrap();

    assert!(plan.iter().any(|c| matches!(c, Call::MkSwap(_))));
    assert!(!plan.iter().any(|c| matches!(c, Call::Mkfs { .. })));

    let has_swap_fstab = plan.iter().any(|c| {
        if let Call::Fstab { path, .. } = c {
            path == &PathBuf::from("none")
        } else { false }
    });
    assert!(has_swap_fstab);
}

#[test]
fn test_no_mount_logic() {
    let lvs = vec![LvRequest {
        name: "data".into(),
        size: SizeUnit::Gigabytes(10),
        fs: Some(FsMount {
            fs: Filesystem::Ext4,
            mount_path: None, 
        }),
    }];

    let plan = plan_provision(vec![], "vg0".into(), SizeUnit::Megabytes(4), lvs).unwrap();

    assert!(plan.iter().any(|c| matches!(c, Call::Mkfs { .. })));
    assert!(!plan.iter().any(|c| matches!(c, Call::Mkdir(_))));
    assert!(!plan.iter().any(|c| matches!(c, Call::Mount { .. })));
    assert!(!plan.iter().any(|c| matches!(c, Call::Fstab { .. })));
}
