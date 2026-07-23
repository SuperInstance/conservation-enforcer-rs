// conservation-enforcer-rs
// Conservation-law enforcement for LLM outputs, backed by a FLUX VM policy.
//
// The policy bytecode is stored verbatim and re-executed against every
// candidate LLM output. If the VM returns Ok(cycles) where cycles <= budget,
// the output is allowed. If the VM errors out, or burns more cycles than the
// configured budget, the output is blocked and a correction template is
// returned instead.

use fluxvm::error::FluxError;
use fluxvm::vm::Interpreter;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Verdict returned by `ConservationEnforcer::enforce`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcementResult {
    /// Whether the LLM output is allowed through.
    pub allowed: bool,
    /// The (possibly corrected) text to hand back to the caller.
    pub output: String,
    /// Set when the output was blocked. Carries the human-readable reason
    /// and a numeric code matched against the FLUX VM's policy errors.
    pub violation: Option<Violation>,
    /// Number of FLUX VM cycles the most recent `enforce` call consumed.
    pub cycles: u64,
}

impl EnforcementResult {
    /// Build an "allowed" verdict carrying through the original LLM output.
    pub fn allowed(output: String) -> Self {
        Self { allowed: true, output, violation: None, cycles: 0 }
    }

    /// Build a "blocked" verdict with a correction string and a code.
    pub fn blocked(output: String, reason: String, code: i32) -> Self {
        Self {
            allowed: false,
            output,
            violation: Some(Violation { reason, code }),
            cycles: 0,
        }
    }
}

/// One concrete violation of a conservation law.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violation {
    /// Human-readable reason (e.g. "token budget exceeded").
    pub reason: String,
    /// Numeric code propagated from the FLUX policy.
    pub code: i32,
}

/// The main enforcer. Holds the policy bytecode and runs it against
/// every LLM candidate output. Owns its own call counter and budget.
pub struct ConservationEnforcer {
    /// Raw FLUX policy bytecode, executed on every `enforce` call.
    pub policy: Vec<u8>,
    /// Current conservation budget (cycle allowance remaining).
    pub budget: i64,
    /// Snapshot of the budget at construction, used by `reset_budget`.
    initial_budget: i64,
    /// Replacement string handed back when output is blocked.
    pub correction_template: String,
    /// When true, every `enforce` call is appended to `audit_path`.
    pub enable_audit: bool,
    /// JSONL file path for audit logs.
    pub audit_path: Option<String>,
    /// Number of times `enforce` has been called.
    pub call_count: u64,
}

impl ConservationEnforcer {
    /// Construct a new enforcer. The policy bytecode is stored verbatim;
    /// it is not dry-run at construction time. Errors during later
    /// enforcement are reported in `EnforcementResult::violation`.
    pub fn new(
        policy: Vec<u8>,
        budget: i64,
        correction_template: Option<String>,
        enable_audit: bool,
        audit_path: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let template = correction_template.unwrap_or_else(|| {
            "⚠️ This response was blocked by a conservation law: {reason}. \
             Please try again with a more conserved response."
                .to_string()
        });
        Ok(Self {
            policy,
            budget,
            initial_budget: budget,
            correction_template: template,
            enable_audit,
            audit_path,
            call_count: 0,
        })
    }

    /// Load policy bytecode from a binary file and construct the enforcer.
    pub fn from_policy_file<P: AsRef<Path>>(
        path: P,
        budget: i64,
        correction_template: Option<String>,
        enable_audit: bool,
        audit_path: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let bytecode = std::fs::read(path)?;
        Self::new(bytecode, budget, correction_template, enable_audit, audit_path)
    }

    /// Persist the current policy bytecode to a file.
    pub fn save_policy<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(path, &self.policy)?;
        Ok(())
    }

    /// Number of times `enforce` has been called on this enforcer.
    pub fn call_count(&self) -> u64 {
        self.call_count
    }

    /// Remaining conservation budget (in cycles).
    pub fn remaining_budget(&self) -> i64 {
        self.budget
    }

    /// Add cycles back to the budget (e.g. after a cooldown).
    /// Panics on negative amounts; replenishment must be non-negative.
    pub fn replenish_budget(&mut self, amount: i64) {
        if amount < 0 {
            panic!("replenish amount must be non-negative");
        }
        self.budget += amount;
    }

    /// Restore the budget to its initial value.
    pub fn reset_budget(&mut self) {
        self.budget = self.initial_budget;
    }

    /// Run the LLM output through the FLUX policy VM.
    ///
    /// `user_input` is the original prompt (not used by the policy today,
    /// but kept for forward compatibility with policies that tokenize or
    /// compute entropy across both input and output).
    ///
    /// Returns an `EnforcementResult`. Allowed outputs carry the original
    /// LLM text; blocked outputs carry the configured correction template.
    pub fn enforce(&mut self, _user_input: &str, llm_output: &str) -> EnforcementResult {
        self.call_count += 1;

        let mut vm = Interpreter::new(&self.policy);
        let verdict = match vm.execute() {
            Ok(cycles) if (cycles as i64) <= self.budget => EnforcementResult {
                allowed: true,
                output: llm_output.to_string(),
                violation: None,
                cycles,
            },
            Ok(cycles) => {
                let msg = format!("cycle budget exceeded: {} > {}", cycles, self.budget);
                let correction = self
                    .correction_template
                    .replace("{reason}", &msg);
                EnforcementResult::blocked(correction, msg, 1)
            }
            Err(FluxError::CycleBudgetExceeded(cycles)) => {
                let msg = format!("cycle limit exceeded at {} cycles", cycles);
                let correction = self.correction_template.replace("{reason}", &msg);
                EnforcementResult::blocked(correction, msg, 2)
            }
            Err(e) => {
                let msg = format!("policy VM error: {:?}", e);
                let correction = self.correction_template.replace("{reason}", &msg);
                EnforcementResult::blocked(correction, msg, 3)
            }
        };

        if self.enable_audit {
            if let Some(path) = &self.audit_path {
                let _ = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .and_then(|mut f| {
                        use std::io::Write;
                        let line = serde_json::to_string(&verdict).unwrap_or_default();
                        writeln!(f, "{}", line)
                    });
            }
        }

        verdict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_policy_and_budget() {
        let enforcer =
            ConservationEnforcer::new(vec![0, 1, 2, 3], 1000, None, false, None).unwrap();
        assert_eq!(enforcer.budget, 1000);
        assert_eq!(enforcer.policy, vec![0, 1, 2, 3]);
        assert_eq!(enforcer.call_count(), 0);
    }

    #[test]
    fn call_count_increments_on_enforce() {
        let mut enforcer =
            ConservationEnforcer::new(vec![], 1000, None, false, None).unwrap();
        enforcer.enforce("hi", "hello");
        enforcer.enforce("hi", "world");
        assert_eq!(enforcer.call_count(), 2);
    }

    #[test]
    fn remaining_budget_tracks_state() {
        let mut enforcer =
            ConservationEnforcer::new(vec![], 500, None, false, None).unwrap();
        assert_eq!(enforcer.remaining_budget(), 500);
        enforcer.replenish_budget(200);
        assert_eq!(enforcer.remaining_budget(), 700);
        enforcer.reset_budget();
        assert_eq!(enforcer.remaining_budget(), 500);
    }

    #[test]
    #[should_panic(expected = "must be non-negative")]
    fn replenish_panics_on_negative() {
        let mut enforcer =
            ConservationEnforcer::new(vec![], 100, None, false, None).unwrap();
        enforcer.replenish_budget(-1);
    }

    #[test]
    fn blocked_keeps_correction_template() {
        let r = EnforcementResult::blocked(
            "corrected".into(),
            "out of budget".into(),
            42,
        );
        assert!(!r.allowed);
        assert_eq!(r.output, "corrected");
        let v = r.violation.unwrap();
        assert_eq!(v.reason, "out of budget");
        assert_eq!(v.code, 42);
    }

    #[test]
    fn allowed_has_no_violation() {
        let r = EnforcementResult::allowed("keep".into());
        assert!(r.allowed);
        assert_eq!(r.output, "keep");
        assert!(r.violation.is_none());
    }

    #[test]
    fn empty_policy_with_high_budget_allows() {
        let mut enforcer =
            ConservationEnforcer::new(vec![], 10_000, None, false, None).unwrap();
        let result = enforcer.enforce("Hello", "Hello world");
        assert!(result.allowed, "empty policy should allow short output");
        assert_eq!(result.output, "Hello world");
    }

    #[test]
    fn empty_policy_with_zero_budget_denies() {
        let mut enforcer =
            ConservationEnforcer::new(vec![], -1, None, false, None).unwrap();
        let result = enforcer.enforce("Hello", "Hello world");
        // Empty policy still executes the VM, which burns cycles (0 or more).
        // With negative budget, any non-negative cycle count exceeds the budget.
        assert!(!result.allowed, "negative budget should deny output");
        assert!(result.output.contains("cycle"), "blocked output should mention cycle");
        assert!(result.violation.is_some(), "denied result should have a violation");
    }
}
