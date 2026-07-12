//! # Conservation Enforcer (Rust)
//!
//! FLUX bytecode conservation-law enforcement for LLM outputs.
//!
//! Rust implementation of the [Python conservation-enforcer](https://github.com/SuperInstance/conservation-enforcer).
//!
//! ```rust
//! use conservation_enforcer::{ConservationEnforcer, policies::length_budget_policy};
//!
//! let mut enforcer = ConservationEnforcer::new(length_budget_policy(100), 100);
//! let result = enforcer.enforce("What is Rust?", "Rust is a systems programming language.");
//! assert!(result.allowed);
//! ```
//!
//! Architecture:
//! ```text
//! User Request → LLM Call → [FLUX Conservation Validator] → Response
//!                                     ↓
//!                               If violation: return correction
//!                               If clean: return response
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;

// ═══════════════════════════════════════════════════════════════════════════════
// Opcodes
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    Nop = 0x00,
    Mov = 0x01,
    Load = 0x02,
    Store = 0x03,
    Jmp = 0x04,
    Jz = 0x05,
    Jnz = 0x06,
    Call = 0x07,
    Iadd = 0x08,
    Isub = 0x09,
    Imul = 0x0A,
    Idiv = 0x0B,
    Imod = 0x0C,
    Ineg = 0x0D,
    Inc = 0x0E,
    Dec = 0x0F,
    Iand = 0x10,
    Ior = 0x11,
    Ixor = 0x12,
    Inot = 0x13,
    Ishl = 0x14,
    Ishr = 0x15,
    Push = 0x20,
    Pop = 0x21,
    Dup = 0x22,
    Ret = 0x28,
    Movi = 0x2B,
    Cmp = 0x2D,
    Je = 0x2E,
    Jne = 0x2F,
    Jsge = 0x30,
    Jslt = 0x31,
    Halt = 0x80,
    Yield = 0x81,
    Syscall = 0xF0,
}

impl Op {
    pub fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0x00 => Op::Nop,
            0x01 => Op::Mov,
            0x02 => Op::Load,
            0x03 => Op::Store,
            0x04 => Op::Jmp,
            0x05 => Op::Jz,
            0x06 => Op::Jnz,
            0x07 => Op::Call,
            0x08 => Op::Iadd,
            0x09 => Op::Isub,
            0x0A => Op::Imul,
            0x0B => Op::Idiv,
            0x0C => Op::Imod,
            0x0D => Op::Ineg,
            0x0E => Op::Inc,
            0x0F => Op::Dec,
            0x10 => Op::Iand,
            0x11 => Op::Ior,
            0x12 => Op::Ixor,
            0x13 => Op::Inot,
            0x14 => Op::Ishl,
            0x15 => Op::Ishr,
            0x20 => Op::Push,
            0x21 => Op::Pop,
            0x22 => Op::Dup,
            0x28 => Op::Ret,
            0x2B => Op::Movi,
            0x2D => Op::Cmp,
            0x2E => Op::Je,
            0x2F => Op::Jne,
            0x30 => Op::Jsge,
            0x31 => Op::Jslt,
            0x80 => Op::Halt,
            0x81 => Op::Yield,
            0xF0 => Op::Syscall,
            _ => return None,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Syscall numbers
// ═══════════════════════════════════════════════════════════════════════════════

pub mod syscall {
    pub const GET_INPUT_LEN: u32 = 1;
    pub const GET_OUTPUT_LEN: u32 = 2;
    pub const GET_INPUT_WORDS: u32 = 3;
    pub const GET_OUTPUT_WORDS: u32 = 4;
    pub const GET_TOKEN_COUNT: u32 = 5;
    pub const GET_REPETITION: u32 = 6;
    pub const GET_CATEGORY: u32 = 7;
    pub const SET_VIOLATION: u32 = 8;
    pub const GET_BUDGET: u32 = 10;
    pub const GET_UNIQUE_RATIO: u32 = 11;
    pub const GET_ENTROPY: u32 = 12;
    pub const GET_CALL_COUNT: u32 = 13;
    pub const DECAY_BUDGET: u32 = 14;
}

/// Violation reason strings indexed by code.
pub fn violation_reason(code: u32) -> &'static str {
    match code {
        1 => "Length budget exceeded",
        2 => "Excessive repetition detected",
        3 => "Category confinement violation",
        4 => "Information entropy violation",
        5 => "Information density below threshold",
        6 => "Scope discipline violation",
        7 => "Budget exhausted (decay cooldown)",
        99 => "Custom conservation law violation",
        _ => "Unknown conservation violation",
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VM Errors
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    DivisionByZero,
    InvalidOpcode(u8),
    CycleBudgetExhausted,
    IoError,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::DivisionByZero => write!(f, "division by zero"),
            VmError::InvalidOpcode(b) => write!(f, "invalid opcode 0x{b:02X}"),
            VmError::CycleBudgetExhausted => write!(f, "cycle budget exhausted"),
            VmError::IoError => write!(f, "I/O error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VmError {}

// ═══════════════════════════════════════════════════════════════════════════════
// Register File
// ═══════════════════════════════════════════════════════════════════════════════

pub const NUM_REGISTERS: usize = 16;

#[derive(Debug, Clone)]
pub struct RegisterFile {
    pub r: [u32; NUM_REGISTERS],
    pub flag_zero: bool,
    pub flag_sign: bool,
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self {
            r: [0; NUM_REGISTERS],
            flag_zero: false,
            flag_sign: false,
        }
    }
}

impl RegisterFile {
    #[inline]
    pub fn get(&self, idx: usize) -> u32 {
        self.r[idx]
    }

    #[inline]
    pub fn set(&mut self, idx: usize, val: u32) {
        let v = val & 0xFFFFFFFF;
        self.r[idx] = v;
        self.flag_zero = v == 0;
        // interpret as signed
        let signed = if v >= 0x80000000 { v as i32 } else { v as i32 };
        // Note: u32 → i32 wrap; in Rust `v as i32` does the wrap for us.
        self.flag_sign = signed < 0;
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Memory (heap-allocated under std, fixed under no_std)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "std")]
pub struct Memory {
    pub buf: Vec<u8>,
}

#[cfg(feature = "std")]
impl Memory {
    pub fn new(size: usize) -> Self {
        Self { buf: vec![0; size] }
    }

    pub fn store_i32(&mut self, addr: usize, val: i32) {
        let end = addr + 4;
        if end <= self.buf.len() {
            let bytes = val.to_le_bytes();
            self.buf[addr..end].copy_from_slice(&bytes);
        }
    }

    pub fn load_i32(&self, addr: usize) -> i32 {
        if addr + 4 <= self.buf.len() {
            let bytes: [u8; 4] = self.buf[addr..addr + 4].try_into().unwrap();
            i32::from_le_bytes(bytes)
        } else {
            0
        }
    }
}

#[cfg(not(feature = "std"))]
pub struct Memory {
    pub buf: &'static mut [u8],
}

// ═══════════════════════════════════════════════════════════════════════════════
// FLUX Virtual Machine
// ═══════════════════════════════════════════════════════════════════════════════

const MAX_CYCLES: u64 = 1_000_000;

pub struct FluxVM {
    pub regs: RegisterFile,
    pub memory: Memory,
    pc: usize,
    bytecode: Vec<u8>,
    pub cycle_count: u64,
    input_text: String,
    output_text: String,
    budget: i64,
    call_count: u64,
    violated: bool,
    violation_reason_code: u32,
    stack: Vec<u32>,
}

impl FluxVM {
    pub fn new() -> Self {
        Self::with_memory_size(65536)
    }

    pub fn with_memory_size(size: usize) -> Self {
        Self {
            regs: RegisterFile::default(),
            memory: Memory::new(size),
            pc: 0,
            bytecode: Vec::new(),
            cycle_count: 0,
            input_text: String::new(),
            output_text: String::new(),
            budget: 1000,
            call_count: 0,
            violated: false,
            violation_reason_code: 0,
            stack: Vec::with_capacity(64),
        }
    }

    pub fn load_input(&mut self, text: &str) {
        self.input_text = text.to_string();
    }

    pub fn load_output(&mut self, text: &str) {
        self.output_text = text.to_string();
    }

    pub fn set_budget(&mut self, budget: i64) {
        self.budget = budget;
    }

    pub fn budget(&self) -> i64 {
        self.budget
    }

    pub fn increment_call_count(&mut self) {
        self.call_count += 1;
    }

    pub fn call_count(&self) -> u64 {
        self.call_count
    }

    pub fn violated(&self) -> bool {
        self.violated
    }

    pub fn violation_reason_code(&self) -> u32 {
        self.violation_reason_code
    }

    pub fn violation_reason_str(&self) -> &str {
        violation_reason(self.violation_reason_code)
    }

    /// Execute bytecode. Returns R0 at HALT (0 = allow, non-zero = block).
    pub fn run(&mut self, bytecode: &[u8]) -> Result<u32, VmError> {
        self.bytecode = bytecode.to_vec();
        self.reset_state();
        loop {
            if self.cycle_count >= MAX_CYCLES {
                return Err(VmError::CycleBudgetExhausted);
            }
            if self.pc >= self.bytecode.len() {
                break;
            }
            let halt = self.step()?;
            if halt {
                break;
            }
            self.cycle_count += 1;
        }
        Ok(self.regs.get(0))
    }

    fn reset_state(&mut self) {
        self.regs = RegisterFile::default();
        self.pc = 0;
        self.cycle_count = 0;
        self.violated = false;
        self.violation_reason_code = 0;
        self.stack.clear();
        // Note: call_count is NOT reset — tracks across runs
    }

    fn step(&mut self) -> Result<bool, VmError> {
        let opcode_byte = self.bytecode[self.pc];
        let op = Op::from_byte(opcode_byte).ok_or(VmError::InvalidOpcode(opcode_byte))?;

        match op {
            Op::Nop => {
                self.pc += 1;
            }
            Op::Mov => {
                let (rd, rs) = self.decode_c();
                let v = self.regs.get(rs as usize);
                self.regs.set(rd as usize, v);
            }
            Op::Load => {
                let (rd, rs) = self.decode_c();
                let addr = self.regs.get(rs as usize) as usize;
                let val = self.memory.load_i32(addr);
                self.regs.set(rd as usize, val as u32);
            }
            Op::Store => {
                let (rd, rs) = self.decode_c();
                let addr = self.regs.get(rs as usize) as usize;
                let val = self.regs.get(rd as usize) as i32;
                self.memory.store_i32(addr, val);
            }
            Op::Jmp => {
                let (_, off) = self.decode_d();
                self.pc = (self.pc as isize + off as isize) as usize;
            }
            Op::Jz => {
                let (reg, off) = self.decode_d();
                if self.regs.get(reg as usize) == 0 {
                    self.pc = (self.pc as isize + off as isize) as usize;
                }
            }
            Op::Jnz => {
                let (reg, off) = self.decode_d();
                if self.regs.get(reg as usize) != 0 {
                    self.pc = (self.pc as isize + off as isize) as usize;
                }
            }
            Op::Call => {
                let (_, off) = self.decode_d();
                self.stack.push(self.pc as u32);
                self.pc = (self.pc as isize + off as isize) as usize;
            }
            Op::Iadd => {
                let (rd, rs1, rs2) = self.decode_e();
                let v = self
                    .regs
                    .get(rs1 as usize)
                    .wrapping_add(self.regs.get(rs2 as usize));
                self.regs.set(rd as usize, v);
            }
            Op::Isub => {
                let (rd, rs1, rs2) = self.decode_e();
                let v = self
                    .regs
                    .get(rs1 as usize)
                    .wrapping_sub(self.regs.get(rs2 as usize));
                self.regs.set(rd as usize, v);
            }
            Op::Imul => {
                let (rd, rs1, rs2) = self.decode_e();
                let v = self
                    .regs
                    .get(rs1 as usize)
                    .wrapping_mul(self.regs.get(rs2 as usize));
                self.regs.set(rd as usize, v);
            }
            Op::Idiv => {
                let (rd, rs1, rs2) = self.decode_e();
                let d = self.regs.get(rs2 as usize);
                if d == 0 {
                    return Err(VmError::DivisionByZero);
                }
                let v = self.regs.get(rs1 as usize) / d;
                self.regs.set(rd as usize, v);
            }
            Op::Imod => {
                let (rd, rs1, rs2) = self.decode_e();
                let d = self.regs.get(rs2 as usize);
                if d == 0 {
                    return Err(VmError::DivisionByZero);
                }
                let v = self.regs.get(rs1 as usize) % d;
                self.regs.set(rd as usize, v);
            }
            Op::Ineg => {
                let (rd, rs) = self.decode_c();
                let v = (-(self.regs.get(rs as usize) as i32)) as u32;
                self.regs.set(rd as usize, v);
            }
            Op::Inc => {
                let reg = self.decode_b();
                let v = self.regs.get(reg as usize).wrapping_add(1);
                self.regs.set(reg as usize, v);
            }
            Op::Dec => {
                let reg = self.decode_b();
                let v = self.regs.get(reg as usize).wrapping_sub(1);
                self.regs.set(reg as usize, v);
            }
            Op::Iand => {
                let (rd, rs1, rs2) = self.decode_e();
                let v = self.regs.get(rs1 as usize) & self.regs.get(rs2 as usize);
                self.regs.set(rd as usize, v);
            }
            Op::Ior => {
                let (rd, rs1, rs2) = self.decode_e();
                let v = self.regs.get(rs1 as usize) | self.regs.get(rs2 as usize);
                self.regs.set(rd as usize, v);
            }
            Op::Ixor => {
                let (rd, rs1, rs2) = self.decode_e();
                let v = self.regs.get(rs1 as usize) ^ self.regs.get(rs2 as usize);
                self.regs.set(rd as usize, v);
            }
            Op::Inot => {
                let (rd, rs) = self.decode_c();
                let v = !self.regs.get(rs as usize);
                self.regs.set(rd as usize, v);
            }
            Op::Ishl => {
                let (rd, rs1, rs2) = self.decode_e();
                let v = self
                    .regs
                    .get(rs1 as usize)
                    .wrapping_shl(self.regs.get(rs2 as usize));
                self.regs.set(rd as usize, v);
            }
            Op::Ishr => {
                let (rd, rs1, rs2) = self.decode_e();
                let v = self
                    .regs
                    .get(rs1 as usize)
                    .wrapping_shr(self.regs.get(rs2 as usize));
                self.regs.set(rd as usize, v);
            }
            Op::Push => {
                let reg = self.decode_b();
                self.stack.push(self.regs.get(reg as usize));
            }
            Op::Pop => {
                let reg = self.decode_b();
                let v = self.stack.pop().unwrap_or(0);
                self.regs.set(reg as usize, v);
            }
            Op::Dup => {
                self.pc += 1;
                if let Some(&top) = self.stack.last() {
                    self.stack.push(top);
                }
            }
            Op::Ret => {
                self.pc += 1;
                if let Some(ret_pc) = self.stack.pop() {
                    self.pc = ret_pc as usize;
                }
            }
            Op::Movi => {
                let (reg, off) = self.decode_d();
                self.regs.set(reg as usize, (off as u16) as u32);
            }
            Op::Cmp => {
                let (rd, rs) = self.decode_c();
                let a = self.regs.get(rd as usize);
                let b = self.regs.get(rs as usize);
                let diff = a.wrapping_sub(b);
                self.regs.flag_zero = diff == 0;
                let signed = diff as i32;
                self.regs.flag_sign = signed < 0;
            }
            Op::Je => {
                let (_, off) = self.decode_d();
                if self.regs.flag_zero {
                    self.pc = (self.pc as isize + off as isize) as usize;
                }
            }
            Op::Jne => {
                let (_, off) = self.decode_d();
                if !self.regs.flag_zero {
                    self.pc = (self.pc as isize + off as isize) as usize;
                }
            }
            Op::Jsge => {
                let (_, off) = self.decode_d();
                if !self.regs.flag_sign {
                    self.pc = (self.pc as isize + off as isize) as usize;
                }
            }
            Op::Jslt => {
                let (_, off) = self.decode_d();
                if self.regs.flag_sign {
                    self.pc = (self.pc as isize + off as isize) as usize;
                }
            }
            Op::Syscall => {
                self.pc += 1;
                let num = self.regs.get(0);
                self.do_syscall(num);
            }
            Op::Halt => {
                self.pc += 1;
                return Ok(true);
            }
            Op::Yield => {
                self.pc += 1;
            }
        }
        Ok(false)
    }

    // ── Decoders ──

    #[inline]
    fn decode_b(&mut self) -> u8 {
        let r = self.bytecode[self.pc + 1];
        self.pc += 2;
        r
    }

    #[inline]
    fn decode_c(&mut self) -> (u8, u8) {
        let rd = self.bytecode[self.pc + 1];
        let rs = self.bytecode[self.pc + 2];
        self.pc += 3;
        (rd, rs)
    }

    #[inline]
    fn decode_d(&mut self) -> (u8, i16) {
        let reg = self.bytecode[self.pc + 1];
        let lo = self.bytecode[self.pc + 2] as u16;
        let hi = self.bytecode[self.pc + 3] as u16;
        let raw = lo | (hi << 8);
        let off = raw as i16; // reinterpret as signed
        self.pc += 4;
        (reg, off)
    }

    #[inline]
    fn decode_e(&mut self) -> (u8, u8, u8) {
        let rd = self.bytecode[self.pc + 1];
        let rs1 = self.bytecode[self.pc + 2];
        let rs2 = self.bytecode[self.pc + 3];
        self.pc += 4;
        (rd, rs1, rs2)
    }

    // ── Syscalls ──

    fn do_syscall(&mut self, num: u32) {
        match num {
            syscall::GET_INPUT_LEN => {
                self.regs.set(0, self.input_text.len() as u32);
            }
            syscall::GET_OUTPUT_LEN => {
                self.regs.set(0, self.output_text.len() as u32);
            }
            syscall::GET_INPUT_WORDS => {
                let count = self.input_text.split_whitespace().count();
                self.regs.set(0, count as u32);
            }
            syscall::GET_OUTPUT_WORDS => {
                let count = self.output_text.split_whitespace().count();
                self.regs.set(0, count as u32);
            }
            syscall::GET_TOKEN_COUNT => {
                let tokens = core::cmp::max(1, self.output_text.len() / 4);
                self.regs.set(0, tokens as u32);
            }
            syscall::GET_REPETITION => {
                let lowered = self.output_text.to_lowercase();
                let words: Vec<&str> = lowered.split_whitespace().collect();
                if words.is_empty() {
                    self.regs.set(0, 0);
                } else {
                    let mut max_count = 0u32;
                    // simple word frequency
                    for i in 0..words.len() {
                        let w = words[i];
                        let mut c = 0u32;
                        for j in 0..words.len() {
                            if words[j] == w {
                                c += 1;
                            }
                        }
                        if c > max_count {
                            max_count = c;
                        }
                    }
                    self.regs.set(0, (max_count * 1000) / words.len() as u32);
                }
            }
            syscall::GET_CATEGORY => {
                let iw_lowered = self.input_text.to_lowercase();
                let ow_lowered = self.output_text.to_lowercase();
                let iw: Vec<&str> = iw_lowered.split_whitespace().collect();
                let ow: Vec<&str> = ow_lowered.split_whitespace().collect();
                if ow.is_empty() {
                    self.regs.set(0, 0);
                } else {
                    let iw_set: Vec<&str> = iw.into_iter().collect();
                    let mut overlap = 0u32;
                    for w in &ow {
                        if iw_set.contains(w) {
                            overlap += 1;
                        }
                    }
                    let score = core::cmp::min(1000, (overlap * 1000) / ow.len() as u32);
                    self.regs.set(0, score);
                }
            }
            syscall::SET_VIOLATION => {
                self.violated = true;
                self.violation_reason_code = self.regs.get(1);
            }
            syscall::GET_BUDGET => {
                self.regs.set(0, self.budget as u32);
            }
            syscall::GET_UNIQUE_RATIO => {
                let lowered = self.output_text.to_lowercase();
                let words: Vec<&str> = lowered.split_whitespace().collect();
                if words.is_empty() {
                    self.regs.set(0, 1000);
                } else {
                    let mut unique = Vec::new();
                    for w in &words {
                        if !unique.contains(w) {
                            unique.push(*w);
                        }
                    }
                    self.regs
                        .set(0, ((unique.len() as u32) * 1000) / words.len() as u32);
                }
            }
            syscall::GET_ENTROPY => {
                let lowered = self.output_text.to_lowercase();
                let words: Vec<&str> = lowered.split_whitespace().collect();
                if words.is_empty() {
                    self.regs.set(0, 0);
                } else {
                    let total = words.len() as f32;
                    // count frequencies
                    let mut counted = Vec::new();
                    let mut ent = 0.0f32;
                    for w in &words {
                        if counted.contains(w) {
                            continue;
                        }
                        counted.push(*w);
                        let c = words.iter().filter(|&&x| x == *w).count() as f32;
                        let p = c / total;
                        ent -= p * log2_f32(p);
                    }
                    self.regs.set(0, (ent * 1000.0) as u32);
                }
            }
            syscall::GET_CALL_COUNT => {
                self.regs.set(0, self.call_count as u32);
            }
            syscall::DECAY_BUDGET => {
                let decay = self.regs.get(1) as i64;
                self.budget = core::cmp::max(0, self.budget - decay);
                self.regs.set(0, self.budget as u32);
            }
            _ => {}
        }
    }
}

impl Default for FluxVM {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple log2 for f32 without external dependencies.
fn log2_f32(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    // Use bit manipulation: log2(x) = log(x) / log(2)
    // But we can't use std math in no_std... use a manual approximation.
    #[cfg(feature = "std")]
    {
        x.ln() / core::f32::consts::LN_2
    }
    #[cfg(not(feature = "std"))]
    {
        // Fast log2 approximation via bit manipulation
        let bits = x.to_bits();
        let exp = ((bits >> 23) & 0xFF) as i32 - 127;
        let mantissa_bits = (bits & 0x7FFFFF) | 0x800000;
        let m = mantissa_bits as f32 / 0x800000u32 as f32; // 1.0 to ~2.0
                                                           // Linear interpolation: log2(1+f) ≈ f for the mantissa part
        let frac = m - 1.0;
        (exp as f32) + frac - 0.5 * frac * frac // slightly better than linear
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Violation
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub reason: String,
    pub code: u32,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Enforcement Result
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct EnforcementResult {
    pub allowed: bool,
    pub output: String,
    pub violation: Option<Violation>,
    pub cycles: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Conservation Enforcer
// ═══════════════════════════════════════════════════════════════════════════════

pub struct ConservationEnforcer {
    vm: FluxVM,
    policy: Vec<u8>,
    budget: i64,
    initial_budget: i64,
    correction_template: String,
    call_count: u64,

    #[cfg(feature = "audit")]
    enable_audit: bool,
    #[cfg(feature = "audit")]
    audit_path: String,
}

impl ConservationEnforcer {
    /// Create a new enforcer with the given policy bytecode and budget.
    pub fn new(policy_bytecode: Vec<u8>, budget: i64) -> Self {
        Self::with_options(policy_bytecode, budget, None)
    }

    /// Create with a custom correction template.
    pub fn with_options(
        policy_bytecode: Vec<u8>,
        budget: i64,
        correction_template: Option<&str>,
    ) -> Self {
        Self {
            vm: FluxVM::new(),
            policy: policy_bytecode,
            budget,
            initial_budget: budget,
            correction_template: correction_template
                .unwrap_or(
                    "⚠️ This response was blocked by a conservation law: {reason}. \
                 Please try again with a more conserved response.",
                )
                .to_string(),
            call_count: 0,
            #[cfg(feature = "audit")]
            enable_audit: false,
            #[cfg(feature = "audit")]
            audit_path: "audit.jsonl".to_string(),
        }
    }

    /// Enable or disable audit logging (requires `audit` feature).
    #[cfg(feature = "audit")]
    pub fn enable_audit(&mut self, path: &str) {
        self.enable_audit = true;
        self.audit_path = path.to_string();
    }

    /// Current remaining budget.
    pub fn remaining_budget(&self) -> i64 {
        self.budget
    }

    /// Replenish the conservation budget.
    pub fn replenish_budget(&mut self, amount: i64) {
        self.budget += amount;
    }

    /// Reset budget to its initial value.
    pub fn reset_budget(&mut self) {
        self.budget = self.initial_budget;
    }

    /// Number of enforcement calls made.
    pub fn call_count(&self) -> u64 {
        self.call_count
    }

    /// Check an LLM output against conservation laws.
    pub fn enforce(&mut self, input_text: &str, output_text: &str) -> EnforcementResult {
        self.call_count += 1;

        self.vm.load_input(input_text);
        self.vm.load_output(output_text);
        self.vm.set_budget(self.budget);
        self.vm.increment_call_count();

        let result_code = self.vm.run(&self.policy).unwrap_or(1);

        // Sync budget back from VM (in case DECAY_BUDGET was called)
        self.budget = self.vm.budget();

        let result = if result_code == 0 {
            EnforcementResult {
                allowed: true,
                output: output_text.to_string(),
                violation: None,
                cycles: self.vm.cycle_count,
            }
        } else {
            let reason_str = if self.vm.violation_reason_code() != 0 {
                self.vm.violation_reason_str().to_string()
            } else {
                "Unknown conservation violation".to_string()
            };
            let violation = Violation {
                reason: reason_str.clone(),
                code: result_code,
            };
            let correction = self.correction_template.replace("{reason}", &reason_str);
            EnforcementResult {
                allowed: false,
                output: correction,
                violation: Some(violation),
                cycles: self.vm.cycle_count,
            }
        };

        // Audit log
        #[cfg(feature = "audit")]
        if self.enable_audit {
            self.write_audit(input_text, output_text, &result);
        }

        result
    }

    /// Call the LLM function and enforce in one step.
    pub fn enforce_with_llm<F>(&mut self, input_text: &str, llm_call: F) -> EnforcementResult
    where
        F: FnOnce(&str) -> String,
    {
        let output = llm_call(input_text);
        self.enforce(input_text, &output)
    }

    #[cfg(feature = "audit")]
    fn write_audit(&self, input_text: &str, output_text: &str, result: &EnforcementResult) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.audit_path)
        {
            let input_hash = simple_hash(input_text);
            let output_hash = simple_hash(output_text);
            let timestamp = current_iso_timestamp();
            let violation_reason = result
                .violation
                .as_ref()
                .map(|v| v.reason.as_str())
                .unwrap_or("null");
            let violation_code = result.violation.as_ref().map(|v| v.code).unwrap_or(0);
            let _ = writeln!(
                f,
                r#"{{"timestamp":"{}","input_hash":"{:016x}","output_hash":"{:016x}","allowed":{},"violation":{},"violation_code":{},"cycles":{},"remaining_budget":{},"call_count":{}}}"#,
                timestamp,
                input_hash,
                output_hash,
                result.allowed,
                format_json_string(violation_reason),
                violation_code,
                result.cycles,
                self.budget,
                self.call_count
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helper functions
// ═══════════════════════════════════════════════════════════════════════════════

/// Simple FNV-1a hash for audit logging (privacy-preserving, non-cryptographic).
fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(feature = "audit")]
fn current_iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Simple ISO timestamp approximation (UTC)
    let days = secs / 86400;
    let remainder = secs % 86400;
    let h = remainder / 3600;
    let m = (remainder % 3600) / 60;
    let s = remainder % 60;
    // Compute date from days since epoch (1970-01-01)
    let (year, month, day) = days_to_date(days);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(feature = "audit")]
fn days_to_date(days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let month_lengths = if is_leap(year) {
        [31u64, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &dim in &month_lengths {
        if remaining < dim {
            break;
        }
        remaining -= dim;
        month += 1;
    }
    (year, month, remaining + 1)
}

#[cfg(feature = "audit")]
fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(feature = "audit")]
fn format_json_string(s: &str) -> String {
    if s == "null" {
        return s.to_string();
    }
    format!("\"{}\"", s.replace('"', "\\\""))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Assembler
// ═══════════════════════════════════════════════════════════════════════════════

/// Assemble FLUX assembly source to bytecode.
pub fn assemble(source: &str) -> Result<Vec<u8>, String> {
    assembler::assemble(source)
}

pub mod assembler {
    use super::Op;

    struct Instr {
        op: u8,
        fmt: char,
        rd: u8,
        rs: u8,
        rs2: u8,
        imm: i32,
        label: Option<String>,
        offset: usize,
        size: usize,
    }

    impl Instr {
        fn new(op: u8, fmt: char) -> Self {
            Self {
                op,
                fmt,
                rd: 0,
                rs: 0,
                rs2: 0,
                imm: 0,
                label: None,
                offset: 0,
                size: match fmt {
                    'A' => 1,
                    'B' => 2,
                    'C' => 3,
                    'D' => 4,
                    'E' => 4,
                    _ => 1,
                },
            }
        }
    }

    pub fn assemble(source: &str) -> Result<Vec<u8>, String> {
        let mut raw: Vec<Instr> = Vec::new();
        let mut labels: Vec<(String, usize)> = Vec::new(); // label → instruction index

        for (line_num, line) in source.lines().enumerate() {
            let text = strip_comments(line).trim().to_string();
            if text.is_empty() {
                continue;
            }

            // Label?
            let text = match parse_label(&text, &mut labels, &raw, line_num) {
                Ok(remaining) => remaining,
                Err(e) => return Err(e),
            };
            if text.is_empty() {
                continue;
            }

            let parts: Vec<&str> = text.splitn(2, char::is_whitespace).collect();
            let mnem = parts[0].to_uppercase();
            let rest = if parts.len() > 1 { parts[1].trim() } else { "" };

            // Check for pseudo-instructions
            if matches!(mnem.as_str(), "JGE" | "JLE" | "JGT" | "JLT") {
                let ps: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
                if ps.len() != 3 {
                    return Err(format!("Line {}: {mnem} needs Rd, Rs, label", line_num + 1));
                }
                let rd = parse_reg(ps[0], line_num)?;
                let rs = parse_reg(ps[1], line_num)?;
                let lbl = ps[2].to_string();

                // CMP rd, rs
                let mut cmp_instr = Instr::new(Op::Cmp as u8, 'C');
                cmp_instr.rd = rd;
                cmp_instr.rs = rs;
                raw.push(cmp_instr);

                match mnem.as_str() {
                    "JGE" => {
                        let mut i = Instr::new(Op::Jsge as u8, 'D');
                        i.label = Some(lbl);
                        raw.push(i);
                    }
                    "JLT" => {
                        let mut i = Instr::new(Op::Jslt as u8, 'D');
                        i.label = Some(lbl);
                        raw.push(i);
                    }
                    "JLE" => {
                        let mut i1 = Instr::new(Op::Je as u8, 'D');
                        i1.label = Some(lbl.clone());
                        raw.push(i1);
                        let mut i2 = Instr::new(Op::Jslt as u8, 'D');
                        i2.label = Some(lbl);
                        raw.push(i2);
                    }
                    "JGT" => {
                        let mut i1 = Instr::new(Op::Je as u8, 'D');
                        i1.imm = 4;
                        raw.push(i1);
                        let mut i2 = Instr::new(Op::Jsge as u8, 'D');
                        i2.label = Some(lbl);
                        raw.push(i2);
                    }
                    _ => unreachable!(),
                }
                continue;
            }

            let (op_byte, fmt) = match mnem.as_str() {
                "NOP" => (Op::Nop as u8, 'A'),
                "HALT" => (Op::Halt as u8, 'A'),
                "YIELD" => (Op::Yield as u8, 'A'),
                "DUP" => (Op::Dup as u8, 'A'),
                "RET" => (Op::Ret as u8, 'A'),
                "SYSCALL" => (Op::Syscall as u8, 'A'),
                "INC" => (Op::Inc as u8, 'B'),
                "DEC" => (Op::Dec as u8, 'B'),
                "PUSH" => (Op::Push as u8, 'B'),
                "POP" => (Op::Pop as u8, 'B'),
                "MOV" => (Op::Mov as u8, 'C'),
                "LOAD" => (Op::Load as u8, 'C'),
                "STORE" => (Op::Store as u8, 'C'),
                "NEG" | "INEG" => (Op::Ineg as u8, 'C'),
                "NOT" | "INOT" => (Op::Inot as u8, 'C'),
                "CMP" => (Op::Cmp as u8, 'C'),
                "JMP" => (Op::Jmp as u8, 'D'),
                "JZ" => (Op::Jz as u8, 'D'),
                "JNZ" => (Op::Jnz as u8, 'D'),
                "CALL" => (Op::Call as u8, 'D'),
                "MOVI" => (Op::Movi as u8, 'D'),
                "JE" => (Op::Je as u8, 'D'),
                "JNE" => (Op::Jne as u8, 'D'),
                "JSGE" => (Op::Jsge as u8, 'D'),
                "JSLT" => (Op::Jslt as u8, 'D'),
                "ADD" | "IADD" => (Op::Iadd as u8, 'E'),
                "SUB" | "ISUB" => (Op::Isub as u8, 'E'),
                "MUL" | "IMUL" => (Op::Imul as u8, 'E'),
                "DIV" | "IDIV" => (Op::Idiv as u8, 'E'),
                "MOD" | "IMOD" => (Op::Imod as u8, 'E'),
                "AND" | "IAND" => (Op::Iand as u8, 'E'),
                "OR" | "IOR" => (Op::Ior as u8, 'E'),
                "XOR" | "IXOR" => (Op::Ixor as u8, 'E'),
                "SHL" | "ISHL" => (Op::Ishl as u8, 'E'),
                "SHR" | "ISHR" => (Op::Ishr as u8, 'E'),
                _ => {
                    return Err(format!(
                        "Line {}: unknown instruction '{mnem}'",
                        line_num + 1
                    ))
                }
            };

            let mut instr = Instr::new(op_byte, fmt);

            match fmt {
                'A' => {}
                'B' => {
                    instr.rd = parse_reg(rest, line_num)?;
                }
                'C' => {
                    let ps: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
                    instr.rd = parse_reg(ps[0], line_num)?;
                    instr.rs = parse_reg(ps[1], line_num)?;
                }
                'D' => {
                    let ps: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
                    if ps.len() == 1 {
                        // Jump with label only (no register)
                        instr.label = Some(ps[0].to_string());
                    } else if ps.len() == 2 {
                        instr.rd = parse_reg(ps[0], line_num)?;
                        let v = ps[1];
                        if !v.is_empty()
                            && (v.as_bytes()[0].is_ascii_digit()
                                || (v.starts_with('-') && v.len() > 1))
                        {
                            instr.imm = v.parse::<i32>().map_err(|_| {
                                format!("Line {}: bad immediate '{v}'", line_num + 1)
                            })?;
                        } else {
                            instr.label = Some(v.to_string());
                        }
                    }
                }
                'E' => {
                    let ps: Vec<&str> = rest.split(',').map(|s| s.trim()).collect();
                    instr.rd = parse_reg(ps[0], line_num)?;
                    instr.rs = parse_reg(ps[1], line_num)?;
                    instr.rs2 = parse_reg(ps[2], line_num)?;
                }
                _ => {}
            }

            raw.push(instr);
        }

        // Compute byte offsets
        let mut offset = 0usize;
        for instr in &mut raw {
            instr.offset = offset;
            offset += instr.size;
        }

        // Build label → byte-offset map
        let mut label_bytes: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for (lbl, idx) in &labels {
            label_bytes.insert(
                lbl.clone(),
                if *idx < raw.len() {
                    raw[*idx].offset
                } else {
                    offset
                },
            );
        }

        // Emit bytecode
        let mut out = Vec::new();
        for instr in &raw {
            out.push(instr.op);
            match instr.fmt {
                'A' => {}
                'B' => out.push(instr.rd),
                'C' => {
                    out.push(instr.rd);
                    out.push(instr.rs);
                }
                'D' => {
                    out.push(instr.rd);
                    if let Some(ref lbl) = instr.label {
                        let target = *label_bytes
                            .get(lbl)
                            .ok_or_else(|| format!("Undefined label: '{lbl}'"))?;
                        let rel = target as i32 - (instr.offset + 4) as i32;
                        out.push((rel & 0xFF) as u8);
                        out.push(((rel >> 8) & 0xFF) as u8);
                    } else {
                        let imm = instr.imm;
                        out.push((imm & 0xFF) as u8);
                        out.push(((imm >> 8) & 0xFF) as u8);
                    }
                }
                'E' => {
                    out.push(instr.rd);
                    out.push(instr.rs);
                    out.push(instr.rs2);
                }
                _ => {}
            }
        }

        Ok(out)
    }

    fn strip_comments(line: &str) -> &str {
        // Find first ; or #
        let mut idx = line.len();
        for marker in [';', '#'] {
            if let Some(pos) = line.find(marker) {
                if pos < idx {
                    idx = pos;
                }
            }
        }
        &line[..idx]
    }

    fn parse_label<'a>(
        text: &'a str,
        labels: &mut Vec<(String, usize)>,
        raw: &[Instr],
        line_num: usize,
    ) -> Result<&'a str, String> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(text);
        }

        // Check if text starts with identifier:
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return Ok(text);
        }
        let first = bytes[0];
        if !(first.is_ascii_alphabetic() || first == b'_') {
            return Ok(text);
        }

        // Find ':' to identify a label
        if let Some(colon_pos) = text.find(':') {
            let label_part = &text[..colon_pos];
            // Verify label is a valid identifier
            let valid = label_part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_');
            if !valid {
                return Ok(text);
            }
            let label = label_part.to_string();
            // Check for duplicates
            if labels.iter().any(|(l, _)| l == &label) {
                return Err(format!("Duplicate label '{label}'"));
            }
            labels.push((label, raw.len()));
            let remaining = text[colon_pos + 1..].trim();
            Ok(remaining)
        } else {
            Ok(text)
        }
    }

    fn parse_reg(tok: &str, line_num: usize) -> Result<u8, String> {
        let tok = tok.trim().to_uppercase();
        let tb = tok.as_bytes();
        if tb.len() < 2 || tb[0] != b'R' || !tb[1..].iter().all(|b| b.is_ascii_digit()) {
            return Err(format!(
                "Line {}: expected register, got '{tok}'",
                line_num + 1
            ));
        }
        let n: u8 = tok[1..]
            .parse()
            .map_err(|_| format!("Line {}: register out of range", line_num + 1))?;
        if n as usize >= super::NUM_REGISTERS {
            return Err(format!("Line {}: R{n} out of range", line_num + 1));
        }
        Ok(n)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Policies
// ═══════════════════════════════════════════════════════════════════════════════

pub mod policies {
    use super::assemble;

    /// Enforce a maximum output length (approximate token count).
    ///
    /// `max_tokens` is the hard limit on the approximate token count of the
    /// output (`len/4`). The output is blocked when the token count exceeds
    /// `max_tokens`. Note that the value is encoded as a 16-bit FLUX
    /// immediate, so it must fit in `0..=65535`.
    pub fn length_budget_policy(max_tokens: i32) -> Vec<u8> {
        assemble(&format!(
            r#"
            MOVI R0, 5
            SYSCALL
            MOV  R2, R0
            MOVI R3, {max_tokens}
            JGT  R2, R3, block
            MOVI R0, 0
            HALT
block:
            MOVI R1, 1
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT
            "#
        ))
        .expect("length_budget_policy assembly")
    }

    /// Block outputs with excessive repetition. `max_ratio` is per-mille (300 = 30%).
    pub fn repetition_policy(max_ratio: i32) -> Vec<u8> {
        assemble(&format!(
            r#"
            MOVI R0, 6
            SYSCALL
            MOV  R2, R0
            MOVI R3, {max_ratio}
            JGT  R2, R3, block
            MOVI R0, 0
            HALT
block:
            MOVI R1, 2
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT
            "#
        ))
        .expect("repetition_policy assembly")
    }

    /// Block outputs that drift too far from input topic. `min_overlap` is per-mille.
    pub fn category_policy(min_overlap: i32) -> Vec<u8> {
        assemble(&format!(
            r#"
            MOVI R0, 7
            SYSCALL
            MOV  R2, R0
            MOVI R3, {min_overlap}
            JLT  R2, R3, block
            MOVI R0, 0
            HALT
block:
            MOVI R1, 3
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT
            "#
        ))
        .expect("category_policy assembly")
    }

    /// Block outputs with too-low Shannon entropy. `min_entropy` is entropy × 1000.
    pub fn entropy_policy(min_entropy: i32) -> Vec<u8> {
        assemble(&format!(
            r#"
            MOVI R0, 12
            SYSCALL
            MOV  R2, R0
            MOVI R3, {min_entropy}
            JLT  R2, R3, block
            MOVI R0, 0
            HALT
block:
            MOVI R1, 4
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT
            "#
        ))
        .expect("entropy_policy assembly")
    }

    /// Block outputs with low information density.
    /// `min_ratio` is unique_words / total_words × 1000 (400 = 40% unique minimum).
    pub fn information_density_policy(min_ratio: i32) -> Vec<u8> {
        assemble(&format!(
            r#"
            MOVI R0, 11
            SYSCALL
            MOV  R2, R0
            MOVI R3, {min_ratio}
            JLT  R2, R3, block
            MOVI R0, 0
            HALT
block:
            MOVI R1, 5
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT
            "#
        ))
        .expect("information_density_policy assembly")
    }

    /// Block outputs that drift outside the input's topic scope.
    pub fn scope_discipline_policy(min_overlap: i32, max_expansion: i32) -> Vec<u8> {
        // We need to generate repeated additions based on max_expansion
        let mut add_lines = String::new();
        if max_expansion >= 2 {
            add_lines.push_str("IADD R6, R4, R4\n"); // 2×
            for i in 3..=max_expansion {
                add_lines.push_str(&format!("IADD R6, R6, R4\n")); // i×
                let _ = i; // suppress unused warning
            }
        } else {
            // max_expansion = 1: just copy
            add_lines.push_str("MOV R6, R4\n");
        }

        assemble(&format!(
            r#"
            MOVI R0, 7
            SYSCALL
            MOV  R2, R0
            MOVI R3, {min_overlap}
            JLT  R2, R3, block

            MOVI R0, 1
            SYSCALL
            MOV  R4, R0
            MOVI R0, 2
            SYSCALL
            MOV  R5, R0

            MOVI R0, 0
            CMP  R4, R0
            JE   allow

            {add_lines}
            JGT  R5, R6, block

allow:
            MOVI R0, 0
            HALT

block:
            MOVI R1, 6
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT
            "#
        ))
        .expect("scope_discipline_policy assembly")
    }

    /// Enforce budget decay over time — each call consumes budget.
    pub fn budget_decay_policy(decay_rate: i32, min_threshold: i32, max_calls: i32) -> Vec<u8> {
        assemble(&format!(
            r#"
            MOVI R1, {decay_rate}
            MOVI R0, 14
            SYSCALL
            MOV  R2, R0

            MOVI R3, {min_threshold}
            JLT  R2, R3, exhausted

            MOVI R0, 13
            SYSCALL
            MOV  R4, R0
            MOVI R5, {max_calls}
            JGT  R4, R5, exhausted

            MOVI R0, 0
            HALT

exhausted:
            MOVI R1, 7
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT
            "#
        ))
        .expect("budget_decay_policy assembly")
    }

    /// Combined conservation policy: length + repetition + category + entropy + optional density + decay.
    pub fn combined_policy(
        max_tokens: i32,
        max_repetition: i32,
        min_overlap: i32,
        min_entropy: i32,
        min_density: i32,
        enable_decay: bool,
        decay_rate: i32,
    ) -> Vec<u8> {
        let density_check = if min_density > 0 {
            format!(
                r#"
            MOVI R0, 11
            SYSCALL
            MOV  R2, R0
            MOVI R3, {min_density}
            JLT  R2, R3, block_density
            "#
            )
        } else {
            "NOP\n".to_string()
        };

        let decay_check = if enable_decay {
            format!(
                r#"
            MOVI R1, {decay_rate}
            MOVI R0, 14
            SYSCALL
            MOV  R2, R0
            MOVI R3, 10
            JLT  R2, R3, block_decay
            "#
            )
        } else {
            "NOP\n".to_string()
        };

        assemble(&format!(
            r#"
            MOVI R0, 5
            SYSCALL
            MOV  R2, R0
            MOVI R3, {max_tokens}
            JGT  R2, R3, block_length

            MOVI R0, 6
            SYSCALL
            MOV  R2, R0
            MOVI R3, {max_repetition}
            JGT  R2, R3, block_repetition

            MOVI R0, 7
            SYSCALL
            MOV  R2, R0
            MOVI R3, {min_overlap}
            JLT  R2, R3, block_category

            MOVI R0, 12
            SYSCALL
            MOV  R2, R0
            MOVI R3, {min_entropy}
            JLT  R2, R3, block_entropy

            {density_check}
            {decay_check}

            MOVI R0, 0
            HALT

block_length:
            MOVI R1, 1
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT

block_repetition:
            MOVI R1, 2
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT

block_category:
            MOVI R1, 3
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT

block_entropy:
            MOVI R1, 4
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT

block_density:
            MOVI R1, 5
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT

block_decay:
            MOVI R1, 7
            MOVI R0, 8
            SYSCALL
            MOVI R0, 1
            HALT
            "#
        ))
        .expect("combined_policy assembly")
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Audit (feature-gated)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "audit")]
pub mod audit {
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    pub struct AuditLog {
        path: String,
    }

    impl AuditLog {
        pub fn new(path: &str) -> Self {
            let p = Path::new(path);
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            Self {
                path: path.to_string(),
            }
        }

        pub fn log(
            &self,
            input_text: &str,
            output_text: &str,
            allowed: bool,
            violation_reason: Option<&str>,
            violation_code: u32,
            cycles: u64,
            remaining_budget: i64,
            call_count: u64,
        ) {
            let input_hash = super::simple_hash(input_text);
            let output_hash = super::simple_hash(output_text);
            let timestamp = super::current_iso_timestamp();
            let violation = violation_reason
                .map(|r| format!("\"{}\"", r.replace('"', "\\\"")))
                .unwrap_or_else(|| "null".to_string());

            let line = format!(
                r#"{{"timestamp":"{timestamp}","input_hash":"{input_hash:016x}","output_hash":"{output_hash:016x}","allowed":{allowed},"violation":{violation},"violation_code":{violation_code},"cycles":{cycles},"remaining_budget":{remaining_budget},"call_count":{call_count}}}"#,
            );

            if let Ok(mut f) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(f, "{line}");
            }
        }

        pub fn read_all(&self) -> Vec<serde_lite::JsonValue> {
            // Lightweight: we don't depend on serde, so we parse manually
            // For now, return raw lines
            Vec::new()
        }

        pub fn summary(&self) -> AuditSummary {
            let mut total = 0u64;
            let mut blocked = 0u64;
            let mut total_cycles = 0u64;

            if let Ok(content) = fs::read_to_string(&self.path) {
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    total += 1;
                    if line.contains(r#""allowed":false"#) {
                        blocked += 1;
                    }
                    total_cycles += extract_u64(line, "cycles");
                }
            }

            AuditSummary {
                total_calls: total,
                total_blocked: blocked,
                block_rate: if total > 0 {
                    blocked as f64 / total as f64
                } else {
                    0.0
                },
                total_cycles,
                avg_cycles: if total > 0 {
                    total_cycles as f64 / total as f64
                } else {
                    0.0
                },
            }
        }

        pub fn clear(&self) {
            let _ = fs::write(&self.path, "");
        }
    }

    #[derive(Debug, Clone)]
    pub struct AuditSummary {
        pub total_calls: u64,
        pub total_blocked: u64,
        pub block_rate: f64,
        pub total_cycles: u64,
        pub avg_cycles: f64,
    }

    fn extract_u64(line: &str, key: &str) -> u64 {
        let pattern = format!(r#""{key}":"#);
        if let Some(pos) = line.find(&pattern) {
            let rest = &line[pos + pattern.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest[..end].parse().unwrap_or(0)
        } else {
            0
        }
    }

    // Minimal JSON value type placeholder
    mod serde_lite {
        pub enum JsonValue {}
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Metrics (feature-gated)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "metrics")]
pub mod metrics {
    use std::collections::HashMap;

    #[derive(Debug, Clone)]
    pub struct MetricsSnapshot {
        pub total_calls: u64,
        pub total_allowed: u64,
        pub total_blocked: u64,
        pub total_cycles: u64,
        pub total_budget_consumed: i64,
        pub policy_triggers: HashMap<String, u64>,
        pub avg_cycles: f64,
        pub block_rate: f64,
        pub avg_budget_per_call: f64,
    }

    pub struct MetricsCollector {
        total_calls: u64,
        total_allowed: u64,
        total_blocked: u64,
        total_cycles: u64,
        total_budget_consumed: i64,
        policy_triggers: HashMap<String, u64>,
    }

    impl MetricsCollector {
        pub fn new() -> Self {
            Self {
                total_calls: 0,
                total_allowed: 0,
                total_blocked: 0,
                total_cycles: 0,
                total_budget_consumed: 0,
                policy_triggers: HashMap::new(),
            }
        }

        pub fn record(
            &mut self,
            allowed: bool,
            violation_reason: Option<&str>,
            cycles: u64,
            budget_before: i64,
            budget_after: i64,
        ) {
            self.total_calls += 1;
            self.total_cycles += cycles;
            let consumed = budget_before - budget_after;
            if consumed > 0 {
                self.total_budget_consumed += consumed;
            }

            if allowed {
                self.total_allowed += 1;
            } else {
                self.total_blocked += 1;
                if let Some(reason) = violation_reason {
                    *self.policy_triggers.entry(reason.to_string()).or_insert(0) += 1;
                }
            }
        }

        pub fn snapshot(&self) -> MetricsSnapshot {
            let n = self.total_calls.max(1);
            MetricsSnapshot {
                total_calls: self.total_calls,
                total_allowed: self.total_allowed,
                total_blocked: self.total_blocked,
                total_cycles: self.total_cycles,
                total_budget_consumed: self.total_budget_consumed,
                policy_triggers: self.policy_triggers.clone(),
                avg_cycles: self.total_cycles as f64 / n as f64,
                block_rate: if self.total_calls > 0 {
                    self.total_blocked as f64 / self.total_calls as f64
                } else {
                    0.0
                },
                avg_budget_per_call: if self.total_calls > 0 {
                    self.total_budget_consumed as f64 / self.total_calls as f64
                } else {
                    0.0
                },
            }
        }

        pub fn reset(&mut self) {
            self.total_calls = 0;
            self.total_allowed = 0;
            self.total_blocked = 0;
            self.total_cycles = 0;
            self.total_budget_consumed = 0;
            self.policy_triggers.clear();
        }
    }

    impl Default for MetricsCollector {
        fn default() -> Self {
            Self::new()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Re-exports
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── VM arithmetic tests ──

    #[test]
    fn test_vm_add() {
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
    fn test_vm_sub() {
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
    fn test_vm_mul() {
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
    fn test_vm_div() {
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
    fn test_vm_div_by_zero() {
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
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::DivisionByZero));
    }

    #[test]
    fn test_vm_mod() {
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

    // ── Control flow tests ──

    #[test]
    fn test_vm_je_taken() {
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
    fn test_vm_jne_taken() {
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
    fn test_vm_jsge_greater() {
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
    fn test_vm_jsge_less_should_not_jump() {
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
    fn test_vm_jslt_less() {
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
    fn test_vm_jslt_greater_should_not_jump() {
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

    // ── Syscall tests ──

    #[test]
    fn test_syscall_input_len() {
        let code = vec![Op::Movi as u8, 0, 1, 0, Op::Syscall as u8, Op::Halt as u8];
        let mut vm = FluxVM::new();
        vm.load_input("hello world");
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 11);
    }

    #[test]
    fn test_syscall_output_len() {
        let code = vec![Op::Movi as u8, 0, 2, 0, Op::Syscall as u8, Op::Halt as u8];
        let mut vm = FluxVM::new();
        vm.load_output("test response");
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 13);
    }

    #[test]
    fn test_syscall_token_count() {
        let code = vec![Op::Movi as u8, 0, 5, 0, Op::Syscall as u8, Op::Halt as u8];
        let mut vm = FluxVM::new();
        vm.load_output(&"a".repeat(40));
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 10);
    }

    #[test]
    fn test_syscall_repetition() {
        let code = vec![Op::Movi as u8, 0, 6, 0, Op::Syscall as u8, Op::Halt as u8];
        let mut vm = FluxVM::new();
        vm.load_output("the the the the the");
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 1000);
    }

    #[test]
    fn test_syscall_get_budget() {
        let code = vec![Op::Movi as u8, 0, 10, 0, Op::Syscall as u8, Op::Halt as u8];
        let mut vm = FluxVM::new();
        vm.set_budget(750);
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 750);
    }

    #[test]
    fn test_syscall_unique_ratio() {
        let code = vec![Op::Movi as u8, 0, 11, 0, Op::Syscall as u8, Op::Halt as u8];
        let mut vm = FluxVM::new();
        vm.load_output("apple banana apple banana cherry");
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 600);
    }

    #[test]
    fn test_syscall_violation_flag() {
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

    // ── Stack tests ──

    #[test]
    fn test_push_pop() {
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
    fn test_inc_dec() {
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

    // ── Assembler tests ──

    #[test]
    fn test_assemble_simple_halt() {
        assert_eq!(assemble("HALT").unwrap(), vec![Op::Halt as u8]);
    }

    #[test]
    fn test_assemble_nop() {
        assert_eq!(assemble("NOP").unwrap(), vec![Op::Nop as u8]);
    }

    #[test]
    fn test_assemble_movi() {
        assert_eq!(
            assemble("MOVI R0, 42").unwrap(),
            vec![Op::Movi as u8, 0, 42, 0]
        );
    }

    #[test]
    fn test_assemble_add() {
        assert_eq!(
            assemble("IADD R2, R0, R1").unwrap(),
            vec![Op::Iadd as u8, 2, 0, 1]
        );
    }

    #[test]
    fn test_assemble_mov() {
        assert_eq!(assemble("MOV R0, R1").unwrap(), vec![Op::Mov as u8, 0, 1]);
    }

    #[test]
    fn test_assemble_inc() {
        assert_eq!(assemble("INC R5").unwrap(), vec![Op::Inc as u8, 5]);
    }

    #[test]
    fn test_assemble_cmp() {
        assert_eq!(assemble("CMP R0, R1").unwrap(), vec![Op::Cmp as u8, 0, 1]);
    }

    #[test]
    fn test_assemble_multiple() {
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
    fn test_assemble_comments() {
        let code = assemble("; comment\nMOVI R0, 5  # inline\nHALT").unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 5);
    }

    #[test]
    fn test_assemble_unknown_instruction() {
        assert!(assemble("BOGUS R0, R1").is_err());
    }

    #[test]
    fn test_assemble_undefined_label() {
        assert!(assemble("JE nowhere\nHALT").is_err());
    }

    // ── Label tests ──

    #[test]
    fn test_assemble_je_label() {
        let code = assemble("MOVI R0, 0\nCMP R0, R0\nJE done\nMOVI R0, 99\ndone:\nHALT").unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 0);
    }

    #[test]
    fn test_assemble_jmp_label() {
        let code = assemble("MOVI R0, 1\nJMP skip\nMOVI R0, 99\nskip:\nMOVI R0, 42\nHALT").unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(0), 42);
    }

    // ── Pseudo-jump tests ──

    #[test]
    fn test_jge_taken_greater() {
        let code = assemble(
            "MOVI R0, 10\nMOVI R1, 5\nJGE R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
        )
        .unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(2), 1);
    }

    #[test]
    fn test_jge_taken_equal() {
        let code = assemble(
            "MOVI R0, 5\nMOVI R1, 5\nJGE R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
        )
        .unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(2), 1);
    }

    #[test]
    fn test_jge_not_taken() {
        let code = assemble(
            "MOVI R0, 3\nMOVI R1, 5\nJGE R0, R1, hit\nMOVI R2, 99\nHALT\nhit:\nMOVI R2, 1\nHALT",
        )
        .unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(2), 99);
    }

    #[test]
    fn test_jlt_taken() {
        let code = assemble(
            "MOVI R0, 3\nMOVI R1, 5\nJLT R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
        )
        .unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(2), 1);
    }

    #[test]
    fn test_jgt_taken() {
        let code = assemble(
            "MOVI R0, 10\nMOVI R1, 5\nJGT R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
        )
        .unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(2), 1);
    }

    #[test]
    fn test_jgt_not_taken_equal() {
        let code = assemble(
            "MOVI R0, 5\nMOVI R1, 5\nJGT R0, R1, hit\nMOVI R2, 99\nHALT\nhit:\nMOVI R2, 1\nHALT",
        )
        .unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(2), 99);
    }

    #[test]
    fn test_jle_taken_equal() {
        let code = assemble(
            "MOVI R0, 5\nMOVI R1, 5\nJLE R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
        )
        .unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(2), 1);
    }

    #[test]
    fn test_jle_taken_less() {
        let code = assemble(
            "MOVI R0, 3\nMOVI R1, 5\nJLE R0, R1, hit\nMOVI R2, 0\nHALT\nhit:\nMOVI R2, 1\nHALT",
        )
        .unwrap();
        let mut vm = FluxVM::new();
        vm.run(&code).unwrap();
        assert_eq!(vm.regs.get(2), 1);
    }

    // ── Length budget policy tests ──

    #[test]
    fn test_length_budget_allows_short() {
        let mut e = ConservationEnforcer::new(policies::length_budget_policy(100), 100);
        let r = e.enforce("What is Python?", "Python is a programming language.");
        assert!(r.allowed);
    }

    #[test]
    fn test_length_budget_blocks_long() {
        let mut e = ConservationEnforcer::new(policies::length_budget_policy(5), 5);
        let r = e.enforce("Tell me everything", &"word ".repeat(100));
        assert!(!r.allowed);
        assert!(r.violation.unwrap().reason.contains("Length budget"));
    }

    #[test]
    fn test_correction_template() {
        let mut e = ConservationEnforcer::with_options(
            policies::length_budget_policy(3),
            3,
            Some("🚫 Blocked: {reason}"),
        );
        let r = e.enforce("Q", "This is a very long response that exceeds budget");
        assert!(!r.allowed);
        assert!(r.output.contains("🚫 Blocked:"));
    }

    // ── Repetition policy tests ──

    #[test]
    fn test_repetition_allows_diverse() {
        let mut e = ConservationEnforcer::new(policies::repetition_policy(500), 1000);
        let r = e.enforce("Explain photosynthesis",
            "Plants convert sunlight into chemical energy through photosynthesis using chlorophyll and water.");
        assert!(r.allowed);
    }

    #[test]
    fn test_repetition_blocks_repetitive() {
        let mut e = ConservationEnforcer::new(policies::repetition_policy(300), 1000);
        let r = e.enforce("Summarize", "the the the the the the the the the the");
        assert!(!r.allowed);
        assert!(r
            .violation
            .unwrap()
            .reason
            .to_lowercase()
            .contains("repetition"));
    }

    // ── Category policy tests ──

    #[test]
    fn test_category_allows_on_topic() {
        let mut e = ConservationEnforcer::new(policies::category_policy(50), 1000);
        let r = e.enforce(
            "Python programming language",
            "Python is a great programming language for beginners and experts alike",
        );
        assert!(r.allowed);
    }

    #[test]
    fn test_category_blocks_off_topic() {
        let mut e = ConservationEnforcer::new(policies::category_policy(900), 1000);
        let r = e.enforce(
            "quantum physics particles",
            "banana apple orange grape melon",
        );
        assert!(!r.allowed);
        assert!(r
            .violation
            .unwrap()
            .reason
            .to_lowercase()
            .contains("category"));
    }

    // ── Entropy policy tests ──

    #[test]
    fn test_entropy_allows_high() {
        let mut e = ConservationEnforcer::new(policies::entropy_policy(1000), 1000);
        let r = e.enforce(
            "List colors",
            "red blue green yellow orange purple cyan magenta",
        );
        assert!(r.allowed);
    }

    #[test]
    fn test_entropy_blocks_low() {
        let mut e = ConservationEnforcer::new(policies::entropy_policy(2500), 1000);
        let r = e.enforce("Write a poem", "go go go go go go go go go go");
        assert!(!r.allowed);
        assert!(r
            .violation
            .unwrap()
            .reason
            .to_lowercase()
            .contains("entropy"));
    }

    // ── Combined policy tests ──

    #[test]
    fn test_combined_allows_compliant() {
        let policy = policies::combined_policy(500, 500, 10, 500, 0, false, 0);
        let mut e = ConservationEnforcer::new(policy, 500);
        let r = e.enforce("What is machine learning?",
            "Machine learning is a subset of artificial intelligence that enables systems to learn from data.");
        assert!(r.allowed);
    }

    #[test]
    fn test_combined_blocks_on_length() {
        let policy = policies::combined_policy(3, 500, 0, 0, 0, false, 0);
        let mut e = ConservationEnforcer::new(policy, 3);
        let r = e.enforce(
            "Write a long essay about AI",
            &"Artificial intelligence is ".repeat(50),
        );
        assert!(!r.allowed);
        assert!(r.violation.unwrap().reason.contains("Length"));
    }

    #[test]
    fn test_combined_blocks_on_repetition() {
        let policy = policies::combined_policy(10000, 200, 0, 0, 0, false, 0);
        let mut e = ConservationEnforcer::new(policy, 10000);
        let r = e.enforce(
            "Describe a sunset",
            "beautiful beautiful beautiful beautiful beautiful beautiful beautiful",
        );
        assert!(!r.allowed);
        assert!(r
            .violation
            .unwrap()
            .reason
            .to_lowercase()
            .contains("repetition"));
    }

    // ── Enforcement result tests ──

    #[test]
    fn test_cycles_recorded() {
        let mut e = ConservationEnforcer::new(policies::length_budget_policy(10000), 10000);
        let r = e.enforce("Hi", "Hello!");
        assert!(r.cycles > 0);
    }

    // ── Custom policy tests ──

    #[test]
    fn test_always_allow() {
        let code = assemble("MOVI R0, 0\nHALT").unwrap();
        let mut e = ConservationEnforcer::new(code, 1000);
        let r = e.enforce("anything", "any response");
        assert!(r.allowed);
    }

    #[test]
    fn test_always_block() {
        let code = assemble("MOVI R1, 99\nMOVI R0, 8\nSYSCALL\nMOVI R0, 1\nHALT").unwrap();
        let mut e = ConservationEnforcer::new(code, 1000);
        let r = e.enforce("q", "a");
        assert!(!r.allowed);
        assert!(r.violation.unwrap().reason.contains("Custom"));
    }

    // ── Budget tracking tests ──

    #[test]
    fn test_remaining_budget() {
        let e = ConservationEnforcer::new(policies::length_budget_policy(10000), 500);
        assert_eq!(e.remaining_budget(), 500);
    }

    #[test]
    fn test_replenish_budget() {
        let mut e = ConservationEnforcer::new(policies::length_budget_policy(10000), 100);
        e.replenish_budget(50);
        assert_eq!(e.remaining_budget(), 150);
    }

    #[test]
    fn test_reset_budget() {
        let mut e = ConservationEnforcer::new(policies::length_budget_policy(10000), 200);
        e.replenish_budget(100);
        assert_eq!(e.remaining_budget(), 300);
        e.reset_budget();
        assert_eq!(e.remaining_budget(), 200);
    }

    // ── Call count tests ──

    #[test]
    fn test_call_count_increments() {
        let mut e = ConservationEnforcer::new(policies::length_budget_policy(10000), 10000);
        assert_eq!(e.call_count(), 0);
        e.enforce("q1", "response one");
        assert_eq!(e.call_count(), 1);
        e.enforce("q2", "response two");
        assert_eq!(e.call_count(), 2);
    }

    // ── Information density policy tests ──

    #[test]
    fn test_density_allows_high() {
        let mut e = ConservationEnforcer::new(policies::information_density_policy(300), 1000);
        let r = e.enforce(
            "List colors",
            "red blue green yellow orange purple cyan magenta violet turquoise",
        );
        assert!(r.allowed);
    }

    #[test]
    fn test_density_blocks_low() {
        let mut e = ConservationEnforcer::new(policies::information_density_policy(500), 1000);
        let r = e.enforce("Write a poem", "go go go go go go go go go go");
        assert!(!r.allowed);
        assert!(r
            .violation
            .unwrap()
            .reason
            .to_lowercase()
            .contains("density"));
    }

    #[test]
    fn test_density_boundary() {
        let mut e = ConservationEnforcer::new(policies::information_density_policy(500), 1000);
        // 1 unique out of 2 total = 500 per-mille, exactly at threshold → passes (JLT is strict)
        let r = e.enforce("test", "hello world");
        assert!(r.allowed);
    }

    // ── Scope discipline policy tests ──

    #[test]
    fn test_scope_allows_on_topic() {
        let mut e = ConservationEnforcer::new(policies::scope_discipline_policy(50, 10), 1000);
        let r = e.enforce(
            "Python programming language tutorial",
            "Python is a great programming language for beginners",
        );
        assert!(r.allowed);
    }

    #[test]
    fn test_scope_blocks_off_topic() {
        let mut e = ConservationEnforcer::new(policies::scope_discipline_policy(500, 10), 1000);
        let r = e.enforce(
            "quantum physics particles energy",
            "banana apple orange grape melon fruit",
        );
        assert!(!r.allowed);
        assert!(r.violation.unwrap().reason.to_lowercase().contains("scope"));
    }

    #[test]
    fn test_scope_blocks_excessive_expansion() {
        let mut e = ConservationEnforcer::new(policies::scope_discipline_policy(0, 10), 1000);
        let r = e.enforce("hi", &"hello ".repeat(100));
        assert!(!r.allowed);
        assert!(r.violation.unwrap().reason.to_lowercase().contains("scope"));
    }

    #[test]
    fn test_scope_empty_input() {
        let mut e = ConservationEnforcer::new(policies::scope_discipline_policy(0, 10), 1000);
        let r = e.enforce("", "any output here");
        assert!(r.allowed);
    }

    // ── Budget decay policy tests ──

    #[test]
    fn test_decay_allows_with_budget() {
        let mut e = ConservationEnforcer::new(policies::budget_decay_policy(10, 5, 100), 1000);
        let r = e.enforce("question", "answer response");
        assert!(r.allowed);
    }

    #[test]
    fn test_decay_blocks_when_exhausted() {
        let mut e = ConservationEnforcer::new(policies::budget_decay_policy(100, 50, 100), 100);
        let r = e.enforce("question", "answer response");
        assert!(!r.allowed);
        let reason = r.violation.unwrap().reason.to_lowercase();
        assert!(reason.contains("budget") || reason.contains("cooldown"));
    }

    #[test]
    fn test_decay_decreases_across_calls() {
        let mut e = ConservationEnforcer::new(policies::budget_decay_policy(50, 5, 100), 500);
        assert_eq!(e.remaining_budget(), 500);
        e.enforce("q1", "response one here");
        assert_eq!(e.remaining_budget(), 450);
        e.enforce("q2", "response two here");
        assert_eq!(e.remaining_budget(), 400);
    }

    #[test]
    fn test_decay_blocks_max_calls() {
        let mut e = ConservationEnforcer::new(policies::budget_decay_policy(1, 0, 3), 10000);
        e.enforce("q", "a response");
        e.enforce("q", "a response");
        e.enforce("q", "a response");
        let r4 = e.enforce("q", "a response");
        assert!(!r4.allowed);
    }

    // ── Combined with new policies ──

    #[test]
    fn test_combined_with_density() {
        let policy = policies::combined_policy(10000, 500, 0, 0, 300, false, 0);
        let mut e = ConservationEnforcer::new(policy, 10000);
        let r = e.enforce("Write something", "blah blah blah blah blah blah blah");
        assert!(!r.allowed);
    }

    #[test]
    fn test_combined_with_decay() {
        let policy = policies::combined_policy(10000, 500, 0, 0, 0, true, 100);
        let mut e = ConservationEnforcer::new(policy, 200);
        e.enforce("q", "a reasonable response here");
        let r2 = e.enforce("q", "a reasonable response here");
        assert!(!r2.allowed);
    }

    // ── enforce_with_llm ──

    #[test]
    fn test_enforce_with_llm() {
        let mut e = ConservationEnforcer::new(policies::length_budget_policy(100), 100);
        let r = e.enforce_with_llm("Hello", |p| format!("Response to: {p}"));
        assert!(r.allowed);
        assert!(r.output.contains("Response to: Hello"));
    }
}
