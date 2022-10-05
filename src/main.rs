#![allow(dead_code, unused_imports)]
#![allow(
    clippy::comparison_chain,
    clippy::len_zero,
    clippy::redundant_closure,
    clippy::collapsible_if,
    clippy::collapsible_else_if
)]

mod bf;
mod cpu;
mod hir;
mod hir2sam;
mod linker;
mod lir2bf;
mod sam;
mod sam2lir;

extern crate nom;
extern crate num;

use std::fmt::Debug;

use crate::bf::*;
use crate::cpu::*;
use crate::hir::*;
use crate::hir2sam::*;
use crate::linker::*;
use crate::lir2bf::*;
use crate::sam::*;
use crate::sam2lir::*;

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
    println!("{}", ops2str(&ops, false));
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

    cpu.add_const_to_register(register, 103050u64, scratch);
    cpu.add_const_to_register(register, 20406u64, scratch);

    cpu.moveprint_byte(register.at(0), scratch);
    cpu.print_text(", ", scratch);
    cpu.moveprint_byte(register.at(1), scratch);
    cpu.print_text(", ", scratch);
    cpu.moveprint_byte(register.at(2), scratch);
    cpu.print_text(", ", scratch);
    cpu.moveprint_byte(register.at(3), scratch);

    let ops = lir2bf(&cpu.into_ops());
    println!("{}", ops2str(&ops, false));
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

    cpu.set_register(register, 123456u64);
    cpu.moveprint_register_hex(register, scratch);

    let ops = lir2bf(&cpu.into_ops());
    println!("{}", ops2str(&ops, false));
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
fn mainf() {
    let mut cfg = CpuConfig::new();
    let mut register_builder = cfg.build_register_track(TrackId::Register1);
    let a = register_builder.add_binregister(32);
    let scratch = cfg.add_scratch_track(TrackId::Scratch1);
    let mut cpu = Cpu::new(&cfg);

    cpu.set_binregister(a, 1234567u64, scratch);
    cpu.print_binregister_in_decimal(a, scratch);

    let ops = lir2bf(&cpu.into_ops());
    let opt_ops = get_optimized_bf_ops(&ops);
    println!("{}", ops2str(&opt_ops, true));
    println!("Num instrs: {}", ops2str(&ops, false).chars().count());
    let mut state = BfState::new();
    let result = state.run_ops(
        &opt_ops,
        &mut std::io::stdin(),
        &mut std::io::stdout(),
        Some(&cfg),
    );
    println!();
    match result {
        Ok(()) => {
            println!("Ran successfully");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    state.print_state(&cfg);
    println!("Instrs executed: {}", state.get_instrs_executed());
}

#[allow(unused)]
fn main() {
    let hir = parse_hir(
        "fn main() {
            println(fib(3));
        }
        
        fn fib(x: u8) -> u8 {
            if x {
                let x_minus_1 : u8 = x - 1;
                if x_minus_1 {
                    let x_minus_2 : u8 = x_minus_1 - 1;
                    let f1 : u8 = fib(x_minus_1);
                    let f2 : u8 = fib(x_minus_2);
                    f1 + f2
                } else {
                    1
                }
            } else {
                1
            }
        }",
    )
    .unwrap();

    /*let fibcode = std::fs::read_to_string("progs/fib.bfrs").expect("failed to read bfrs code");
    let hir = parse_hir(&fibcode).unwrap();*/

    let sam = hir2sam(&hir);
    println!("{:?}", sam);

    let linked = link_sam_fns(sam);
    println!("{:?}", linked);

    let (ops, cfg) = sam2lir(linked);
    let ops = lir2bf(&ops);
    let opt_ops = get_optimized_bf_ops(&ops);
    println!("{}", ops2str(&opt_ops, true));
    println!("Num instrs: {}", ops2str(&ops, false).chars().count());
    
    let mut state = BfState::new();
    let result = state.run_ops(
        &opt_ops,
        &mut std::io::stdin(),
        &mut std::io::stdout(),
        Some(&cfg),
    );
    println!();
    match result {
        Ok(()) => {
            println!("Ran successfully");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    state.print_state(&cfg);
    println!("Num instrs: {}", ops2str(&ops, false).chars().count());
    println!("Instrs executed: {}", state.get_instrs_executed());
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_parsed_bf_prog(prog: &Vec<BfOp>, i: &str, o: &str, cfg: Option<&CpuConfig>) {
        let prog = get_optimized_bf_ops(prog);
        let mut state = BfState::new();
        let mut r = i.as_bytes();
        let mut w = Vec::new();
        state
            .run_ops(&prog, &mut r, &mut w, cfg)
            .unwrap_or_else(print_err);
        assert_eq!(w, o.as_bytes());
        if let Some(cfg) = cfg {
            state.check_scratch_is_empty(cfg);
        }
    }

    fn test_raw_bf_prog(prog: &str, i: &str, o: &str) {
        test_parsed_bf_prog(&parse_bf(prog).unwrap_or_else(print_err), i, o, None);
    }

    fn test_lir_prog(prog: &Vec<Lir>, i: &str, o: &str, cfg: &CpuConfig) {
        test_parsed_bf_prog(&lir2bf(prog), i, o, Some(cfg));
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

        cpu.add_const_to_register(register, 103050u64, scratch);
        cpu.add_const_to_register(register, 20406u64, scratch);

        cpu.moveprint_byte(register.at(0), scratch);
        cpu.print_text(", ", scratch);
        cpu.moveprint_byte(register.at(1), scratch);
        cpu.print_text(", ", scratch);
        cpu.moveprint_byte(register.at(2), scratch);
        cpu.print_text(", ", scratch);
        cpu.moveprint_byte(register.at(3), scratch);

        test_lir_prog(&cpu.into_ops(), "", "0, 1, 226, 64", &cfg);
    }

    #[test]
    fn test_print_register_hex() {
        let mut cfg = CpuConfig::new();
        let register = cfg.add_register_track(TrackId::Register1, 4);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_register(register, 123456u64);
        cpu.moveprint_register_hex(register, scratch);

        test_lir_prog(&cpu.into_ops(), "", "0x0001E240", &cfg);
    }

    #[test]
    fn test_unpack_and_print_register() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let register = register_builder.add_register(4);
        let binregister = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.add_const_to_register(register, 0b111111111101010101010101u64, scratch);
        cpu.unpack_register(register, binregister, scratch, false);
        cpu.print_binregister_in_binary(binregister, scratch);

        test_lir_prog(
            &cpu.into_ops(),
            "",
            "0b00000000111111111101010101010101",
            &cfg,
        );
    }

    #[test]
    fn test_ifzero_binregister() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let binregister = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(binregister, 0b1000000000000000000000u64, scratch);
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

        test_lir_prog(&cpu.into_ops(), "", "11", &cfg);
    }

    #[test]
    fn test_add_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let reg1 = register_builder.add_binregister(32);
        let reg2 = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(reg1, 789742058u64, scratch);
        cpu.set_binregister(reg2, 391490498u64, scratch);
        cpu.add_binregister_to_binregister(reg1, reg2, scratch);
        cpu.print_binregister_in_binary(reg2, scratch);

        test_lir_prog(
            &cpu.into_ops(),
            "",
            "0b01000110011010000010110110101100",
            &cfg,
        );
    }

    #[test]
    fn test_sub_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let reg1 = register_builder.add_binregister(32);
        let reg2 = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(reg1, 289742058u64, scratch);
        cpu.set_binregister(reg2, 791490498u64, scratch);
        cpu.sub_binregister_from_binregister(reg1, reg2, scratch);
        cpu.print_binregister_in_binary(reg2, scratch);

        test_lir_prog(
            &cpu.into_ops(),
            "",
            "0b00011101111010000001001011011000",
            &cfg,
        );
    }

    #[test]
    fn test_shift_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let reg1 = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(reg1, 0b01000110011010000010110110101100u64, scratch);
        cpu.shift_binregister_left(reg1, scratch);
        cpu.print_binregister_in_binary(reg1, scratch);

        test_lir_prog(
            &cpu.into_ops(),
            "",
            "0b10001100110100000101101101011000",
            &cfg,
        );
    }

    #[test]
    fn test_shift_binregisters_right() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let reg1 = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(reg1, 0b01000110011010000010110110101101u64, scratch);
        cpu.shift_binregister_right(reg1, scratch);
        cpu.print_binregister_in_binary(reg1, scratch);

        test_lir_prog(
            &cpu.into_ops(),
            "",
            "0b00100011001101000001011011010110",
            &cfg,
        );
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

        cpu.set_binregister(reg1, 103050u64, scratch);
        cpu.set_binregister(reg2, 1561594u64, scratch);
        cpu.mul_binregisters(reg1, reg2, reg3, scratch);
        cpu.print_binregister_in_binary(reg3, scratch);

        test_lir_prog(
            &cpu.into_ops(),
            "",
            "0b01110111101101101101100011000100",
            &cfg,
        );
    }

    #[test]
    fn test_div_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let a = register_builder.add_binregister(32);
        let b = register_builder.add_binregister(32);
        let div = register_builder.add_binregister(32);
        let rem = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(a, 1037250132u64, scratch);
        cpu.set_binregister(b, 156347u64, scratch);
        cpu.div_binregisters(a, b, div, rem, scratch);
        cpu.print_binregister_in_binary(div, scratch);
        cpu.print_newline(scratch);
        cpu.print_binregister_in_binary(rem, scratch);

        test_lir_prog(
            &cpu.into_ops(),
            "",
            "0b00000000000000000001100111101010\n0b00000000000000001010110001100110",
            &cfg,
        );
    }

    #[test]
    fn test_div_binregisters_10() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let a = register_builder.add_binregister(32);
        let b = register_builder.add_binregister(4);
        let div = register_builder.add_binregister(32);
        let rem = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(a, 1037250132u64, scratch);
        cpu.set_binregister(b, 10u64, scratch);
        cpu.div_binregisters(a, b, div, rem, scratch);
        cpu.print_binregister_in_binary(div, scratch);
        cpu.print_newline(scratch);
        cpu.print_binregister_in_binary(rem, scratch);

        test_lir_prog(
            &cpu.into_ops(),
            "",
            "0b00000110001011101011011111010101\n0b00000000000000000000000000000010",
            &cfg,
        );
    }

    #[test]
    fn test_print_binregister_decimal() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let a = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);

        cpu.set_binregister(a, 1037250132u64, scratch);
        cpu.print_binregister_in_decimal(a, scratch);

        test_lir_prog(&cpu.into_ops(), "", "1037250132", &cfg);
    }

    #[test]
    fn test_cmp_2_binregisters() {
        let mut cfg = CpuConfig::new();
        let mut register_builder = cfg.build_register_track(TrackId::Register1);
        let a = register_builder.add_binregister(32);
        let b = register_builder.add_binregister(32);
        let scratch = cfg.add_scratch_track(TrackId::Scratch1);
        let mut cpu = Cpu::new(&cfg);
        let (cmp_result, scratch) = scratch.split_1();
        cpu.add_const_to_byte(cmp_result, b'5');

        cpu.set_binregister(a, 136u64, scratch);
        cpu.set_binregister(b, 138u64, scratch);
        cpu.cmp_2_uint_binregisters(a, b, cmp_result, scratch);
        cpu.goto(cmp_result);
        cpu.out();
        cpu.inc_binregister(a, scratch);
        cpu.cmp_2_uint_binregisters(a, b, cmp_result, scratch);
        cpu.goto(cmp_result);
        cpu.out();
        cpu.inc_binregister(a, scratch);
        cpu.cmp_2_uint_binregisters(a, b, cmp_result, scratch);
        cpu.goto(cmp_result);
        cpu.out();
        cpu.inc_binregister(a, scratch);
        cpu.cmp_2_uint_binregisters(a, b, cmp_result, scratch);
        cpu.goto(cmp_result);
        cpu.out();
        cpu.inc_binregister(a, scratch);
        cpu.cmp_2_uint_binregisters(a, b, cmp_result, scratch);
        cpu.goto(cmp_result);
        cpu.out();
        cpu.clr();

        test_lir_prog(&cpu.into_ops(), "", "43345", &cfg);
    }

    #[test]
    fn test_full_fib() {
        let hir = parse_hir(
            "fn main() {
                println(fib(2));
            }
            
            fn fib(x: u8) -> u8 {
                if x {
                    let x_minus_1 : u8 = x - 1;
                    if x_minus_1 {
                        let x_minus_2 : u8 = x_minus_1 - 1;
                        let f1 : u8 = fib(x_minus_1);
                        let f2 : u8 = fib(x_minus_2);
                        f1 + f2
                    } else {
                        1
                    }
                } else {
                    1
                }
            }",
        )
        .unwrap();

        let sam = hir2sam(&hir);

        let linked = link_sam_fns(sam);

        let (ops, cfg) = sam2lir(linked);

        test_lir_prog(&ops, "", "2\n", &cfg);
    }
}
