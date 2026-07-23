# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-07-22

### Added
- CHANGELOG.md following Keep-A-Changelog format
- Regression test for cycle-budget tracking on empty policy (empty_policy_with_zero_budget_denies)

### Changed
- Bumped version to 0.1.1

## [0.1.0] - Initial Release

First published release to crates.io:
- FluxVM 0.1.1 API alignment
- 7 unit tests covering core enforcement behavior
- ConservationEnforcer with policy bytecode execution, budget tracking, and audit logging
