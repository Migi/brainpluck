use crate::{CpuConfig, TrackKind};
use std::cell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::{Read, Write};

#[derive(Clone)]
pub enum BfOp {
    Left,
    Right,
    Inc,
    Dec,
    In,
    Out,
    Loop(Vec<BfOp>),
    Clr,
    Shift(i16),
    Add(u8),
    MoveAdd(i16),
    MoveAdd2(i16, i16),
    Comment(String),
    DebugMessage(String),
    Crash(String),
    Breakpoint,
    PrintRegisters,
    CheckScratchIsEmptyFromHere(String),
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

struct ShiftAdd {
    shift: i16,
    add: u8,
}

fn get_loop_as_shiftadds(ops: &Vec<BfOp>) -> Option<HashMap<i16, u8>> {
    let mut shift_adds: HashMap<i16, u8> = HashMap::new();
    let mut cur_shift = 0;
    let mut encounter_add = |x: u8, shift: i16| match shift_adds.entry(shift) {
        Entry::Occupied(e) => {
            let new_add = e.get().wrapping_add(x);
            *e.into_mut() = new_add;
        }
        Entry::Vacant(e) => {
            e.insert(x);
        }
    };
    for op in ops {
        match op {
            BfOp::Left => {
                cur_shift -= 1;
            }
            BfOp::Right => {
                cur_shift += 1;
            }
            BfOp::Inc => {
                encounter_add(1, cur_shift);
            }
            BfOp::Dec => {
                encounter_add(255, cur_shift);
            }
            BfOp::Shift(shift) => {
                cur_shift += *shift;
            }
            BfOp::Add(val) => {
                encounter_add(*val, cur_shift);
            }
            _ => {
                return None;
            }
        }
    }
    if cur_shift == 0 {
        Some(shift_adds)
    } else {
        None
    }
}

pub fn get_optimized_bf_ops(ops: &Vec<BfOp>) -> Vec<BfOp> {
    let mut result = Vec::new();
    struct BufferState {
        cur_shift: i16,
        cur_add: u8,
    }
    impl BufferState {
        fn flush_shift(&mut self, result: &mut Vec<BfOp>) {
            if self.cur_shift == 1 {
                result.push(BfOp::Right);
            } else if self.cur_shift == -1 {
                result.push(BfOp::Left);
            } else if self.cur_shift != 0 {
                result.push(BfOp::Shift(self.cur_shift));
            }
            self.cur_shift = 0;
        }

        fn flush_add(&mut self, result: &mut Vec<BfOp>) {
            if self.cur_add == 1 {
                result.push(BfOp::Inc);
            } else if self.cur_add == 255 {
                result.push(BfOp::Dec);
            } else if self.cur_add != 0 {
                result.push(BfOp::Add(self.cur_add));
            }
            self.cur_add = 0;
        }

        fn flush_all(&mut self, result: &mut Vec<BfOp>) {
            assert!(!(self.cur_shift != 0 && self.cur_add != 0));
            self.flush_shift(result);
            self.flush_add(result);
        }
    }
    let mut buffer = BufferState {
        cur_shift: 0,
        cur_add: 0,
    };
    for op in ops {
        match op {
            BfOp::Left => {
                buffer.flush_add(&mut result);
                buffer.cur_shift -= 1;
            }
            BfOp::Right => {
                buffer.flush_add(&mut result);
                buffer.cur_shift += 1;
            }
            BfOp::Inc => {
                buffer.flush_shift(&mut result);
                buffer.cur_add = buffer.cur_add.wrapping_add(1);
            }
            BfOp::Dec => {
                buffer.flush_shift(&mut result);
                buffer.cur_add = buffer.cur_add.wrapping_sub(1);
            }
            BfOp::Shift(shift) => {
                buffer.flush_add(&mut result);
                buffer.cur_shift += *shift;
            }
            BfOp::Add(val) => {
                buffer.flush_shift(&mut result);
                buffer.cur_add = buffer.cur_add.wrapping_add(*val);
            }
            BfOp::Loop(ops) => {
                buffer.flush_all(&mut result);
                let mut created_output = false;
                if let Some(shift_adds) = get_loop_as_shiftadds(ops) {
                    if let Some(255) = shift_adds.get(&0) {
                        if shift_adds.len() == 1 {
                            result.push(BfOp::Clr);
                            created_output = true;
                        } else if shift_adds.len() == 2 {
                            for (shift, add) in shift_adds {
                                if shift != 0 {
                                    if add == 1 {
                                        assert!(!created_output);
                                        result.push(BfOp::MoveAdd(shift));
                                        created_output = true;
                                    }
                                }
                            }
                        } else if shift_adds.len() == 3 {
                            let mut shift1 = None;
                            let mut shift2 = None;
                            for (shift, add) in shift_adds {
                                if shift != 0 {
                                    if add == 1 {
                                        if shift1.is_none() {
                                            shift1 = Some(shift);
                                        } else {
                                            assert!(shift2.is_none());
                                            shift2 = Some(shift);
                                        }
                                    }
                                }
                            }
                            if let Some(shift1) = shift1 {
                                if let Some(shift2) = shift2 {
                                    result.push(BfOp::MoveAdd2(shift1, shift2));
                                    created_output = true;
                                }
                            }
                        }
                    }
                }
                if !created_output {
                    result.push(BfOp::Loop(get_optimized_bf_ops(ops)));
                }
            }
            other => {
                buffer.flush_all(&mut result);
                result.push(other.clone());
            }
        }
    }
    buffer.flush_all(&mut result);
    result
}

pub struct BfState {
    cells: Vec<u8>,
    cell_ptr: usize,
    instrs_executed: u64,
}

#[derive(Debug)]
pub enum RunOpError {
    PtrOutOfBounds,
    ReaderErr(std::io::Error),
    WriterErr(std::io::Error),
    Crashed,
    Other(String),
}

impl BfState {
    pub fn new() -> BfState {
        BfState {
            cells: vec![0; 1],
            cell_ptr: 0,
            instrs_executed: 0,
        }
    }

    pub fn get_instrs_executed(&self) -> u64 {
        self.instrs_executed
    }

    fn get_valid_ptr(&mut self, shift: i16) -> Result<usize, RunOpError> {
        let new_ptr = self.cell_ptr as isize + shift as isize;
        if new_ptr < 0 {
            Err(RunOpError::PtrOutOfBounds)
        } else {
            let result = new_ptr as usize;
            if self.cells.len() <= result {
                self.cells.resize(result + 1, 0);
            }
            Ok(result)
        }
    }

    pub fn run_op(
        &mut self,
        op: &BfOp,
        reader: &mut impl Read,
        writer: &mut impl Write,
        cpu_config: Option<&CpuConfig>,
    ) -> Result<(), RunOpError> {
        self.instrs_executed += 1;
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
            BfOp::Clr => {
                self.cells[self.cell_ptr] = 0;
            }
            BfOp::Shift(shift) => {
                self.cell_ptr = self.get_valid_ptr(*shift)?;
            }
            BfOp::Add(val) => {
                self.cells[self.cell_ptr] = self.cells[self.cell_ptr].wrapping_add(*val);
            }
            BfOp::MoveAdd(shift) => {
                let other_ptr = self.get_valid_ptr(*shift)?;
                self.cells[other_ptr] =
                    self.cells[other_ptr].wrapping_add(self.cells[self.cell_ptr]);
                self.cells[self.cell_ptr] = 0;
            }
            BfOp::MoveAdd2(shift1, shift2) => {
                let other_ptr = self.get_valid_ptr(*shift1)?;
                self.cells[other_ptr] =
                    self.cells[other_ptr].wrapping_add(self.cells[self.cell_ptr]);
                let other_ptr = self.get_valid_ptr(*shift2)?;
                self.cells[other_ptr] =
                    self.cells[other_ptr].wrapping_add(self.cells[self.cell_ptr]);
                self.cells[self.cell_ptr] = 0;
            }
            BfOp::Comment(_) => {}
            BfOp::DebugMessage(msg) => {
                println!("{}", msg);
            }
            BfOp::Crash(msg) => {
                println!("{}", msg);
                return Err(RunOpError::Crashed);
            }
            BfOp::Breakpoint => {
                if let Some(cfg) = cpu_config {
                    self.print_state(cfg);
                }
            }
            BfOp::PrintRegisters => {
                if let Some(cfg) = cpu_config {
                    self.print_registers(cfg);
                }
            }
            BfOp::CheckScratchIsEmptyFromHere(msg) => {
                if let Some(cfg) = cpu_config {
                    let num_tracks = cfg.get_tracks().len();
                    let mut i = self.cell_ptr;
                    while i < self.cells.len() {
                        if self.cells[i] != 0 {
                            return Err(RunOpError::Other(format!(
                                "CheckScratchIsEmptyFromHere: Not empty at index {}, value {}. Message: {}",
                                i, self.cells[i], msg
                            )));
                        }
                        i += num_tracks;
                    }
                } else {
                    panic!("Called CheckScratchIsEmptyFromHere without cpu config!");
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
        cpu_config: Option<&CpuConfig>,
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
        let num_digits = |x: u8| x.to_string().chars().count();
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
                        caret_i += num_digits(self.cells[i]) + 2;
                        print!("{}, ", self.cells[i]);
                        i += num_tracks;
                    }
                    println!();
                    if let Some(print_caret_at) = print_caret_at {
                        println!("{}^", " ".repeat(print_caret_at));
                    }
                }
                TrackKind::Scratch(track) => {
                    let mut i = track.track.track_num as usize;
                    let mut caret_i = 0;
                    let mut print_caret_at = None;
                    while i < self.cells.len() {
                        if i == self.cell_ptr {
                            print_caret_at = Some(caret_i);
                        }
                        caret_i += num_digits(self.cells[i]) + 2;
                        print!("{}, ", self.cells[i]);
                        i += num_tracks;
                    }
                    println!();
                    if let Some(print_caret_at) = print_caret_at {
                        println!("{}^", " ".repeat(print_caret_at));
                    }
                }
                TrackKind::MultipleRegisters(track_num, _, _) => {
                    let mut i = *track_num as usize;
                    let mut caret_i = 0;
                    let mut print_caret_at = None;
                    while i < self.cells.len() {
                        if i == self.cell_ptr {
                            print_caret_at = Some(caret_i);
                        }
                        caret_i += num_digits(self.cells[i]) + 2;
                        print!("{}, ", self.cells[i]);
                        i += num_tracks;
                    }
                    println!();
                    if let Some(print_caret_at) = print_caret_at {
                        println!("{}^", " ".repeat(print_caret_at));
                    }
                }
                _ => {
                    println!("Unknown type!");
                }
            }
        }
    }

    pub fn print_registers(&self, cpu: &CpuConfig) {
        let tracks = cpu.get_tracks();
        let num_tracks = tracks.len();
        let cur_track_num = self.cell_ptr % num_tracks;
        let offset = self.cell_ptr / num_tracks;
        for (_id, track) in tracks {
            match track {
                TrackKind::MultipleRegisters(track_num, register_map, binregister_map) => {
                    if cur_track_num as isize != *track_num {
                        continue;
                    }
                    for (name, register) in register_map {
                        let mut val_str = String::new();
                        let mut val = 0u32;
                        for i in 0..register.size {
                            let cell_val = self.cells[cur_track_num
                                + (offset + i as usize + register.offset as usize) * num_tracks];
                            val *= 256;
                            val += cell_val as u32;
                            val_str += &format!("{}, ", cell_val);
                        }
                        println!("{}: {}", name, val);
                    }
                    for (name, register) in binregister_map {
                        let mut val_str = String::new();
                        let mut val = 0u32;
                        for i in 0..register.size {
                            let cell_val = self.cells[cur_track_num
                                + (offset + i as usize + register.offset as usize) * num_tracks];
                            val *= 2;
                            val += cell_val as u32;
                            val_str += &format!("{}, ", cell_val);
                        }
                        println!("{}: {}", name, val);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn check_scratch_is_empty(&self, cpu: &CpuConfig) {
        let tracks = cpu.get_tracks();
        let num_tracks = tracks.len();
        for (id, track) in tracks {
            if let TrackKind::Scratch(track) = track {
                let mut i = track.track.track_num as usize;
                while i < self.cells.len() {
                    if self.cells[i] != 0 {
                        panic!(
                            "Scratch {:?} is not zero! at position {}: value {}",
                            id, i, self.cells[i]
                        );
                    }
                    i += num_tracks;
                }
            }
        }
    }
}

pub fn ops2str(ops: &Vec<BfOp>, print_optimizations: bool, clean_output: bool) -> String {
    fn write_shift(result: &mut String, shift: i16) {
        if shift < 0 {
            for _ in 0..-shift {
                *result += "<";
            }
        } else {
            for _ in 0..shift {
                *result += ">";
            }
        }
    }

    fn rec(ops: &Vec<BfOp>, result: &mut String, print_optimizations: bool, clean_output: bool) {
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
                    rec(ops, result, print_optimizations, clean_output);
                    *result += "]";
                }
                BfOp::Clr => {
                    if print_optimizations && !clean_output {
                        *result += "Clr";
                    } else {
                        *result += "[-]";
                    }
                }
                BfOp::Shift(shift) => {
                    if print_optimizations && !clean_output {
                        *result += &format!("Shift({})", shift);
                    } else {
                        write_shift(result, *shift);
                    }
                }
                BfOp::Add(val) => {
                    if print_optimizations && !clean_output {
                        *result += &format!("Add({})", val);
                    } else {
                        if *val <= 128 {
                            for _ in 0..*val {
                                *result += "+";
                            }
                        } else {
                            for _ in *val..=255 {
                                *result += "-";
                            }
                        }
                    }
                }
                BfOp::MoveAdd(shift) => {
                    if print_optimizations && !clean_output {
                        *result += &format!("MoveAdd({})", shift);
                    } else {
                        *result += "[-";
                        write_shift(result, *shift);
                        *result += "+";
                        write_shift(result, -*shift);
                        *result += "]";
                    }
                }
                BfOp::MoveAdd2(shift1, shift2) => {
                    if print_optimizations && !clean_output {
                        *result += &format!("MoveAdd2({}, {})", shift1, shift2);
                    } else {
                        *result += "[-";
                        write_shift(result, *shift1);
                        *result += "+";
                        write_shift(result, *shift2 - *shift1);
                        *result += "+";
                        write_shift(result, -*shift2);
                        *result += "]";
                    }
                }
                BfOp::Comment(msg) => {
                    if clean_output {
                        // no output
                    } else if print_optimizations && !clean_output {
                        *result += &format!("Comment({})", msg);
                    } else {
                        *result += msg;
                    }
                }
                BfOp::DebugMessage(msg) => {
                    if clean_output {
                        // no output
                    } else if print_optimizations {
                        *result += &format!("DebugMessage({})", msg);
                    } else {
                        *result += "#";
                    }
                }
                BfOp::Crash(msg) => {
                    if clean_output {
                        // no output
                    } else if print_optimizations {
                        *result += &format!("Crash({})", msg);
                    } else {
                        *result += "!";
                    }
                }
                BfOp::Breakpoint => {
                    if clean_output {
                        // no output
                    } else if print_optimizations {
                        *result += "Breakpoint";
                    } else {
                        *result += "$";
                    }
                }
                BfOp::PrintRegisters => {
                    if clean_output {
                        // no output
                    } else if print_optimizations {
                        *result += "PrintRegisters";
                    } else {
                        *result += "";
                    }
                }
                BfOp::CheckScratchIsEmptyFromHere(msg) => {
                    if clean_output {
                        // no output
                    } else if print_optimizations {
                        *result += &format!("CheckScratchIsEmptyFromHere({})", msg);
                    } else {
                        *result += "&";
                    }
                }
            }
        }
    }

    let mut result = String::new();
    rec(ops, &mut result, print_optimizations, clean_output);
    result
}
