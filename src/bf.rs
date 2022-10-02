use std::io::{Read, Write};
use crate::{CpuConfig, TrackKind};

pub enum BfOp {
    Left,
    Right,
    Inc,
    Dec,
    In,
    Out,
    Loop(Vec<BfOp>),
    DebugMessage(String),
    Crash(String),
    Breakpoint
}

#[derive(Debug, Copy, Clone)]
pub struct TextPos {
    line_num: usize,
    col: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct UnbalancedOpenBracket {
    pos: TextPos,
}

#[derive(Debug, Copy, Clone)]
pub struct UnbalancedCloseBracket {
    pos: TextPos,
}

#[derive(Debug)]
pub enum ParseBfProgError {
    UnbalancedOpenBracket(UnbalancedOpenBracket),
    UnbalancedCloseBracket(UnbalancedCloseBracket),
}

pub fn parse_bf(s: &str) -> Result<Vec<BfOp>, ParseBfProgError> {
    struct StackFrame {
        open_bracket_pos: TextPos,
        ops: Vec<BfOp>,
    }

    let mut stack = vec![StackFrame {
        open_bracket_pos: TextPos {
            line_num: 0,
            col: 0,
        },
        ops: vec![],
    }];
    for (line_num, line) in s.lines().enumerate() {
        for (col, c) in line.chars().enumerate() {
            let pos = TextPos {
                line_num: line_num + 1,
                col: col + 1,
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
                    ops: vec![],
                });
            } else if c == ']' {
                if stack.len() <= 1 {
                    return Err(ParseBfProgError::UnbalancedCloseBracket(
                        UnbalancedCloseBracket { pos },
                    ));
                } else {
                    let top = stack.pop().unwrap();
                    stack.last_mut().unwrap().ops.push(BfOp::Loop(top.ops));
                }
            }
        }
    }

    if stack.len() > 1 {
        Err(ParseBfProgError::UnbalancedOpenBracket(
            UnbalancedOpenBracket {
                pos: stack.last().unwrap().open_bracket_pos,
            },
        ))
    } else {
        Ok(stack.pop().unwrap().ops)
    }
}

pub struct BfState {
    cells: Vec<u8>,
    cell_ptr: usize,
}

#[derive(Debug)]
pub enum RunOpError {
    PtrOutOfBounds,
    ReaderErr(std::io::Error),
    WriterErr(std::io::Error),
    Crashed
}

impl BfState {
    pub fn new() -> BfState {
        BfState {
            cells: vec![0; 1],
            cell_ptr: 0,
        }
    }

    pub fn run_op(
        &mut self,
        op: &BfOp,
        reader: &mut impl Read,
        writer: &mut impl Write,
        cpu_config: Option<&CpuConfig>
    ) -> Result<(), RunOpError> {
        match op {
            BfOp::Left => {
                if self.cell_ptr == 0 {
                    return Err(RunOpError::PtrOutOfBounds);
                } else {
                    self.cell_ptr -= 1;
                }
            }
            BfOp::Right => {
                self.cell_ptr += 1;
                if self.cell_ptr >= self.cells.len() {
                    self.cells.push(0);
                }
            }
            BfOp::Inc => {
                self.cells[self.cell_ptr] = self.cells[self.cell_ptr].wrapping_add(1);
            }
            BfOp::Dec => {
                self.cells[self.cell_ptr] = self.cells[self.cell_ptr].wrapping_sub(1);
            }
            BfOp::In => {
                let mut buf: [u8; 1] = [0; 1];
                match reader.read_exact(&mut buf) {
                    Ok(()) => {
                        // simply ignore \r
                        let c = buf[0];
                        if c != 13 {
                            self.cells[self.cell_ptr] = c;
                        }
                    }
                    Err(e) => match e.kind() {
                        std::io::ErrorKind::UnexpectedEof => {
                            self.cells[self.cell_ptr] = 0;
                        }
                        _ => {
                            return Err(RunOpError::ReaderErr(e));
                        }
                    },
                }
            }
            BfOp::Out => {
                let byte = self.cells[self.cell_ptr];
                let buf: [u8; 1] = [byte];
                match writer.write_all(&buf) {
                    Ok(()) => {}
                    Err(e) => {
                        return Err(RunOpError::WriterErr(e));
                    }
                }
                match writer.flush() {
                    Ok(()) => {}
                    Err(e) => {
                        return Err(RunOpError::WriterErr(e));
                    }
                }
            }
            BfOp::Loop(ops) => {
                while self.cells[self.cell_ptr] != 0 {
                    self.run_ops(ops, reader, writer, cpu_config)?;
                }
            }
            BfOp::DebugMessage(msg) => {
                println!("{}", msg);
            }
            BfOp::Crash(msg) => {
                println!("{}", msg);
                return Err(RunOpError::Crashed)
            }
            BfOp::Breakpoint => {
                if let Some(cfg) = cpu_config {
                    self.print_state(cfg);
                }
            }
        }
        Ok(())
    }

    pub fn run_ops(
        &mut self,
        ops: &[BfOp],
        reader: &mut impl Read,
        writer: &mut impl Write,
        cpu_config: Option<&CpuConfig>
    ) -> Result<(), RunOpError> {
        for op in ops {
            self.run_op(op, reader, writer, cpu_config)?;
        }
        Ok(())
    }

    pub fn print_tape(&self) {
        for cell in &self.cells {
            print!("{}, ", cell);
        }
    }

    pub fn print_state(&self, cpu: &CpuConfig) {
        let num_digits = |x: u8| {
            x.to_string().chars().count()
        };
        println!("CPU STATE:");
        let tracks = cpu.get_tracks();
        let num_tracks = tracks.len();
        for (id, track) in tracks {
            println!("Track {:?}:", id);
            match track {
                TrackKind::Data(track) => {
                    let mut i = track.track_num as usize;
                    let mut caret_i = 0;
                    let mut print_caret_at = None;
                    while i < self.cells.len() {
                        if i == self.cell_ptr {
                            print_caret_at = Some(caret_i);
                        }
                        caret_i += num_digits(self.cells[i])+2;
                        print!("{}, ", self.cells[i]);
                        i += num_tracks;
                    }
                    println!();
                    if let Some(print_caret_at) = print_caret_at {
                        println!("{}^", std::iter::repeat(" ").take(print_caret_at).collect::<String>());
                    }
                },
                TrackKind::Scratch(track) => {
                    let mut i = track.track.track_num as usize;
                    let mut caret_i = 0;
                    let mut print_caret_at = None;
                    while i < self.cells.len() {
                        if i == self.cell_ptr {
                            print_caret_at = Some(caret_i);
                        }
                        caret_i += num_digits(self.cells[i])+2;
                        print!("{}, ", self.cells[i]);
                        i += num_tracks;
                    }
                    println!();
                    if let Some(print_caret_at) = print_caret_at {
                        println!("{}^", std::iter::repeat(" ").take(print_caret_at).collect::<String>());
                    }
                },
                _ => {
                    println!("Unknown type!");
                }
            }
        }
    }
}

pub fn ops2str(ops: &Vec<BfOp>) -> String {
    fn rec(ops: &Vec<BfOp>, result: &mut String) {
        for op in ops {
            match op {
                BfOp::Left => {
                    *result += "<";
                }
                BfOp::Right => {
                    *result += ">";
                }
                BfOp::Inc => {
                    *result += "+";
                }
                BfOp::Dec => {
                    *result += "-";
                }
                BfOp::In => {
                    *result += ",";
                }
                BfOp::Out => {
                    *result += ".";
                }
                BfOp::Loop(ops) => {
                    *result += "[";
                    rec(ops, result);
                    *result += "]";
                }
                BfOp::DebugMessage(_msg) => {
                    *result += "#";
                }
                BfOp::Crash(_msg) => {
                    *result += "!";
                }
                BfOp::Breakpoint => {
                    *result += "$";
                }
            }
        }
    }

    let mut result = String::new();
    rec(ops, &mut result);
    result
}
