//! Integration tests for conservation-enforcer (Rust)
//!
//! These mirror the Python test suite, ensuring behavioral parity.

use conservation_enforcer::{assemble, policies, ConservationEnforcer, FluxVM, Op, VmError};

// ═══════════════════════════════════════════════════════════════════════════════
// VM Arithmetic
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn vm_add() {
    let code = vec![
        Op::Movi as u8,
        0,
        10,
        0,
        Op::Movi as u8,
        1,
        20,
        0,
        Op::Iadd as u8,
        2,
        0,
        1,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 30);
}

#[test]
fn vm_sub() {
    let code = vec![
        Op::Movi as u8,
        0,
        50,
        0,
        Op::Movi as u8,
        1,
        20,
        0,
        Op::Isub as u8,
        2,
        0,
        1,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 30);
}

#[test]
fn vm_mul() {
    let code = vec![
        Op::Movi as u8,
        0,
        7,
        0,
        Op::Movi as u8,
        1,
        6,
        0,
        Op::Imul as u8,
        2,
        0,
        1,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 42);
}

#[test]
fn vm_div() {
    let code = vec![
        Op::Movi as u8,
        0,
        100,
        0,
        Op::Movi as u8,
        1,
        5,
        0,
        Op::Idiv as u8,
        2,
        0,
        1,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 20);
}

#[test]
fn vm_div_by_zero() {
    let code = vec![
        Op::Movi as u8,
        0,
        10,
        0,
        Op::Movi as u8,
        1,
        0,
        0,
        Op::Idiv as u8,
        2,
        0,
        1,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    assert_eq!(vm.run(&code), Err(VmError::DivisionByZero));
}

#[test]
fn vm_mod() {
    let code = vec![
        Op::Movi as u8,
        0,
        17,
        0,
        Op::Movi as u8,
        1,
        5,
        0,
        Op::Imod as u8,
        2,
        0,
        1,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 2);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Control Flow
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn vm_je_taken() {
    let code = vec![
        Op::Movi as u8,
        0,
        5,
        0,
        Op::Movi as u8,
        1,
        5,
        0,
        Op::Cmp as u8,
        0,
        1,
        Op::Je as u8,
        0,
        4,
        0,
        Op::Movi as u8,
        0,
        99,
        0,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 5);
}

#[test]
fn vm_jne_taken() {
    let code = vec![
        Op::Movi as u8,
        0,
        5,
        0,
        Op::Movi as u8,
        1,
        3,
        0,
        Op::Cmp as u8,
        0,
        1,
        Op::Jne as u8,
        0,
        4,
        0,
        Op::Movi as u8,
        0,
        99,
        0,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 5);
}

#[test]
fn vm_jsge_greater() {
    let code = vec![
        Op::Movi as u8,
        0,
        10,
        0,
        Op::Movi as u8,
        1,
        5,
        0,
        Op::Cmp as u8,
        0,
        1,
        Op::Jsge as u8,
        0,
        4,
        0,
        Op::Movi as u8,
        0,
        99,
        0,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 10);
}

#[test]
fn vm_jsge_less_no_jump() {
    let code = vec![
        Op::Movi as u8,
        0,
        3,
        0,
        Op::Movi as u8,
        1,
        5,
        0,
        Op::Cmp as u8,
        0,
        1,
        Op::Jsge as u8,
        0,
        4,
        0,
        Op::Movi as u8,
        0,
        99,
        0,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 99);
}

#[test]
fn vm_jslt_less() {
    let code = vec![
        Op::Movi as u8,
        0,
        3,
        0,
        Op::Movi as u8,
        1,
        8,
        0,
        Op::Cmp as u8,
        0,
        1,
        Op::Jslt as u8,
        0,
        4,
        0,
        Op::Movi as u8,
        0,
        99,
        0,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 3);
}

#[test]
fn vm_jslt_greater_no_jump() {
    let code = vec![
        Op::Movi as u8,
        0,
        10,
        0,
        Op::Movi as u8,
        1,
        3,
        0,
        Op::Cmp as u8,
        0,
        1,
        Op::Jslt as u8,
        0,
        4,
        0,
        Op::Movi as u8,
        0,
        99,
        0,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 99);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Syscalls
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn syscall_input_len() {
    let code = vec![Op::Movi as u8, 0, 1, 0, Op::Syscall as u8, Op::Halt as u8];
    let mut vm = FluxVM::new();
    vm.load_input("hello world");
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 11);
}

#[test]
fn syscall_output_len() {
    let code = vec![Op::Movi as u8, 0, 2, 0, Op::Syscall as u8, Op::Halt as u8];
    let mut vm = FluxVM::new();
    vm.load_output("test response");
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 13);
}

#[test]
fn syscall_token_count() {
    let code = vec![Op::Movi as u8, 0, 5, 0, Op::Syscall as u8, Op::Halt as u8];
    let mut vm = FluxVM::new();
    vm.load_output(&"a".repeat(40));
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 10);
}

#[test]
fn syscall_repetition() {
    let code = vec![Op::Movi as u8, 0, 6, 0, Op::Syscall as u8, Op::Halt as u8];
    let mut vm = FluxVM::new();
    vm.load_output("the the the the the");
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 1000);
}

#[test]
fn syscall_get_budget() {
    let code = vec![Op::Movi as u8, 0, 10, 0, Op::Syscall as u8, Op::Halt as u8];
    let mut vm = FluxVM::new();
    vm.set_budget(750);
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 750);
}

#[test]
fn syscall_unique_ratio() {
    let code = vec![Op::Movi as u8, 0, 11, 0, Op::Syscall as u8, Op::Halt as u8];
    let mut vm = FluxVM::new();
    vm.load_output("apple banana apple banana cherry");
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 600);
}

// The per-mille ratios are computed as `count * 1000 / total`. With u32
// arithmetic, `count * 1000` overflows once a word appears more than
// ~4.29M times (panic in debug, wrap in release). The math must use a
// wider intermediate so large inputs cannot trigger undefined behaviour
// or silently wrong results.
#[test]
fn syscall_ratios_no_overflow_on_large_input() {
    // 4.500.000 identical tokens: max_count * 1000 = 4.5e9 > u32::MAX.
    let big = "x ".repeat(4_500_000);
    let mut vm = FluxVM::new();
    vm.load_output(&big);

    // GET_REPETITION (syscall 6): all-same input -> 1000 per-mille.
    let code = vec![Op::Movi as u8, 0, 6, 0, Op::Syscall as u8, Op::Halt as u8];
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 1000);

    // GET_UNIQUE_RATIO (syscall 11): 1 unique / 4.5M total -> 0 per-mille.
    let code = vec![Op::Movi as u8, 0, 11, 0, Op::Syscall as u8, Op::Halt as u8];
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 0);
}

#[test]
fn syscall_violation_flag() {
    let code = vec![
        Op::Movi as u8,
        1,
        2,
        0,
        Op::Movi as u8,
        0,
        8,
        0,
        Op::Syscall as u8,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert!(vm.violated());
    assert!(vm
        .violation_reason_str()
        .to_lowercase()
        .contains("repetition"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Stack
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn stack_push_pop() {
    let code = vec![
        Op::Movi as u8,
        0,
        42,
        0,
        Op::Push as u8,
        0,
        Op::Movi as u8,
        0,
        0,
        0,
        Op::Pop as u8,
        1,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(1), 42);
}

#[test]
fn stack_inc_dec() {
    let code = vec![
        Op::Movi as u8,
        0,
        5,
        0,
        Op::Inc as u8,
        0,
        Op::Inc as u8,
        0,
        Op::Dec as u8,
        0,
        Op::Halt as u8,
    ];
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 6);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Length Budget Policy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn length_allows_short_output() {
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(100), 100);
    let result = e.enforce("What is Python?", "Python is a programming language.");
    assert!(result.allowed);
}

#[test]
fn length_blocks_long_output() {
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(5), 5);
    let result = e.enforce("Tell me everything", &"word ".repeat(100));
    assert!(!result.allowed);
    assert!(result.violation.unwrap().reason.contains("Length budget"));
}

#[test]
fn length_correction_message() {
    let mut e = ConservationEnforcer::with_options(
        policies::length_budget_policy(3),
        3,
        Some("🚫 Blocked: {reason}"),
    );
    let result = e.enforce("Q", "This is a very long response that exceeds budget");
    assert!(!result.allowed);
    assert!(result.output.contains("🚫 Blocked:"));
}

// The threshold must be the `max_tokens` parameter, NOT the enforcer's
// conservation budget. Earlier the policy compared the token count against
// `GET_BUDGET`, so passing a small `max_tokens` with a large budget let
// oversized outputs through. These two cases pin the threshold to
// `max_tokens` regardless of the budget.

#[test]
fn length_threshold_is_max_tokens_not_budget() {
    // max_tokens = 3 (tiny), budget = 1_000_000 (huge).
    // Output "word " * 100 -> 500 chars -> ~125 tokens, well above 3.
    // Must be blocked even though the budget is enormous.
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(3), 1_000_000);
    let result = e.enforce("Tell me everything", &"word ".repeat(100));
    assert!(!result.allowed);
    assert!(result.violation.unwrap().reason.contains("Length budget"));
}

#[test]
fn length_allows_under_max_tokens_even_when_budget_low() {
    // max_tokens = 1000 (large), budget = 3 (tiny).
    // Output of 24 chars -> ~6 tokens: above the budget of 3 but well under
    // max_tokens of 1000. Must be allowed: the budget is not the length gate.
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(1000), 3);
    let result = e.enforce("Q", "a short response here");
    assert!(result.allowed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Repetition Policy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn repetition_allows_diverse() {
    let mut e = ConservationEnforcer::new(policies::repetition_policy(500), 1000);
    let result = e.enforce(
        "Explain photosynthesis",
        "Plants convert sunlight into chemical energy through photosynthesis using chlorophyll and water.",
    );
    assert!(result.allowed);
}

#[test]
fn repetition_blocks_repetitive() {
    let mut e = ConservationEnforcer::new(policies::repetition_policy(300), 1000);
    let result = e.enforce("Summarize", "the the the the the the the the the the");
    assert!(!result.allowed);
    assert!(result
        .violation
        .unwrap()
        .reason
        .to_lowercase()
        .contains("repetition"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Category Policy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn category_allows_on_topic() {
    let mut e = ConservationEnforcer::new(policies::category_policy(50), 1000);
    let result = e.enforce(
        "Python programming language",
        "Python is a great programming language for beginners and experts alike",
    );
    assert!(result.allowed);
}

#[test]
fn category_blocks_off_topic() {
    let mut e = ConservationEnforcer::new(policies::category_policy(900), 1000);
    let result = e.enforce(
        "quantum physics particles",
        "banana apple orange grape melon",
    );
    assert!(!result.allowed);
    assert!(result
        .violation
        .unwrap()
        .reason
        .to_lowercase()
        .contains("category"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Entropy Policy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn entropy_allows_high() {
    let mut e = ConservationEnforcer::new(policies::entropy_policy(1000), 1000);
    let result = e.enforce(
        "List colors",
        "red blue green yellow orange purple cyan magenta",
    );
    assert!(result.allowed);
}

#[test]
fn entropy_blocks_low() {
    let mut e = ConservationEnforcer::new(policies::entropy_policy(2500), 1000);
    let result = e.enforce("Write a poem", "go go go go go go go go go go");
    assert!(!result.allowed);
    assert!(result
        .violation
        .unwrap()
        .reason
        .to_lowercase()
        .contains("entropy"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combined Policy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn combined_allows_compliant() {
    let policy = policies::combined_policy(500, 500, 10, 500, 0, false, 0);
    let mut e = ConservationEnforcer::new(policy, 500);
    let result = e.enforce(
        "What is machine learning?",
        "Machine learning is a subset of artificial intelligence that enables systems to learn from data.",
    );
    assert!(result.allowed);
}

#[test]
fn combined_blocks_length() {
    let policy = policies::combined_policy(3, 500, 0, 0, 0, false, 0);
    let mut e = ConservationEnforcer::new(policy, 3);
    let result = e.enforce(
        "Write a long essay about AI",
        &"Artificial intelligence is ".repeat(50),
    );
    assert!(!result.allowed);
    assert!(result.violation.unwrap().reason.contains("Length"));
}

#[test]
fn combined_blocks_repetition() {
    let policy = policies::combined_policy(10000, 200, 0, 0, 0, false, 0);
    let mut e = ConservationEnforcer::new(policy, 10000);
    let result = e.enforce(
        "Describe a sunset",
        "beautiful beautiful beautiful beautiful beautiful beautiful beautiful",
    );
    assert!(!result.allowed);
    assert!(result
        .violation
        .unwrap()
        .reason
        .to_lowercase()
        .contains("repetition"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Information Density Policy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn density_allows_high() {
    let mut e = ConservationEnforcer::new(policies::information_density_policy(300), 1000);
    let result = e.enforce(
        "List colors",
        "red blue green yellow orange purple cyan magenta violet turquoise",
    );
    assert!(result.allowed);
}

#[test]
fn density_blocks_low() {
    let mut e = ConservationEnforcer::new(policies::information_density_policy(500), 1000);
    let result = e.enforce("Write a poem", "go go go go go go go go go go");
    assert!(!result.allowed);
    assert!(result
        .violation
        .unwrap()
        .reason
        .to_lowercase()
        .contains("density"));
}

#[test]
fn density_boundary() {
    let mut e = ConservationEnforcer::new(policies::information_density_policy(500), 1000);
    // "go go" -> 1 unique out of 2 total = 500 per-mille, exactly at the
    // threshold. JLT is strict (<), so equal-to-threshold is allowed.
    let at = e.enforce("test", "go go");
    assert!(at.allowed);
    // "go go go" -> 1 unique out of 3 total = 333 per-mille, just below the
    // threshold, so it must be blocked.
    let below = e.enforce("test", "go go go");
    assert!(!below.allowed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Scope Discipline Policy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn scope_allows_on_topic() {
    let mut e = ConservationEnforcer::new(policies::scope_discipline_policy(50, 10), 1000);
    let result = e.enforce(
        "Python programming language tutorial",
        "Python is a great programming language for beginners",
    );
    assert!(result.allowed);
}

#[test]
fn scope_blocks_off_topic() {
    let mut e = ConservationEnforcer::new(policies::scope_discipline_policy(500, 10), 1000);
    let result = e.enforce(
        "quantum physics particles energy",
        "banana apple orange grape melon fruit",
    );
    assert!(!result.allowed);
    assert!(result
        .violation
        .unwrap()
        .reason
        .to_lowercase()
        .contains("scope"));
}

#[test]
fn scope_blocks_excessive_expansion() {
    let mut e = ConservationEnforcer::new(policies::scope_discipline_policy(0, 10), 1000);
    let result = e.enforce("hi", &"hello ".repeat(100));
    assert!(!result.allowed);
    assert!(result
        .violation
        .unwrap()
        .reason
        .to_lowercase()
        .contains("scope"));
}

#[test]
fn scope_empty_input() {
    let mut e = ConservationEnforcer::new(policies::scope_discipline_policy(0, 10), 1000);
    let result = e.enforce("", "any output here");
    assert!(result.allowed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Budget Decay Policy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn decay_allows_with_budget() {
    let mut e = ConservationEnforcer::new(policies::budget_decay_policy(10, 5, 100), 1000);
    let result = e.enforce("question", "answer response");
    assert!(result.allowed);
}

#[test]
fn decay_blocks_when_exhausted() {
    let mut e = ConservationEnforcer::new(policies::budget_decay_policy(100, 50, 100), 100);
    let result = e.enforce("question", "answer response");
    assert!(!result.allowed);
    let reason = result.violation.unwrap().reason.to_lowercase();
    assert!(reason.contains("budget") || reason.contains("cooldown"));
}

#[test]
fn decay_decreases_across_calls() {
    let mut e = ConservationEnforcer::new(policies::budget_decay_policy(50, 5, 100), 500);
    assert_eq!(e.remaining_budget(), 500);
    e.enforce("q1", "response one here");
    assert_eq!(e.remaining_budget(), 450);
    e.enforce("q2", "response two here");
    assert_eq!(e.remaining_budget(), 400);
}

#[test]
fn decay_blocks_max_calls() {
    let mut e = ConservationEnforcer::new(policies::budget_decay_policy(1, 0, 3), 10000);
    e.enforce("q", "a response");
    e.enforce("q", "a response");
    e.enforce("q", "a response");
    let r4 = e.enforce("q", "a response");
    assert!(!r4.allowed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Budget Sync After Decay
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn budget_syncs_after_decay() {
    let mut e = ConservationEnforcer::new(policies::budget_decay_policy(50, 5, 100), 500);
    e.enforce("q", "a response here");
    assert_eq!(e.remaining_budget(), 450);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Enforcement Result
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn cycles_recorded() {
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(10000), 10000);
    let result = e.enforce("Hi", "Hello!");
    assert!(result.cycles > 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Custom Policies
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn custom_always_allow() {
    let code = assemble("MOVI R0, 0\nHALT").unwrap();
    let mut e = ConservationEnforcer::new(code, 1000);
    let result = e.enforce("anything", "any response");
    assert!(result.allowed);
}

#[test]
fn custom_always_block() {
    let code = assemble("MOVI R1, 99\nMOVI R0, 8\nSYSCALL\nMOVI R0, 1\nHALT").unwrap();
    let mut e = ConservationEnforcer::new(code, 1000);
    let result = e.enforce("q", "a");
    assert!(!result.allowed);
    assert!(result.violation.unwrap().reason.contains("Custom"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Budget Tracking
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn remaining_budget_reflects_state() {
    let e = ConservationEnforcer::new(policies::length_budget_policy(10000), 500);
    assert_eq!(e.remaining_budget(), 500);
}

#[test]
fn replenish_adds_to_budget() {
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(10000), 100);
    e.replenish_budget(50);
    assert_eq!(e.remaining_budget(), 150);
}

#[test]
fn reset_restores_initial_budget() {
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(10000), 200);
    e.replenish_budget(100);
    assert_eq!(e.remaining_budget(), 300);
    e.reset_budget();
    assert_eq!(e.remaining_budget(), 200);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Call Count
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn call_count_increments() {
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(10000), 10000);
    assert_eq!(e.call_count(), 0);
    e.enforce("q1", "response one");
    assert_eq!(e.call_count(), 1);
    e.enforce("q2", "response two");
    assert_eq!(e.call_count(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Enforce with LLM
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn enforce_with_llm_wraps_call() {
    let mut e = ConservationEnforcer::new(policies::length_budget_policy(100), 100);
    let result = e.enforce_with_llm("Hello", |p| format!("Response to: {p}"));
    assert!(result.allowed);
    assert!(result.output.contains("Response to: Hello"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Assembler — Labels and Pseudo-Jumps
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn asm_je_label() {
    let code = assemble("MOVI R0, 0\nCMP R0, R0\nJE done\nMOVI R0, 99\ndone:\nHALT").unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 0);
}

#[test]
fn asm_jmp_label() {
    let code = assemble("MOVI R0, 1\nJMP skip\nMOVI R0, 99\nskip:\nMOVI R0, 42\nHALT").unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 42);
}

#[test]
fn asm_jge_taken_greater() {
    let code = assemble(
        "MOVI R0, 10\nMOVI R1, 5\nJGE R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
    )
    .unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 1);
}

#[test]
fn asm_jge_taken_equal() {
    let code = assemble(
        "MOVI R0, 5\nMOVI R1, 5\nJGE R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
    )
    .unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 1);
}

#[test]
fn asm_jge_not_taken() {
    let code = assemble(
        "MOVI R0, 3\nMOVI R1, 5\nJGE R0, R1, hit\nMOVI R2, 99\nHALT\nhit:\nMOVI R2, 1\nHALT",
    )
    .unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 99);
}

#[test]
fn asm_jlt_taken() {
    let code = assemble(
        "MOVI R0, 3\nMOVI R1, 5\nJLT R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
    )
    .unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 1);
}

#[test]
fn asm_jgt_taken() {
    let code = assemble(
        "MOVI R0, 10\nMOVI R1, 5\nJGT R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
    )
    .unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 1);
}

#[test]
fn asm_jgt_not_taken_equal() {
    let code = assemble(
        "MOVI R0, 5\nMOVI R1, 5\nJGT R0, R1, hit\nMOVI R2, 99\nHALT\nhit:\nMOVI R2, 1\nHALT",
    )
    .unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 99);
}

#[test]
fn asm_jle_taken_equal() {
    let code = assemble(
        "MOVI R0, 5\nMOVI R1, 5\nJLE R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
    )
    .unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 1);
}

#[test]
fn asm_jle_taken_less() {
    let code = assemble(
        "MOVI R0, 3\nMOVI R1, 5\nJLE R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
    )
    .unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(2), 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Assembler — Edge Cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn asm_simple_halt() {
    assert_eq!(assemble("HALT").unwrap(), vec![Op::Halt as u8]);
}

#[test]
fn asm_nop() {
    assert_eq!(assemble("NOP").unwrap(), vec![Op::Nop as u8]);
}

#[test]
fn asm_movi() {
    assert_eq!(
        assemble("MOVI R0, 42").unwrap(),
        vec![Op::Movi as u8, 0, 42, 0]
    );
}

#[test]
fn asm_add() {
    assert_eq!(
        assemble("IADD R2, R0, R1").unwrap(),
        vec![Op::Iadd as u8, 2, 0, 1]
    );
}

#[test]
fn asm_mov() {
    assert_eq!(assemble("MOV R0, R1").unwrap(), vec![Op::Mov as u8, 0, 1]);
}

#[test]
fn asm_inc() {
    assert_eq!(assemble("INC R5").unwrap(), vec![Op::Inc as u8, 5]);
}

#[test]
fn asm_cmp() {
    assert_eq!(assemble("CMP R0, R1").unwrap(), vec![Op::Cmp as u8, 0, 1]);
}

#[test]
fn asm_multiple_instructions() {
    let code = assemble("MOVI R0, 10\nMOVI R1, 20\nIADD R2, R0, R1\nHALT").unwrap();
    let expected = vec![
        Op::Movi as u8,
        0,
        10,
        0,
        Op::Movi as u8,
        1,
        20,
        0,
        Op::Iadd as u8,
        2,
        0,
        1,
        Op::Halt as u8,
    ];
    assert_eq!(code, expected);
}

#[test]
fn asm_comments() {
    let code = assemble("; comment\nMOVI R0, 5  # inline\nHALT").unwrap();
    let mut vm = FluxVM::new();
    vm.run(&code).unwrap();
    assert_eq!(vm.regs.get(0), 5);
}

#[test]
fn asm_unknown_instruction() {
    assert!(assemble("BOGUS R0, R1").is_err());
}

#[test]
fn asm_undefined_label() {
    assert!(assemble("JE nowhere\nHALT").is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combined with New Policies
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn combined_with_density() {
    let policy = policies::combined_policy(10000, 500, 0, 0, 300, false, 0);
    let mut e = ConservationEnforcer::new(policy, 10000);
    let result = e.enforce("Write something", "blah blah blah blah blah blah blah");
    assert!(!result.allowed);
}

#[test]
fn combined_with_decay() {
    let policy = policies::combined_policy(10000, 500, 0, 0, 0, true, 100);
    let mut e = ConservationEnforcer::new(policy, 200);
    e.enforce("q", "a reasonable response here");
    let r2 = e.enforce("q", "a reasonable response here");
    assert!(!r2.allowed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Audit (feature-gated)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "audit")]
#[test]
fn audit_log_read_all_returns_logged_lines() {
    use conservation_enforcer::audit::AuditLog;

    let path = std::env::temp_dir().join(format!(
        "conservation_enforcer_audit_{}.jsonl",
        std::process::id()
    ));
    // Start from a clean file.
    let log = AuditLog::new(path.to_str().unwrap());
    log.clear();
    assert!(log.read_all().is_empty());

    log.log("question", "answer", true, None, 0, 12, 500, 1);
    log.log(
        "question",
        "answer",
        false,
        Some("Length budget exceeded"),
        1,
        7,
        500,
        2,
    );

    let records = log.read_all();
    assert_eq!(
        records.len(),
        2,
        "read_all must return the two logged records"
    );
    assert!(records[0].contains(r#""allowed":true"#));
    assert!(records[1].contains(r#""allowed":false"#));
    assert!(records[1].contains(r#""violation_code":1"#));

    // summary() must agree with the raw records.
    let s = log.summary();
    assert_eq!(s.total_calls, 2);
    assert_eq!(s.total_blocked, 1);

    log.clear();
    assert!(log.read_all().is_empty());
    let _ = std::fs::remove_file(&path);
}
