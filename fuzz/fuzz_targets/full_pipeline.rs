#![no_main]
use libfuzzer_sys::fuzz_target;
use lvq::{parser, planner, verifier, exec, core::DraftStatus};

fuzz_target!(|data: &[u8]| {
    // Convert fuzzer bytes to a string
    let input_str = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Split into args, prepending a dummy binary name "lvq"
    let mut args = vec!["lvq".to_string()];
    args.extend(input_str.split_whitespace().map(|s| s.to_string()));

    // Run the gauntlet
    if let Ok(action) = parser::parse(args) {
        if let Ok(draft) = planner::plan(action) {
            if let Ok(verified_draft) = verifier::verify(draft) {
                if verified_draft.status == DraftStatus::Ready {
                    let _ = exec::provision::exec_provision(verified_draft);
                }
            }
        }
    }
});
