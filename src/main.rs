#![allow(dead_code, unused_imports)]

mod bf;
mod cpu;
mod hir;
mod lir2bf;
mod sam;
mod hir2sam;
mod linker;

extern crate nom;
extern crate num;

use num::BigUint;
use std::fmt::Debug;

use crate::bf::*;
use crate::cpu::*;
use crate::hir::*;
use crate::lir2bf::*;
use crate::sam::*;
use crate::hir2sam::*;
use crate::linker::*;

fn print_err<T>(e: impl Debug) -> T {
    panic!("Error: {:?}", e)
}

#[allow(unused)]
fn maina() {
    //let prog = parse_bf_prog("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.").unwrap_or_else(print_err);
    //let prog = parse_bf_prog(">++++++++[-<+++++++++>]<.>>+>-[+]++>++>+++[>[->+++<<+++>]<<]>-----.>->+++..+++.>-.<<+[>[+>+]>>]<--------------.>>.+++.------.--------.>+.>+.").unwrap_or_else(print_err);
    //let prog = parse_bf_prog("++++++++[>++++++++<-]>[<++++>-]+<[>-<[>++++<-]>[<++++++++>-]<[>++++++++<-]+>[>++++++++++[>+++++<-]>+.-.[-]<<[-]<->] <[>>+++++++[>+++++++<-]>.+++++.[-]<<<-]] >[>++++++++[>+++++++<-]>.[-]<<-]<+++++++++++[>+++>+++++++++>+++++++++>+<<<<-]>-.>-.+++++++.+++++++++++.<.>>.++.+++++++..<-.>>-[[-]<]").unwrap_or_else(print_err);
    let contents = std::fs::read_to_string("progs/LostKng.b").expect("failed to read bf code");
    let prog = parse_bf(&contents).unwrap_or_else(print_err);
    let mut state = BfState::new();
    state
        .run_ops(&prog, &mut std::io::stdin(), &mut std::io::stdout())
        .unwrap_or_else(print_err);
}

#[allow(unused)]
fn mainb() {
    let fibcode = std::fs::read_to_string("progs/fib.bfrs").expect("failed to read bfrs code");

    //let hir = parse_hir("fn main() { let a : u32 = 7; let b : u32 = foo(); let c : u32 = 88; println(b); } fn foo() -> u32 { let a : u32 = 9; let b: u32 = 17; b }").unwrap();
    //let hir = parse_hir("fn main() { let a : u32 = 7; let b : u32 = if 9 { a } else { 9 }; print(b); }").unwrap();
    let hir = parse_hir(&fibcode).expect("Failed to parse");
    //println!("{:?}", hir);
    let sam = hir2sam(&hir);
    println!("{:?}", sam);

    let linked = link_sam_fns(sam);
    println!("{:?}", linked);

    {
        let mut samstate = SamState::new(linked);
        //println!("{:?}", samstate);

        while !samstate.halted {
            //println!("{:?}", samstate.decode_next_op());
            samstate.step(&mut std::io::stdin(), &mut std::io::stdout());
            //println!("{:?}", samstate);
        }
    }

    println!("Done.");
}

#[allow(unused)]
fn mainc() {
    let mut cfg = CpuConfig::new();
    let data = cfg.add_data_track(TrackId::Heap);
    let scratch = cfg.add_scratch_track(TrackId::Scratch1);
    let mut cpu = Cpu::new(&cfg);

    cpu.add_const_to_byte(data.at(0), 234);
    cpu.moveprint_byte(data.at(0), scratch);

    let ops = lir2bf(&cpu.into_ops());
    println!("{}", ops2str(&ops));
    let mut state = BfState::new();
    state
        .run_ops(&ops, &mut std::io::stdin(), &mut std::io::stdout())
        .unwrap_or_else(print_err);
}

#[allow(unused)]
fn main() {
    let mut cfg = CpuConfig::new();
    let register = cfg.add_register_track(TrackId::Register1, 4);
    let scratch = cfg.add_scratch_track(TrackId::Scratch1);
    let mut cpu = Cpu::new(&cfg);

    cpu.add_const_to_register(register, BigUint::from(103050u64), scratch);
    cpu.add_const_to_register(register, BigUint::from( 20406u64), scratch);
    
    cpu.moveprint_byte(register.at(0), scratch);
    cpu.print_text(", ", scratch);
    cpu.moveprint_byte(register.at(1), scratch);
    cpu.print_text(", ", scratch);
    cpu.moveprint_byte(register.at(2), scratch);
    cpu.print_text(", ", scratch);
    cpu.moveprint_byte(register.at(3), scratch);

    let ops = lir2bf(&cpu.into_ops());
    println!("{}", ops2str(&ops));
    let mut state = BfState::new();
    state
        .run_ops(&ops, &mut std::io::stdin(), &mut std::io::stdout())
        .unwrap_or_else(print_err);
}

#[allow(unused)]
fn maine() {
    let mut cfg = CpuConfig::new();
    let register = cfg.add_register_track(TrackId::Register1, 4);
    let scratch = cfg.add_scratch_track(TrackId::Scratch1);
    let mut cpu = Cpu::new(&cfg);

    cpu.set_register(register, BigUint::from(123456u64));
    cpu.moveprint_register_hex(register, scratch);

    let ops = lir2bf(&cpu.into_ops());
    println!("{}", ops2str(&ops));
    let mut state = BfState::new();
    state
        .run_ops(&ops, &mut std::io::stdin(), &mut std::io::stdout())
        .unwrap_or_else(print_err);

    // should print 0x0001E240
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_parsed_bf_prog(prog: &Vec<BfOp>, i: &str, o: &str) {
        let mut state = BfState::new();
        let mut r = i.as_bytes();
        let mut w = Vec::new();
        state
            .run_ops(&prog, &mut r, &mut w)
            .unwrap_or_else(print_err);
        assert_eq!(w, o.as_bytes());
    }

    fn test_raw_bf_prog(prog: &str, i: &str, o: &str) {
        test_parsed_bf_prog(&parse_bf(prog).unwrap_or_else(print_err), i, o);
    }

    fn test_lir_prog(prog: &Vec<Lir>, i: &str, o: &str) {
        test_parsed_bf_prog(&lir2bf(prog), i, o);
    }

    #[test]
    fn test_hello_world_1() {
        test_raw_bf_prog("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.", "", "Hello World!\n");
    }

    #[test]
    fn test_hello_world_2() {
        test_raw_bf_prog(">++++++++[-<+++++++++>]<.>>+>-[+]++>++>+++[>[->+++<<+++>]<<]>-----.>->+++..+++.>-.<<+[>[+>+]>>]<--------------.>>.+++.------.--------.>+.>+.", "", "Hello World!\n");
    }

    #[test]
    fn test_cell_size_check() {
        test_raw_bf_prog(
            "++++++++[>++++++++<-]>[<++++>-]+<[>-<[>++++<-]>[<++++++++>-]<
            [>++++++++<-]+>[>++++++++++[>+++++<-]>+.-.[-]<<[-]<->] <[>>++
            +++++[>+++++++<-]>.+++++.[-]<<<-]] >[>++++++++[>+++++++<-]>.[
            -]<<-]<+++++++++++[>+++>+++++++++>+++++++++>+<<<<-]>-.>-.++++
            +++.+++++++++++.<.>>.++.+++++++..<-.>>-[[-]<]",
            "",
            "8 bit cells",
        );
    }

    #[test]
    fn test_add_const_to_register() {
        let mut cfg = CpuConfig::new();
        let register = cfg.add_register_track(TrackId::Register1, 4);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.add_const_to_register(register, BigUint::from(103050u64), scratch);
        cpu.add_const_to_register(register, BigUint::from( 20406u64), scratch);

        cpu.moveprint_byte(register.at(0), scratch);
        cpu.print_text(", ", scratch);
        cpu.moveprint_byte(register.at(1), scratch);
        cpu.print_text(", ", scratch);
        cpu.moveprint_byte(register.at(2), scratch);
        cpu.print_text(", ", scratch);
        cpu.moveprint_byte(register.at(3), scratch);

        test_lir_prog(&cpu.into_ops(), "", "0, 1, 226, 64");
    }

    #[test]
    fn test_print_register_hex() {
        let mut cfg = CpuConfig::new();
        let register = cfg.add_register_track(TrackId::Register1, 4);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_register(register, BigUint::from(123456u64));
        cpu.moveprint_register_hex(register, scratch);

        test_lir_prog(&cpu.into_ops(), "", "0x0001E240");
    }
}
