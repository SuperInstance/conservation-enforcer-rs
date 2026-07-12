# ⚡ Conservation Enforcer (Rust)

![Crates.io](https://img.shields.io/crates/v/si-conservation-enforcer)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Tests](https://img.shields.io/badge/tests-95%2B-brightgreen)
![License](https://img.shields.io/github/license/SuperInstance/conservation-enforcer-rs)

**FLUX bytecode conservation-law enforcement for LLM outputs — Rust implementation.**

This is the Rust port of the [Python conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer), providing a deterministic, auditable policy layer that wraps any LLM call. It demonstrates that AI behavior can be governed by conservation laws — the same mathematical principles that govern physics.

## Philosophy

Part of [Working Animal Architecture](https://github.com/SuperInstance/AI-Writings), where **γ + η = C** (genome + nurture = capability). Conservation enforcement is the **fence** — the physical boundary that keeps working animals on-task. Just as a fence doesn't judge the animal, FLUX bytecode doesn't judge the model. It just enforces.

```
User Request → LLM Call → [FLUX Conservation Validator] → Response
                                    ↓
                              If violation: return correction
                              If clean: return response
```

The FLUX bytecode acts as a deterministic, auditable policy layer. **You can't lie to bytecode** — it doesn't have opinions, it just executes instructions.

## Why Rust?

- **Zero-cost abstractions** — The entire VM, assembler, and enforcement layer adds negligible overhead
- **No_std compatible** — Works in embedded, WASM, and kernel contexts (use `default-features = false`)
- **No external dependencies** — The entire crate is self-contained
- **Memory safe** — Rust's ownership model guarantees no UB in the policy VM
- **Deterministic** — Same input + bytecode = same output, every time

## Installation

```bash
cargo add si-conservation-enforcer
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
si-conservation-enforcer = "0.1"
```

## Quick Start

```rust
use conservation_enforcer::{ConservationEnforcer, policies::combined_policy};

fn main() {
    let policy = combined_policy(
        500,    // max_tokens
        300,    // max_repetition (30%)
        100,    // min_overlap (10%)
        1500,   // min_entropy
        0,      // min_density (disabled)
        false,  // enable_decay
        0,      // decay_rate
    );

    let mut enforcer = ConservationEnforcer::new(policy, 500);

    let result = enforcer.enforce(
        "What is machine learning?",
        "Machine learning is a subset of AI that learns from data.",
    );

    if result.allowed {
        println!("{}", result.output);
    } else {
        println!("Blocked: {}", result.violation.unwrap().reason);
    }
}
```

## OpenAI Integration

```rust
use conservation_enforcer::{ConservationEnforcer, policies::length_budget_policy};

fn main() {
    let mut enforcer = ConservationEnforcer::new(length_budget_policy(500), 500);

    // Call your LLM here (e.g., via async-openai, reqwest, etc.)
    let llm_response = call_your_llm("Tell me about quantum physics");

    let result = enforcer.enforce("Tell me about quantum physics", &llm_response);

    match result.allowed {
        true => println!("{}", result.output),
        false => println!("🚫 {}", result.output),
    }
}

fn call_your_llm(prompt: &str) -> String {
    // Your LLM call here
    String::from("...")
}
```

## Enforcement in Action

```rust
use conservation_enforcer::{ConservationEnforcer, policies::combined_policy};

fn main() {
    let policy = combined_policy(500, 300, 100, 1500, 300, false, 0);
    let mut enforcer = ConservationEnforcer::new(policy, 500);

    // ✅ Good response
    let r1 = enforcer.enforce("What is AI?", "AI is the simulation of human intelligence in machines.");
    println!("{} {} cycles", if r1.allowed { "✅" } else { "🚫" }, r1.cycles);

    // 🚫 Too repetitive
    let r2 = enforcer.enforce("Summarize", "the the the the the the the the the the");
    println!("{} {}", if r2.allowed { "✅" } else { "🚫" },
        r2.violation.map(|v| v.reason).unwrap_or_default());
}
```

## Conservation Laws

### 1. Length Budget (Information Quantity)
```rust
let policy = policies::length_budget_policy(500);
```
The output cannot exceed the allocated information budget. Analogous to energy conservation — you can't output more information than allocated.

### 2. Repetition Limit (Information Diversity)
```rust
let policy = policies::repetition_policy(300); // max 30% repetition
```
The output must maintain diversity. Degenerate repetition is the informational equivalent of thermal equilibrium.

### 3. Category Confinement (Topical Coherence)
```rust
let policy = policies::category_policy(150); // 15% word overlap required
```
The output must stay within the category/domain of the input.

### 4. Entropy Floor (Information Density)
```rust
let policy = policies::entropy_policy(2000); // 2.0 bits/word minimum
```
The output must have sufficient Shannon entropy.

### 5. Information Density (Token Efficiency)
```rust
let policy = policies::information_density_policy(400); // 40% unique tokens
```
Measures the ratio of unique tokens to total tokens.

### 6. Scope Discipline (Topic Boundary)
```rust
let policy = policies::scope_discipline_policy(120, 10);
```
Checks topical overlap AND limits output expansion to 10× input lengths.

### 7. Budget Decay (Temporal Conservation)
```rust
let policy = policies::budget_decay_policy(50, 10, 100);
```
The enforcement budget itself is a conserved quantity. Each call consumes budget.

### Combined Policy (All Laws)
```rust
let policy = policies::combined_policy(
    500,    // max_tokens
    300,    // max_repetition
    100,    // min_overlap
    1500,   // min_entropy
    300,    // min_density (0 = disabled)
    true,   // enable_decay
    50,     // decay_rate
);
```

## FLUX ISA

| Format | Layout | Example |
|--------|--------|---------|
| A | `[opcode]` | `HALT` |
| B | `[opcode][reg]` | `INC R0` |
| C | `[opcode][rd][rs]` | `CMP R0, R1` |
| D | `[opcode][reg][off_lo][off_hi]` | `JE label` |
| E | `[opcode][rd][rs1][rs2]` | `IADD R0, R1, R2` |

### Syscalls

| # | Name | Returns |
|---|------|---------|
| 1 | GET_INPUT_LEN | Length of input text |
| 2 | GET_OUTPUT_LEN | Length of output text |
| 3 | GET_INPUT_WORDS | Word count of input |
| 4 | GET_OUTPUT_WORDS | Word count of output |
| 5 | GET_TOKEN_COUNT | Approximate token count |
| 6 | GET_REPETITION | Max word frequency ratio × 1000 |
| 7 | GET_CATEGORY | Input/output word overlap × 1000 |
| 8 | SET_VIOLATION | Sets violation flag (R1 = reason code) |
| 10 | GET_BUDGET | Configured information budget |
| 11 | GET_UNIQUE_RATIO | Unique/total words × 1000 |
| 12 | GET_ENTROPY | Shannon entropy × 1000 |
| 13 | GET_CALL_COUNT | Enforcement calls in this session |
| 14 | DECAY_BUDGET | R1 = decay amount, returns new budget |

## Writing Custom Policies

Use the `assemble()` function to compile FLUX assembly:

```rust
use conservation_enforcer::{assemble, ConservationEnforcer};

fn main() {
    let code = assemble(r#"
        ;; Block if unique ratio < 30%
        MOVI R0, 11             ; GET_UNIQUE_RATIO
        SYSCALL
        MOV  R2, R0
        MOVI R3, 300            ; threshold
        JLT  R2, R3, block
        MOVI R0, 0              ; ALLOW
        HALT

    block:
        MOVI R1, 5              ; reason: INFORMATION_DENSITY
        MOVI R0, 8              ; SET_VIOLATION
        SYSCALL
        MOVI R0, 1              ; BLOCK
        HALT
    "#).unwrap();

    let mut enforcer = ConservationEnforcer::new(code, 1000);
    let result = enforcer.enforce("test", "hello world foo bar baz");
    println!("Allowed: {}", result.allowed);
}
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | ✅ | Standard library (alloc, file I/O) |
| `audit` | ❌ | JSON Lines audit logging to files |
| `metrics` | ❌ | Metrics collection and export |

For `no_std` environments:
```toml
[dependencies]
si-conservation-enforcer = { version = "0.1", default-features = false }
```

## Cross-Implementation

This component exists in two languages:
- **Python** (`pip install conservation-enforcer`) — [SuperInstance/conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer)
- **Rust** (`cargo add si-conservation-enforcer`) — [SuperInstance/conservation-enforcer-rs](https://github.com/SuperInstance/conservation-enforcer-rs)

Both implement the same specification. Choose based on your runtime.

### Detailed Comparison

This crate is a line-by-line port of the [Python conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer) v0.2.0. The Python version's test suite (95 tests) has been replicated in Rust. The bytecode produced by the assembler is binary-compatible — you can assemble a policy in Python and execute it in Rust, and vice versa.

| Component | Python | Rust |
|-----------|--------|------|
| FLUX VM | `conservation_enforcer.vm.VM` | `FluxVM` |
| Assembler | `conservation_enforcer.assembler.assemble()` | `conservation_enforcer::assemble()` |
| Enforcer | `ConservationEnforcer` | `ConservationEnforcer` |
| Policies | `policies/` module | `policies` module |
| Audit | `audit.AuditLog` | `audit::AuditLog` (feature-gated) |
| Metrics | `metrics.MetricsCollector` | `metrics::MetricsCollector` (feature-gated) |

## Architecture

```
src/
└── lib.rs          Entire crate (VM, assembler, enforcer, policies, audit, metrics)

tests/
└── integration.rs  Comprehensive test suite (95+ tests)
```

## Ecosystem

### FLUX Runtime
- [conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer) — Python VM (`pip install conservation-enforcer`)
- [conservation-enforcer-rs](https://github.com/SuperInstance/conservation-enforcer-rs) — Rust VM (this crate)
- [flux-core](https://github.com/SuperInstance/flux-core) — Core Rust VM (`cargo add fluxvm`)

### Conservation
- [conservation-law-rs](https://github.com/SuperInstance/conservation-law-rs) — Conservation laws for agent dynamics (Rust)
- [flux-registry](https://github.com/SuperInstance/flux-registry) — Pre-compiled policy registry

## License

MIT

---

*This is not alignment theory. This is enforcement engineering.*
