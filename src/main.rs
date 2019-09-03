mod bf;
mod cpu;
mod hir;
mod lir2bf;

extern crate nom;
extern crate num;

use num::BigUint;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::bf::*;
use crate::cpu::*;
use crate::hir::*;
use crate::lir2bf::*;

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
    println!("{:?}", parse_hir("48 + 37 * 4"));
    println!("{:?}", parse_hir("test(foo(), 7)"));
}

#[allow(unused)]
fn mainc() {
    let mut cfg = CpuConfig::new();
    let data = cfg.add_data_track(TrackId::Heap);
    let scratch1 = cfg.add_scratch_track(TrackId::Scratch1);
    let scratch2 = cfg.add_scratch_track(TrackId::Scratch2);
    let mut cpu = Cpu::new(&cfg);

    cpu.add_const_to_byte(data.at(0), 234);
    cpu.moveprint_byte(data.at(0), scratch1, scratch2);

    let ops = lir2bf(cpu.into_ops());
    println!("{}", ops2str(&ops));
    let mut state = BfState::new();
    state
        .run_ops(&ops, &mut std::io::stdin(), &mut std::io::stdout())
        .unwrap_or_else(print_err);
}

#[allow(unused)]
fn maind() {
    let mut cfg = CpuConfig::new();
    let register = cfg.add_register_track(TrackId::Register1, 4);
    let scratch1 = cfg.add_scratch_track(TrackId::Scratch1);
    let scratch2 = cfg.add_scratch_track(TrackId::Scratch2);
    let mut cpu = Cpu::new(&cfg);

    cpu.add_const_to_register(register, BigUint::from(123456u64));
    cpu.moveprint_byte(register.at(0), scratch1, scratch2);
    cpu.print_text(", ", scratch1);
    cpu.moveprint_byte(register.at(1), scratch1, scratch2);
    cpu.print_text(", ", scratch1);
    cpu.moveprint_byte(register.at(2), scratch1, scratch2);
    cpu.print_text(", ", scratch1);
    cpu.moveprint_byte(register.at(3), scratch1, scratch2);

    let ops = lir2bf(cpu.into_ops());
    println!("{}", ops2str(&ops));
    let mut state = BfState::new();
    state
        .run_ops(&ops, &mut std::io::stdin(), &mut std::io::stdout())
        .unwrap_or_else(print_err);
}

fn main() {
    let mut cfg = CpuConfig::new();
    let register = cfg.add_register_track(TrackId::Register1, 4);
    let scratch1 = cfg.add_scratch_track(TrackId::Scratch1);
    let scratch2 = cfg.add_scratch_track(TrackId::Scratch2);
    let scratch3 = cfg.add_scratch_track(TrackId::Scratch3);
    let mut cpu = Cpu::new(&cfg);

    cpu.add_const_to_register(register, BigUint::from(123456u64));
    cpu.moveprint_register_hex(register, scratch1, scratch2, scratch3);

    let ops = lir2bf(cpu.into_ops());
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

    fn test_prog(prog: &str, i: &str, o: &str) {
        let prog = parse_bf(prog).unwrap_or_else(print_err);
        let mut state = BfState::new();
        let mut r = i.as_bytes();
        let mut w = Vec::new();
        state
            .run_ops(&prog, &mut r, &mut w)
            .unwrap_or_else(print_err);
        assert_eq!(w, o.as_bytes());
    }

    #[test]
    fn test_hello_world_1() {
        test_prog("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.", "", "Hello World!\n");
    }

    #[test]
    fn test_hello_world_2() {
        test_prog(">++++++++[-<+++++++++>]<.>>+>-[+]++>++>+++[>[->+++<<+++>]<<]>-----.>->+++..+++.>-.<<+[>[+>+]>>]<--------------.>>.+++.------.--------.>+.>+.", "", "Hello World!\n");
    }

    #[test]
    fn test_cell_size_check() {
        test_prog(
            "++++++++[>++++++++<-]>[<++++>-]+<[>-<[>++++<-]>[<++++++++>-]<
            [>++++++++<-]+>[>++++++++++[>+++++<-]>+.-.[-]<<[-]<->] <[>>++
            +++++[>+++++++<-]>.+++++.[-]<<<-]] >[>++++++++[>+++++++<-]>.[
            -]<<-]<+++++++++++[>+++>+++++++++>+++++++++>+<<<<-]>-.>-.++++
            +++.+++++++++++.<.>>.++.+++++++..<-.>>-[[-]<]",
            "",
            "8 bit cells",
        );
    }
}
