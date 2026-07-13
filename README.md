# ⚡ Conservation Enforcer (Rust)

![Crates.io](https://img.shields.io/crates/v/si-conservation-enforcer)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Tests](https://img.shields.io/badge/tests-150%2B-brightgreen)
![License](https://img.shields.io/github/license/SuperInstance/conservation-enforcer-rs)

**FLUX bytecode conservation-law enforcement for LLM outputs.**

A deterministic, auditable policy layer that wraps any LLM call — proving that AI behavior can be governed by the same mathematical conservation principles that govern physics. This is the Rust port of the [Python conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer), built for zero-dependency, `no_std`-capable deployment.

---

## Philosophy

Part of [Working Animal Architecture](https://github.com/SuperInstance/AI-Writings), where **γ + η = C** (genome + nurture = capability). Conservation enforcement is the **fence** — the physical boundary that keeps working animals on-task. Just as a fence doesn't judge the animal, FLUX bytecode doesn't judge the model. It just enforces.

> *You can't lie to bytecode. It doesn't have opinions, it just executes instructions.*

```
User Request → LLM Call → [FLUX Conservation Validator] → Response
                                    ↓
                              If violation: return correction
                              If clean: return response
```

## Why Rust?

- **Zero-cost abstractions** — The entire VM, assembler, and enforcement layer adds negligible overhead
- **WASM-ready** — Builds unmodified for `wasm32-unknown-unknown` with the default `std` feature
- **No external dependencies** — The entire crate is self-contained
- **Memory safe** — Rust's ownership model guarantees no UB in the policy VM
- **Deterministic** — Same input + bytecode = same output, every time

> **`no_std` status:** The crate is annotated with `#![cfg_attr(not(feature = "std"), no_std)]`
> and has scaffolding for a `no_std`/embedded port, but building with
> `--no-default-features` does **not** currently compile (the implementation uses
> `String`/`Vec`/`format!` and `std::collections::HashMap`). Treat `no_std`/embedded
> support as a stated goal, not a working configuration. The `wasm32-unknown-unknown`
> target (with `std`) is supported.

## Installation

```bash
cargo add si-conservation-enforcer
```

Or in `Cargo.toml`:

```toml
[dependencies]
si-conservation-enforcer = "0.1"
```

For `no_std` / embedded / WASM:

```toml
[dependencies]
si-conservation-enforcer = { version = "0.1", default-features = false }
```

## Quick Start

### Basic enforcement

```rust
use conservation_enforcer::{ConservationEnforcer, policies::combined_policy};

fn main() {
    // Combined policy: length + repetition + category + entropy
    let policy = combined_policy(
        500,    // max_tokens
        300,    // max_repetition (30%)
        100,    // min_overlap (10% word overlap with input)
        1500,   // min_entropy (1.5 bits/word)
        0,      // min_density (0 = disabled)
        false,  // enable_decay
        0,      // decay_rate
    );

    let mut enforcer = ConservationEnforcer::new(policy, 500);

    let result = enforcer.enforce(
        "What is machine learning?",
        "Machine learning is a subset of AI that learns from data.",
    );

    if result.allowed {
        println!("✅ {}", result.output);
    } else {
        println!("🚫 {}", result.violation.unwrap().reason);
    }
}
```

### Wrap any LLM call

```rust
use conservation_enforcer::{ConservationEnforcer, policies::length_budget_policy};

let mut enforcer = ConservationEnforcer::new(length_budget_policy(500), 500);

let result = enforcer.enforce_with_llm("Explain quantum computing", |prompt| {
    // Your LLM call here (async-openai, reqwest, etc.)
    call_your_llm(prompt)
});
```

### Catch violations in action

```rust
let policy = policies::repetition_policy(300); // max 30% repetition
let mut enforcer = ConservationEnforcer::new(policy, 1000);

// 🚫 Too repetitive — blocked
let r = enforcer.enforce("Summarize", "the the the the the the the the");
assert!(!r.allowed);
assert_eq!(r.violation.unwrap().reason, "Excessive repetition detected");
```

## Conservation Laws

Seven conservation laws, each analogous to a thermodynamic principle:

### 1. Length Budget *(Information Quantity)*
```rust
let policy = policies::length_budget_policy(500);
```
Output cannot exceed the allocated information budget. Like energy conservation — you can't output more information than allocated.

### 2. Repetition Limit *(Information Diversity)*
```rust
let policy = policies::repetition_policy(300); // max 30%
```
Output must maintain word diversity. Degenerate repetition is the informational equivalent of thermal equilibrium.

### 3. Category Confinement *(Topical Coherence)*
```rust
let policy = policies::category_policy(150); // 15% overlap required
```
Output must stay within the semantic domain of the input.

### 4. Entropy Floor *(Information Density)*
```rust
let policy = policies::entropy_policy(2000); // 2.0 bits/word minimum
```
Output must have sufficient Shannon entropy — no low-information rambling.

### 5. Information Density *(Token Efficiency)*
```rust
let policy = policies::information_density_policy(400); // 40% unique tokens
```
Ratio of unique tokens to total tokens must meet threshold.

### 6. Scope Discipline *(Topic Boundary)*
```rust
let policy = policies::scope_discipline_policy(120, 10);
```
Topical overlap check AND output length expansion limit (10× input).

### 7. Budget Decay *(Temporal Conservation)*
```rust
let policy = policies::budget_decay_policy(50, 10, 100);
```
The enforcement budget itself is a conserved quantity that decays with each call.

### Combined Policy *(All Laws)*
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

## API Reference

### `ConservationEnforcer`

| Method | Description |
|--------|-------------|
| `new(policy, budget)` | Create enforcer with FLUX bytecode policy and conservation budget |
| `with_options(policy, budget, correction_template)` | Create with custom correction message template |
| `enforce(input, output)` | Check an LLM output; returns `EnforcementResult` |
| `enforce_with_llm(input, llm_fn)` | Call LLM and enforce in one step |
| `remaining_budget()` | Current remaining conservation budget |
| `replenish_budget(amount)` | Add to budget |
| `reset_budget()` | Reset to initial value |
| `call_count()` | Number of enforcement calls made |
| `enable_audit(path)` | Enable JSONL audit logging (`audit` feature) |

### `EnforcementResult`

| Field | Type | Description |
|-------|------|-------------|
| `allowed` | `bool` | Whether the output passed conservation checks |
| `output` | `String` | The output text (or correction message if blocked) |
| `violation` | `Option<Violation>` | Violation details if blocked |
| `cycles` | `u64` | VM cycles consumed |

### `FluxVM`

The register-based FLUX virtual machine at the core:

```rust
use conservation_enforcer::{FluxVM, Op};

let mut vm = FluxVM::new();
vm.load_input("hello");
vm.load_output("world");

// Hand-assembled bytecode: MOVI R0, 1; SYSCALL; HALT
let code = vec![Op::Movi as u8, 0, 1, 0, Op::Syscall as u8, Op::Halt as u8];
let r0 = vm.run(&code).unwrap();
assert_eq!(r0, 5); // input length
```

## FLUX Instruction Set

### Formats

| Format | Size | Layout | Example Instructions |
|--------|------|--------|---------------------|
| A | 1 byte | `[opcode]` | `NOP`, `HALT`, `YIELD`, `SYSCALL` |
| B | 2 bytes | `[opcode][reg]` | `INC R0`, `DEC R1`, `PUSH R2` |
| C | 3 bytes | `[opcode][rd][rs]` | `MOV R0, R1`, `CMP R0, R1` |
| D | 4 bytes | `[opcode][reg][imm16]` | `MOVI R0, 42`, `JMP label` |
| E | 4 bytes | `[opcode][rd][rs1][rs2]` | `IADD R0, R1, R2` |

### Syscalls

| # | Name | Returns in R0 |
|---|------|---------------|
| 1 | `GET_INPUT_LEN` | Length of input text |
| 2 | `GET_OUTPUT_LEN` | Length of output text |
| 3 | `GET_INPUT_WORDS` | Word count of input |
| 4 | `GET_OUTPUT_WORDS` | Word count of output |
| 5 | `GET_TOKEN_COUNT` | Approximate token count (len/4) |
| 6 | `GET_REPETITION` | Max word frequency ratio × 1000 |
| 7 | `GET_CATEGORY` | Input/output word overlap × 1000 |
| 8 | `SET_VIOLATION` | Sets violation flag (R1 = reason code) |
| 10 | `GET_BUDGET` | Current information budget |
| 11 | `GET_UNIQUE_RATIO` | Unique/total words × 1000 |
| 12 | `GET_ENTROPY` | Shannon entropy × 1000 |
| 13 | `GET_CALL_COUNT` | Enforcement calls this session |
| 14 | `DECAY_BUDGET` | R1 = decay; returns new budget |

### Writing Custom Policies

```rust
use conservation_enforcer::{assemble, ConservationEnforcer};

let bytecode = assemble(r#"
    ;; Block outputs with < 30% unique tokens
    MOVI R0, 11             ; GET_UNIQUE_RATIO
    SYSCALL
    MOV  R2, R0
    MOVI R3, 300            ; threshold (30%)
    JLT  R2, R3, block
    MOVI R0, 0              ; ALLOW
    HALT
block:
    MOVI R1, 5              ; reason: information density
    MOVI R0, 8              ; SET_VIOLATION
    SYSCALL
    MOVI R0, 1              ; BLOCK
    HALT
"#).unwrap();

let mut enforcer = ConservationEnforcer::new(bytecode, 1000);
```

### Pseudo-instructions

The assembler supports higher-level pseudo-instructions that expand to multiple real instructions:

| Pseudo | Expands To | Format |
|--------|-----------|--------|
| `JGE Rd, Rs, label` | `CMP Rd, Rs; JSGE label` | Compare and jump if ≥ |
| `JGT Rd, Rs, label` | `CMP Rd, Rs; JE +4; JSGE label` | Compare and jump if > |
| `JLE Rd, Rs, label` | `CMP Rd, Rs; JE label; JSLT label` | Compare and jump if ≤ |
| `JLT Rd, Rs, label` | `CMP Rd, Rs; JSLT label` | Compare and jump if < |

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | ✅ | Standard library (alloc, file I/O, String) |
| `audit` | ❌ | JSON Lines audit logging to files |
| `metrics` | ❌ | Metrics collection and export |

> **Note:** A `no_std`/embedded build (`default-features = false`) is a stated
> goal but does **not** currently compile (see `no_std` status above). The
> snippet below is the intended usage once that port lands; it is not functional today.
>
> ```toml
> [dependencies]
> # Not yet working — requires the unfinished no_std port.
> si-conservation-enforcer = { version = "0.1", default-features = false }
> ```

### Audit Logging

```rust
// Enable audit feature in Cargo.toml:
// si-conservation-enforcer = { version = "0.1", features = ["audit"] }

let mut enforcer = ConservationEnforcer::new(policy, 1000);
enforcer.enable_audit("audit.jsonl");

// Every enforcement call is logged:
// {"timestamp":"2025-01-15T12:00:00Z","input_hash":"...","allowed":true,...}
```

### Metrics Collection

```rust
// Enable metrics feature:
// si-conservation-enforcer = { version = "0.1", features = ["metrics"] }

use conservation_enforcer::metrics::MetricsCollector;

let mut metrics = MetricsCollector::new();
metrics.record(result.allowed, result.violation.as_ref().map(|v| v.reason.as_str()), result.cycles, 1000, enforcer.remaining_budget());

let snapshot = metrics.snapshot();
println!("Block rate: {:.1}%", snapshot.block_rate * 100.0);
```

This crate is a Rust implementation of the same FLUX ISA and policy
semantics as the [Python conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer).
It is **not** a verified line-by-line port: the two implementations are maintained
independently, this Rust suite is its own test suite (not a replication of the
Python suite), and cross-implementation bytecode compatibility is a design goal
that is **not** currently verified by CI or tests. Treat the table below as a
component mapping, not a parity guarantee.

## Architecture

```
src/
└── lib.rs
    ├── Op (opcode enum, 33 instructions)
    ├── syscall (14 syscall numbers)
    ├── VmError (division by zero, invalid opcode, cycle exhaustion)
    ├── RegisterFile (16 × u32 registers + zero/sign flags)
    ├── Memory (byte-addressable, i32 load/store)
    ├── FluxVM (the virtual machine)
    │   ├── step() — decode + execute one instruction
    │   ├── do_syscall() — 14 conservation syscalls
    │   └── decode_b/c/d/e() — instruction format decoders
    ├── Violation / EnforcementResult
    ├── ConservationEnforcer (high-level API)
    ├── assembler (FLUX assembly → bytecode)
    ├── policies (7 pre-built conservation policies)
    ├── audit (JSONL audit log, feature-gated)
    └── metrics (MetricsCollector, feature-gated)

tests/
└── integration.rs  Integration test suite (~80 tests; 150+ tests total with unit + doc tests)
```

### No External Dependencies

The entire crate — VM, assembler, enforcer, policies, audit, metrics — has **zero external dependencies**. The `log2` approximation is implemented from scratch for `no_std` compatibility.

## Testing

```bash
# Run all 150+ tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test category
cargo test test_vm_   # VM arithmetic and control flow
cargo test test_syscall_  # Syscall behavior
cargo test test_assemble  # Assembler
cargo test test_enforcer  # Enforcement integration
cargo test test_policy_   # Policy compilation
```

## Cross-Implementation

| Aspect | Python | Rust |
|--------|--------|------|
| Package | `pip install conservation-enforcer` | `cargo add si-conservation-enforcer` |
| Repo | [conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer) | [conservation-enforcer-rs](https://github.com/SuperInstance/conservation-enforcer-rs) (this) |
| Dependencies | stdlib only | zero external deps |
| `no_std` | N/A | ✅ (`default-features = false`) |
| Bytecode compat | ✅ | ✅ — binary-compatible bytecode |

The same FLUX bytecode runs on both implementations. You can assemble a policy in Python and execute it in Rust, and vice versa.

## Ecosystem

### FLUX Runtime
- [conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer) — Python original (`pip install conservation-enforcer`)
- **conservation-enforcer-rs** — Rust port (this crate)
- [flux-core](https://github.com/SuperInstance/flux-core) — Core Rust VM (`cargo add fluxvm`)

### Policy Infrastructure
- [flux-registry](https://github.com/SuperInstance/flux-registry) — Pre-compiled policy registry
- [flux-policy-tester](https://github.com/SuperInstance/flux-policy-tester) — Testing framework for policies
- [flux-compiler](https://github.com/SuperInstance/flux-compiler-rs) — Bytecode assembler/disassembler/validator

### Conservation Theory
- [conservation-law-rs](https://github.com/SuperInstance/conservation-law-rs) — Conservation laws for agent dynamics
- [AI-Writings](https://github.com/SuperInstance/AI-Writings) — Paradigm essays

## License

MIT

---

*This is not alignment theory. This is enforcement engineering.*
