use proptest::prelude::*;
use super::*;
use crate::core::Command;
use proptest::sample::Index;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]

    #[test]
    fn test_parse_provision_chaos_resilience(
        junk_args in prop::collection::vec(".*", 0..20)
    ) {
        let mut args = vec!["lvq".to_string(), "provision".to_string()];
        args.extend(junk_args);

        let result = parse_provision(&args);
        
        assert!(result.is_ok() || result.is_err()); 
    }

    #[test]
    fn test_parse_provision_reflexive_roundtrip(
        generated_pvs in prop::collection::vec("[a-zA-Z0-9/]+", 1..5),
        generated_vg in "[a-zA-Z0-9_]+",
        generated_lvs in prop::collection::vec("[a-zA-Z0-9_]+:10G", 1..3) 
    ) {
        let mut args = vec!["lvq".to_string(), "provision".to_string()];
        
        args.push("--pv".to_string());
        args.push(generated_pvs.join(":"));
        
        args.push("--vg".to_string());
        args.push(generated_vg.clone());

        for lv_str in &generated_lvs {
            args.push("--lv".to_string());
            args.push(lv_str.clone());
        }

        let parsed_command = parse_provision(&args).expect("Failed to parse valid generated args");

        let Command::Provision { pvs, vg_name, lvs, .. } = parsed_command;
        
        prop_assert_eq!(pvs.len(), generated_pvs.len());
        prop_assert_eq!(vg_name, generated_vg);
        prop_assert_eq!(lvs.len(), generated_lvs.len());
    }

    #[test]
    fn test_parse_floating_auto_confirm_dynamic(
        generated_pvs in prop::collection::vec("/dev/[a-z]+[0-9]*", 1..3),
        generated_vg in "[a-zA-Z0-9_]+",
        generated_lvs in prop::collection::vec("[a-zA-Z0-9_]+:[1-9][0-9]{0,10}[GM]", 1..20),
        
        insert_picker in any::<Index>()
    ) {
        let mut args = vec!["lvq".to_string(), "provision".to_string()];
        
        args.push("--pv".to_string());
        args.push(generated_pvs.join(":"));
        
        args.push("--vg".to_string());
        args.push(generated_vg.clone());

        for lv in &generated_lvs {
            args.push("--lv".to_string());
            args.push(lv.clone());
        }

        let mut safe_indices = vec![1, 2];
        let mut i = 4;
        while i <= args.len() {
            safe_indices.push(i);
            i += 2;
        }

        let safe_pos = *insert_picker.get(&safe_indices);
        
        args.insert(safe_pos, "-y".to_string());

        let result = super::parse(args).expect("Dynamic command with floating -y should parse");

        prop_assert!(result.auto_confirm, "Failed to detect -y at dynamic position {}", safe_pos);
        
        let Command::Provision { lvs, .. } = result.command;
        prop_assert_eq!(lvs.len(), generated_lvs.len(), "Lost LVs during routing");
    }

}

#[test]
fn test_parse_router_matrix() {
    let test_cases = vec![
        (
            vec!["lvq"],
            "Usage: lvq <command> [options]"
        ),
        (
            vec!["lvq", "destroy", "--lv", "data"],
            "Unknown command: destroy"
        ),
        (
            vec!["lvq", "--pv", "/dev/sda"],
            "Unknown command: "
        ),
        (
            vec!["lvq", "--vg", "provision", "--pv", "/dev/sda"],
            "Unknown command: " 
        ),
    ];

    for (args_str, expected_err) in test_cases {
        let args: Vec<String> = args_str.iter().map(|s| s.to_string()).collect();
        let result = parse(args);
        
        assert!(result.is_err(), "Expected error on input: {:?}", args_str);
        assert_eq!(result.unwrap_err(), expected_err);
    }
}
