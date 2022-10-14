use crate::{CpuConfig, TrackKind};
use num_format::{Locale, ToFormattedString};
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
        let mut comment = String::new();
        for (col, c) in line.chars().enumerate() {
            let pos = TextPos {
                line_num: line_num + 1,
                col: col + 1,
            };
            if "<>+-,.[]".contains(c) {
                if !comment.is_empty() {
                    stack
                        .last_mut()
                        .unwrap()
                        .ops
                        .push(BfOp::Comment(std::mem::replace(
                            &mut comment,
                            String::new(),
                        )));
                }
            } else {
                comment.push(c);
            }
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
        if !comment.is_empty() {
            stack.last_mut().unwrap().ops.push(BfOp::Comment(comment));
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

#[derive(Debug)]
pub enum RunOpError {
    PtrOutOfBounds,
    ReaderErr(std::io::Error),
    WriterErr(std::io::Error),
    Crashed,
    Other(String),
}

pub struct BfState {
    cells: Vec<u8>,
    cell_ptr: usize,
}

impl BfState {
    pub fn new() -> BfState {
        BfState {
            cells: vec![0; 1],
            cell_ptr: 0,
        }
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

    pub fn run_op_f(
        &mut self,
        op: &BfOp,
        get_char_in: &mut impl FnMut() -> Result<u8, RunOpError>,
        write_char_out: &mut impl FnMut(u8) -> Result<(), RunOpError>,
        cpu_config: Option<&CpuConfig>,
        mut loop_count: Option<&mut LoopCount>,
    ) -> Result<(), RunOpError> {
        if let Some(loop_count) = &mut loop_count {
            match op {
                BfOp::Comment(_) => {}
                BfOp::Breakpoint => {}
                BfOp::DebugMessage(_) => {}
                BfOp::CheckScratchIsEmptyFromHere(_) => {}
                BfOp::PrintRegisters => {}
                _ => {
                    loop_count.self_instrs_executed += 1;
                    loop_count.tot_instrs_executed += 1;
                }
            }
        }
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
                self.cells[self.cell_ptr] = get_char_in()?;
            }
            BfOp::Out => {
                let byte = self.cells[self.cell_ptr];
                write_char_out(byte)?;
            }
            BfOp::Loop(ops) => {
                if let Some(loop_count) = loop_count {
                    loop_count.tot_instrs_executed += loop_count.goto_next_loop(|loop_count| {
                        let at_begin = loop_count.tot_instrs_executed;
                        while self.cells[self.cell_ptr] != 0 {
                            loop_count.num_times_loop_run += 1;
                            loop_count.next_loop = 0;
                            self.run_ops_f(
                                ops,
                                &mut *get_char_in,
                                &mut *write_char_out,
                                cpu_config,
                                Some(&mut *loop_count),
                            )?;
                        }
                        assert!(loop_count.tot_instrs_executed >= at_begin);
                        Ok(loop_count.tot_instrs_executed - at_begin)
                    })?;
                } else {
                    while self.cells[self.cell_ptr] != 0 {
                        self.run_ops_f(
                            ops,
                            &mut *get_char_in,
                            &mut *write_char_out,
                            cpu_config,
                            None,
                        )?;
                    }
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
        loop_count: Option<&mut LoopCount>,
    ) -> Result<(), RunOpError> {
        self.run_ops_f(
            ops,
            &mut move || {
                let mut buf: [u8; 1] = [0; 1];
                loop {
                    match reader.read_exact(&mut buf) {
                        Ok(()) => {
                            // simply ignore \r
                            let c = buf[0];
                            if c != 13 {
                                return Ok(c);
                            }
                        }
                        Err(e) => match e.kind() {
                            std::io::ErrorKind::UnexpectedEof => {
                                return Ok(0);
                            }
                            _ => {
                                return Err(RunOpError::ReaderErr(e));
                            }
                        },
                    }
                }
            },
            &mut move |byte| {
                let buf: [u8; 1] = [byte];
                match writer.write_all(&buf) {
                    Ok(()) => {}
                    Err(e) => {
                        return Err(RunOpError::WriterErr(e));
                    }
                }
                match writer.flush() {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        return Err(RunOpError::WriterErr(e));
                    }
                }
            },
            cpu_config,
            loop_count,
        )
    }

    pub fn run_ops_f(
        &mut self,
        ops: &[BfOp],
        get_char_in: &mut impl FnMut() -> Result<u8, RunOpError>,
        write_char_out: &mut impl FnMut(u8) -> Result<(), RunOpError>,
        cpu_config: Option<&CpuConfig>,
        mut loop_count: Option<&mut LoopCount>,
    ) -> Result<(), RunOpError> {
        for op in ops {
            self.run_op_f(
                op,
                &mut *get_char_in,
                &mut *write_char_out,
                cpu_config,
                loop_count.as_deref_mut(),
            )?;
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

pub struct BfFormatOptions<'a> {
    pub print_optimizations: bool,
    pub clean_output: bool,
    pub indented: bool,
    pub only_loops_and_comments: bool,
    pub loop_count: Option<&'a LoopCount>,
}

impl<'a> BfFormatOptions<'a> {
    pub fn clean() -> BfFormatOptions<'static> {
        BfFormatOptions {
            print_optimizations: false,
            clean_output: true,
            indented: false,
            only_loops_and_comments: false,
            loop_count: None,
        }
    }

    pub fn clean_with_comments() -> BfFormatOptions<'static> {
        BfFormatOptions {
            print_optimizations: false,
            clean_output: false,
            indented: false,
            only_loops_and_comments: false,
            loop_count: None,
        }
    }

    pub fn with_opts() -> BfFormatOptions<'static> {
        BfFormatOptions {
            print_optimizations: true,
            clean_output: false,
            indented: false,
            only_loops_and_comments: false,
            loop_count: None,
        }
    }

    pub fn perf_clean(loop_count: &'a LoopCount) -> BfFormatOptions {
        BfFormatOptions {
            print_optimizations: false,
            clean_output: false,
            indented: true,
            only_loops_and_comments: true,
            loop_count: Some(loop_count),
        }
    }

    pub fn perf_verbose(loop_count: &'a LoopCount) -> BfFormatOptions {
        BfFormatOptions {
            print_optimizations: true,
            clean_output: false,
            indented: true,
            only_loops_and_comments: false,
            loop_count: Some(loop_count),
        }
    }

    fn should_print_optimizations(&self) -> bool {
        self.print_optimizations && !self.clean_output
    }
}

pub fn ops2str(ops: &Vec<BfOp>, format_opts: BfFormatOptions) -> String {
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

    fn rec(
        ops: &Vec<BfOp>,
        result: &mut String,
        format_opts: &BfFormatOptions,
        cur_indent_level: Option<usize>,
        mut loop_count: Option<(&LoopCount, usize)>,
    ) {
        for op in ops {
            match op {
                BfOp::Left => {
                    if !format_opts.only_loops_and_comments {
                        *result += "<";
                    }
                }
                BfOp::Right => {
                    if !format_opts.only_loops_and_comments {
                        *result += ">";
                    }
                }
                BfOp::Inc => {
                    if !format_opts.only_loops_and_comments {
                        *result += "+";
                    }
                }
                BfOp::Dec => {
                    if !format_opts.only_loops_and_comments {
                        *result += "-";
                    }
                }
                BfOp::In => {
                    if !format_opts.only_loops_and_comments {
                        *result += ",";
                    }
                }
                BfOp::Out => {
                    if !format_opts.only_loops_and_comments {
                        *result += ".";
                    }
                }
                BfOp::Loop(ops) => {
                    *result += "[";
                    if let Some(loop_count) = &mut loop_count {
                        *result += &format!(
                            "{}",
                            loop_count
                                .0
                                .children_counts
                                .get(loop_count.1)
                                .map(|l| l.tot_instrs_executed.to_formatted_string(&Locale::en))
                                .unwrap_or("0".to_string())
                        );
                    }
                    if let Some(cur_indent_level) = cur_indent_level {
                        *result += "\n";
                        for _ in 0..=cur_indent_level {
                            *result += " ";
                        }
                    }
                    let rec_loop_count = if let Some((loop_count, i)) = &loop_count {
                        if let Some(child_loop_count) = loop_count.children_counts.get(*i) {
                            Some((child_loop_count, 0))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    rec(
                        ops,
                        result,
                        format_opts,
                        cur_indent_level.map(|x| x + 1),
                        rec_loop_count,
                    );
                    if let Some((_, i)) = &mut loop_count {
                        *i += 1;
                    }
                    if let Some(cur_indent_level) = cur_indent_level {
                        *result += "\n";
                        for _ in 0..cur_indent_level {
                            *result += " ";
                        }
                    }
                    *result += "]";
                }
                BfOp::Clr => {
                    if !format_opts.only_loops_and_comments {
                        if format_opts.should_print_optimizations() {
                            *result += "Clr";
                        } else {
                            *result += "[-]";
                        }
                    }
                }
                BfOp::Shift(shift) => {
                    if !format_opts.only_loops_and_comments {
                        if format_opts.should_print_optimizations() {
                            *result += &format!("Shift({})", shift);
                        } else {
                            write_shift(result, *shift);
                        }
                    }
                }
                BfOp::Add(val) => {
                    if !format_opts.only_loops_and_comments {
                        if format_opts.should_print_optimizations() {
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
                }
                BfOp::MoveAdd(shift) => {
                    if !format_opts.only_loops_and_comments {
                        if format_opts.should_print_optimizations() {
                            *result += &format!("MoveAdd({})", shift);
                        } else {
                            *result += "[-";
                            write_shift(result, *shift);
                            *result += "+";
                            write_shift(result, -*shift);
                            *result += "]";
                        }
                    }
                }
                BfOp::MoveAdd2(shift1, shift2) => {
                    if !format_opts.only_loops_and_comments {
                        if format_opts.should_print_optimizations() {
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
                }
                BfOp::Comment(msg) => {
                    if format_opts.clean_output {
                        // no output
                    } else if format_opts.should_print_optimizations() {
                        *result += &format!("Comment({})", msg);
                    } else {
                        *result += msg;
                    }
                }
                BfOp::DebugMessage(msg) => {
                    if format_opts.clean_output {
                        // no output
                    } else if format_opts.should_print_optimizations() {
                        *result += &format!("DebugMessage({})", msg);
                    } else {
                        *result += "#";
                    }
                }
                BfOp::Crash(msg) => {
                    if format_opts.clean_output {
                        // no output
                    } else if format_opts.should_print_optimizations() {
                        *result += &format!("Crash({})", msg);
                    } else {
                        *result += "!";
                    }
                }
                BfOp::Breakpoint => {
                    if format_opts.clean_output {
                        // no output
                    } else if format_opts.should_print_optimizations() {
                        *result += "Breakpoint";
                    } else {
                        *result += "$";
                    }
                }
                BfOp::PrintRegisters => {
                    if format_opts.clean_output {
                        // no output
                    } else if format_opts.should_print_optimizations() {
                        *result += "PrintRegisters";
                    } else {
                        *result += "";
                    }
                }
                BfOp::CheckScratchIsEmptyFromHere(msg) => {
                    if format_opts.clean_output {
                        // no output
                    } else if format_opts.should_print_optimizations() {
                        *result += &format!("CheckScratchIsEmptyFromHere({})", msg);
                    } else {
                        *result += "&";
                    }
                }
            }
        }
    }

    let mut result = String::new();
    let cur_indent_level = if format_opts.indented { Some(0) } else { None };
    rec(
        ops,
        &mut result,
        &format_opts,
        cur_indent_level,
        format_opts.loop_count.map(|l| (l, 0)),
    );
    result
}

#[derive(Debug)]
pub struct LoopCount {
    self_instrs_executed: u64,
    tot_instrs_executed: u64,
    num_times_loop_run: u64,
    children_counts: Vec<LoopCount>,
    next_loop: usize,
}

impl LoopCount {
    pub fn new() -> LoopCount {
        LoopCount {
            self_instrs_executed: 0,
            tot_instrs_executed: 0,
            num_times_loop_run: 0,
            children_counts: Vec::new(),
            next_loop: 0,
        }
    }

    fn goto_next_loop<R>(&mut self, f: impl FnOnce(&mut LoopCount) -> R) -> R {
        if self.next_loop >= self.children_counts.len() {
            self.children_counts.push(LoopCount {
                self_instrs_executed: 0,
                tot_instrs_executed: 0,
                num_times_loop_run: 0,
                children_counts: Vec::new(),
                next_loop: 0,
            });
        }
        assert!(self.next_loop < self.children_counts.len());
        let ret_val = f(&mut self.children_counts[self.next_loop]);
        self.next_loop += 1;
        ret_val
    }

    pub fn get_self_instrs_executed(&self) -> u64 {
        self.self_instrs_executed
    }

    pub fn get_instrs_executed(&self) -> u64 {
        self.tot_instrs_executed
    }
}
