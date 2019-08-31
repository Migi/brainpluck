/*enum BfToken {
    Left,
    Right,
    Inc,
    Dec,
    Out,
    In,
    OpenBracket,
    CloseBracket
}

struct OpenBracketInstruction {
    close_bracket_pos: usize
}

struct CloseBracketInstruction {
    open_bracket_pos: usize
}

enum Instruction {
    Left,
    Right,
    Inc,
    Dec,
    Out,
    In,
    OpenBracket(OpenBracketInstruction),
    CloseBracket(CloseBracketInstruction)
}*/

use std::io::{Read, Write};
use std::fmt::Debug;

enum BfOp {
    Left,
    Right,
    Inc,
    Dec,
    In,
    Out,
    Loop(Vec<BfOp>)
}

#[derive(Debug,Copy,Clone)]
struct TextPos {
    line_num: usize,
    col: usize,
}

#[derive(Debug,Copy,Clone)]
struct UnbalancedOpenBracket {
    pos: TextPos
}

#[derive(Debug,Copy,Clone)]
struct UnbalancedCloseBracket {
    pos: TextPos
}

#[derive(Debug)]
enum ParseBfProgError {
    UnbalancedOpenBracket(UnbalancedOpenBracket),
    UnbalancedCloseBracket(UnbalancedCloseBracket),
}

fn parse_bf_prog(s: &str) -> Result<Vec<BfOp>, ParseBfProgError> {
    struct StackFrame {
        open_bracket_pos: TextPos,
        ops: Vec<BfOp>
    }

    let mut stack = vec![StackFrame {
        open_bracket_pos: TextPos {
            line_num: 0,
            col: 0
        },
        ops: vec![]
    }];
    for (line_num, line) in s.lines().enumerate() {
        for (col, c) in line.chars().enumerate() {
            let pos = TextPos {
                line_num: line_num+1,
                col: col+1
            };
            if c == '<' {
                stack.last_mut().unwrap().ops.push(BfOp::Left);
            } else if c == '>' {
                stack.last_mut().unwrap().ops.push(BfOp::Right);
            } else if c == '+' {
                stack.last_mut().unwrap().ops.push(BfOp::Inc);
            } else if c == '-' {
                stack.last_mut().unwrap().ops.push(BfOp::Dec);
            } else if c == ',' {
                stack.last_mut().unwrap().ops.push(BfOp::In);
            } else if c == '.' {
                stack.last_mut().unwrap().ops.push(BfOp::Out);
            } else if c == '[' {
                stack.push(StackFrame {
                    open_bracket_pos: pos,
                    ops: vec![]
                });
            } else if c == ']' {
                if stack.len() <= 1 {
                    return Err(ParseBfProgError::UnbalancedCloseBracket(UnbalancedCloseBracket {
                        pos
                    }))
                } else {
                    let top = stack.pop().unwrap();
                    stack.last_mut().unwrap().ops.push(BfOp::Loop(top.ops));
                }
            }
        }
    }

    if stack.len() > 1 {
        Err(ParseBfProgError::UnbalancedOpenBracket(UnbalancedOpenBracket {
            pos: stack.last().unwrap().open_bracket_pos
        }))
    } else {
        Ok(stack.pop().unwrap().ops)
    }
}

struct BfState {
    cells: Vec<u8>,
    cell_ptr: usize
}

#[derive(Debug)]
enum RunOpError {
    PtrOutOfBounds,
    ReaderErr(std::io::Error),
    WriterErr(std::io::Error),
}

impl BfState {
    fn new() -> BfState {
        BfState {
            cells: vec![0; 1000],
            cell_ptr: 0
        }
    }

    fn run_op(&mut self, op: &BfOp, reader: &mut impl Read, writer: &mut impl Write) -> Result<(), RunOpError> {
        match op {
            BfOp::Left => {
                if self.cell_ptr == 0 {
                    return Err(RunOpError::PtrOutOfBounds);
                } else {
                    self.cell_ptr -= 1;
                }
            },
            BfOp::Right => {
                self.cell_ptr += 1;
                if self.cell_ptr >= self.cells.len() {
                    self.cells.push(0);
                }
            },
            BfOp::Inc => {
                self.cells[self.cell_ptr] = self.cells[self.cell_ptr].wrapping_add(1);
            },
            BfOp::Dec => {
                self.cells[self.cell_ptr] = self.cells[self.cell_ptr].wrapping_sub(1);
            },
            BfOp::In => {
                let mut buf : [u8;1] = [0;1];
                match reader.read_exact(&mut buf) {
                    Ok(()) => {
                        // simply ignore \r
                        let c = buf[0];
                        if c != 13 {
                            self.cells[self.cell_ptr] = c;
                        }
                    },
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::UnexpectedEof => {
                            self.cells[self.cell_ptr] = 0;
                        },
                        _ => {
                            return Err(RunOpError::ReaderErr(e));
                        }
                    }
                }
            },
            BfOp::Out => {
                let byte = self.cells[self.cell_ptr];
                let buf : [u8;1] = [byte];
                match writer.write_all(&buf) {
                    Ok(()) => {},
                    Err(e) => {
                        return Err(RunOpError::WriterErr(e));
                    }
                }
                match writer.flush() {
                    Ok(()) => {},
                    Err(e) => {
                        return Err(RunOpError::WriterErr(e));
                    }
                }
            },
            BfOp::Loop(ops) => {
                while self.cells[self.cell_ptr] != 0 {
                    self.run_ops(ops, reader, writer)?;
                }
            }
        }
        Ok(())
    }
    
    fn run_ops(&mut self, ops: &[BfOp], reader: &mut impl Read, writer: &mut impl Write) -> Result<(), RunOpError> {
        for op in ops {
            self.run_op(op, reader, writer)?;
        }
        Ok(())
    }
}

fn print_err<T>(e: impl Debug) -> T {
    panic!("Error: {:?}", e)
}

fn main() {
    //let prog = parse_bf_prog("++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.").unwrap_or_else(print_err);
    //let prog = parse_bf_prog(">++++++++[-<+++++++++>]<.>>+>-[+]++>++>+++[>[->+++<<+++>]<<]>-----.>->+++..+++.>-.<<+[>[+>+]>>]<--------------.>>.+++.------.--------.>+.>+.").unwrap_or_else(print_err);
    //let prog = parse_bf_prog("++++++++[>++++++++<-]>[<++++>-]+<[>-<[>++++<-]>[<++++++++>-]<[>++++++++<-]+>[>++++++++++[>+++++<-]>+.-.[-]<<[-]<->] <[>>+++++++[>+++++++<-]>.+++++.[-]<<<-]] >[>++++++++[>+++++++<-]>.[-]<<-]<+++++++++++[>+++>+++++++++>+++++++++>+<<<<-]>-.>-.+++++++.+++++++++++.<.>>.++.+++++++..<-.>>-[[-]<]").unwrap_or_else(print_err);
    let contents = std::fs::read_to_string("progs/LostKng.b").expect("failed to read bf code");
    let prog = parse_bf_prog(&contents).unwrap_or_else(print_err);
    let mut state = BfState::new();
    state.run_ops(&prog, &mut std::io::stdin(), &mut std::io::stdout()).unwrap_or_else(print_err);
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_prog(prog: &str, i: &str, o: &str) {
        let prog = parse_bf_prog(prog).unwrap_or_else(print_err);
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