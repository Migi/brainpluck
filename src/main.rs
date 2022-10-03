#![allow(dead_code, unused_imports)]

mod bf;
mod cpu;
mod hir;
mod hir2sam;
mod linker;
mod lir2bf;
mod sam;

extern crate nom;
extern crate num;

use num::BigUint;
use std::fmt::Debug;

use crate::bf::*;
use crate::cpu::*;
use crate::hir::*;
use crate::hir2sam::*;
use crate::linker::*;
use crate::lir2bf::*;
use crate::sam::*;

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
        .run_ops(&prog, &mut std::io::stdin(), &mut std::io::stdout(), None)
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

    cpu.set_byte(data.at(0), 234);
    cpu.moveprint_byte(data.at(0), scratch);

    let ops = lir2bf(&cpu.into_ops());
    println!("{}", ops2str(&ops));
    let mut state = BfState::new();
    state
        .run_ops(
            &ops,
            &mut std::io::stdin(),
            &mut std::io::stdout(),
            Some(&cfg),
        )
        .unwrap_or_else(print_err);
}

#[allow(unused)]
fn maind() {
    let mut cfg = CpuConfig::new();
    let register = cfg.add_register_track(TrackId::Register1, 4);
    let scratch = cfg.add_scratch_track(TrackId::Scratch1);
    let mut cpu = Cpu::new(&cfg);

    cpu.add_const_to_register(register, BigUint::from(103050u64), scratch);
    cpu.add_const_to_register(register, BigUint::from(20406u64), scratch);

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
        .run_ops(
            &ops,
            &mut std::io::stdin(),
            &mut std::io::stdout(),
            Some(&cfg),
        )
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
        .run_ops(
            &ops,
            &mut std::io::stdin(),
            &mut std::io::stdout(),
            Some(&cfg),
        )
        .unwrap_or_else(print_err);

    // should print 0x0001E240
}

#[allow(unused)]
fn main() {
    let mut cfg = CpuConfig::new();
    let mut register_builder = cfg.build_register_track(TrackId::Register1);
    let reg1 = register_builder.add_binregister(32);
    let reg2 = register_builder.add_binregister(32);
    let reg3 = register_builder.add_binregister(32);
    let scratch = cfg.add_scratch_track(TrackId::Scratch1);
    let mut cpu = Cpu::new(&cfg);

    //cpu.add_const_to_register(register, BigUint::from(0b10101u64), scratch);
    //cpu.unpack_register_onto_zeros(register, binregister, scratch);
    cpu.set_binregister(reg1, BigUint::from(103050u64), scratch);
    cpu.set_binregister(reg2, BigUint::from(1561594u64), scratch);
    cpu.mul_binregisters(reg1, reg2, reg3, scratch);
    cpu.print_binregister_in_binary(reg3, scratch);
    cpu.print_newline(scratch);
    cpu.print_text("0b01110111101101101101100011000100", scratch);

    let ops = lir2bf(&cpu.into_ops());
    println!("{}", ops2str(&ops));
    let mut state = BfState::new();
    let result = state.run_ops(
        &ops,
        &mut std::io::stdin(),
        &mut std::io::stdout(),
        Some(&cfg),
    );
    println!("");
    match result {
        Ok(()) => {
            println!("Ran successfully");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    state.print_state(&cfg);
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_parsed_bf_prog(prog: &Vec<BfOp>, i: &str, o: &str) {
        let mut state = BfState::new();
        let mut r = i.as_bytes();
        let mut w = Vec::new();
        state
            .run_ops(&prog, &mut r, &mut w, None)
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
        cpu.add_const_to_register(register, BigUint::from(20406u64), scratch);

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

    #[test]
    fn test_unpack_and_print_register() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let register = register_builder.add_register(4);
        let binregister = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.add_const_to_register(
            register,
            BigUint::from(0b111111111101010101010101u64),
            scratch,
        );
        cpu.unpack_register_onto_zeros(register, binregister, scratch);
        cpu.print_binregister_in_binary(binregister, scratch);

        test_lir_prog(&cpu.into_ops(), "", "0b00000000111111111101010101010101");
    }

    #[test]
    fn test_ifzero_binregister() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let binregister = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(
            binregister,
            BigUint::from(0b1000000000000000000000u64),
            scratch,
        );
        cpu.if_binregister_nonzero_else(
            binregister,
            scratch,
            |cpu, scratch| {
                cpu.breakpoint();
                cpu.print_text("1", scratch);
            },
            |cpu, _| {
                cpu.crash("oh no");
            },
        );
        cpu.clr_binregister(binregister, scratch);
        cpu.if_binregister_nonzero_else(
            binregister,
            scratch,
            |cpu, _| {
                cpu.crash("oh no");
            },
            |cpu, scratch| {
                cpu.print_text("1", scratch);
            },
        );

        test_lir_prog(&cpu.into_ops(), "", "11");
    }

    #[test]
    fn test_add_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let reg1 = register_builder.add_binregister(32);
        let reg2 = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(reg1, BigUint::from(789742058u64), scratch);
        cpu.set_binregister(reg2, BigUint::from(391490498u64), scratch);
        cpu.add_binregister_to_binregister(reg1, reg2, scratch);
        cpu.print_binregister_in_binary(reg2, scratch);

        test_lir_prog(&cpu.into_ops(), "", "0b01000110011010000010110110101100");
    }

    #[test]
    fn test_sub_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let reg1 = register_builder.add_binregister(32);
        let reg2 = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(reg1, BigUint::from(289742058u64), scratch);
        cpu.set_binregister(reg2, BigUint::from(791490498u64), scratch);
        cpu.sub_binregister_from_binregister(reg1, reg2, scratch);
        cpu.print_binregister_in_binary(reg2, scratch);

        test_lir_prog(&cpu.into_ops(), "", "0b00011101111010000001001011011000");
    }

    #[test]
    fn test_shift_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let reg1 = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(
            reg1,
            BigUint::from(0b01000110011010000010110110101100u64),
            scratch,
        );
        cpu.shift_binregister_left(reg1, scratch);
        cpu.print_binregister_in_binary(reg1, scratch);

        test_lir_prog(&cpu.into_ops(), "", "0b10001100110100000101101101011000");
    }

    #[test]
    fn test_mul_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let reg1 = register_builder.add_binregister(32);
        let reg2 = register_builder.add_binregister(32);
        let reg3 = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(reg1, BigUint::from(103050u64), scratch);
        cpu.set_binregister(reg2, BigUint::from(1561594u64), scratch);
        cpu.mul_binregisters(reg1, reg2, reg3, scratch);
        cpu.print_binregister_in_binary(reg3, scratch);

        test_lir_prog(&cpu.into_ops(), "", "0b01110111101101101101100011000100");
    }
}
