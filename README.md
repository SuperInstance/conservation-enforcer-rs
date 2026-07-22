# conservation-enforcer-rs

Rust implementation of the Conservation Enforcer for FLUX bytecode conservation-law enforcement.

## Description

This is a Rust library for enforcing conservation laws on LLM outputs using FLUX bytecode, inspired by the Python `conservation-enforcer` package.

## Installation

Add this to your `Cargo.toml`:

```toml
conservation-enforcer-rs = { git = "https://github.com/SuperInstance/conservation-enforcer-rs" }
```

## Usage

```rust
use conservation_enforcer_rs::{ConservationEnforcer, EnforcementResult};

fn main() {
    // Example policy bytecode (replace with actual FLUX bytecode)
    let policy_bytecode = vec![0, 1, 2, 3];
    let mut enforcer = ConservationEnforcer::new(
        policy_bytecode,
        1000, // budget
        None, // correction template (use default)
        false, // enable_audit
        None, // audit_path
    );

    let result = enforcer.enforce("What is AI?", "AI is intelligent.");
    if result.allowed {
        println!("Allowed: {}", result.output);
    } else {
        println!("Blocked: {:?}", result.violation);
    }
}
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
