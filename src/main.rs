mod hir;
mod cpu;
mod bf;
mod lir2bf;

extern crate nom;
extern crate num_bigint;
extern crate num_traits;

use std::fmt::Debug;
use crate::bf::*;
use crate::hir::*;
use crate::cpu::*;
use crate::lir2bf::*;
use std::collections::HashMap;

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
    state.run_ops(&prog, &mut std::io::stdin(), &mut std::io::stdout()).unwrap_or_else(print_err);
}

#[allow(unused)]
fn mainb() {
    println!("{:?}", parse_hir("48 + 37 * 4"));
    println!("{:?}", parse_hir("test(foo(), 7)"));
}

fn main() {
    let mut tracks = HashMap::new();
    tracks.insert(TrackId::Heap, TrackKind::Data( Track { track_num: 0 }));
    tracks.insert(TrackId::Scratch1, TrackKind::Scratch( ScratchTrack { track: Track { track_num: 1 }}));
    tracks.insert(TrackId::Scratch2, TrackKind::Scratch( ScratchTrack { track: Track { track_num: 2 }}));
    let mut cpu = Cpu::new(&mut tracks);
    cpu.add_const_to_byte(Pos { track: 0, frame: 0 }, 14);
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_prog(prog: &str, i: &str, o: &str) {
        let prog = parse_bf(prog).unwrap_or_else(print_err);
        let mut state = BfState::new();
        let mut r = i.as_bytes();
        let mut w = Vec::new();
        state.run_ops(&prog, &mut r, &mut w).unwrap_or_else(print_err);
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
            "8 bit cells"
        );
    }
}