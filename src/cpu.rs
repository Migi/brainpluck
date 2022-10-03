use num::BigUint;
use num::Integer;
use num::Zero;
use std::collections::HashMap;

#[derive(Clone)]
pub enum Lir {
    Left,
    Right,
    Inc,
    Dec,
    In,
    Out,
    Loop(Vec<Lir>),
    DebugMessage(String),
    Crash(String),
    Breakpoint,
    CheckScratchIsEmptyFromHere(String),
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct Pos {
    pub frame: isize,
    pub track: isize,
}

impl Pos {
    fn index(&self, cfg: &CpuConfig) -> isize {
        self.frame * cfg.frame_size() + self.track
    }

    fn get_shifted(&self, frame_shift: isize) -> Pos {
        Pos {
            track: self.track,
            frame: self.frame + frame_shift,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Track {
    pub track_num: isize,
}

impl Track {
    pub fn at(&self, frame: isize) -> Pos {
        Pos {
            frame,
            track: self.track_num,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Register {
    pub track: Track,
    pub size: isize,
    pub offset: isize,
}

impl Register {
    pub fn at(&self, frame: isize) -> Pos {
        assert!(frame >= 0);
        assert!(frame < self.size);
        self.track.at(frame + self.offset)
    }
}

#[derive(Clone, Copy)]
pub struct BinRegister {
    pub track: Track,
    pub size: isize,
    pub offset: isize,
}

impl BinRegister {
    pub fn at(&self, frame: isize) -> Pos {
        assert!(frame >= 0);
        assert!(frame < self.size);
        self.track.at(frame + self.offset)
    }

    pub fn last_pos(&self) -> Pos {
        self.at(self.size - 1)
    }

    pub fn at_unchecked(&self, frame: isize) -> Pos {
        self.track.at(frame + self.offset)
    }

    pub fn subview(&self, offset: isize, size: isize) -> BinRegister {
        assert!(offset + size <= self.size);
        BinRegister {
            track: self.track,
            size,
            offset: self.offset + offset,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ScratchTrack {
    pub track: Track,
    pub offset: isize,                  // shift all accesses by this amount
    pub dont_go_left_of: Option<isize>, // don't access left of this position (at offset=0)
}

impl ScratchTrack {
    pub fn at(&self, frame: isize) -> Pos {
        if let Some(l) = self.dont_go_left_of {
            if frame + self.offset < l {
                panic!(
                    "Going left of scratchtrack's limits! {}+{} < {}",
                    frame, self.offset, l
                );
            }
        }
        self.track.at(frame + self.offset)
    }

    #[allow(unused)]
    pub fn get_2_pos(&self, start: isize) -> [Pos; 2] {
        [self.at(start), self.at(start + 1)]
    }

    #[allow(unused)]
    pub fn get_3_pos(&self, start: isize) -> [Pos; 3] {
        [self.at(start), self.at(start + 1), self.at(start + 2)]
    }

    #[allow(unused)]
    pub fn get_4_pos(&self, start: isize) -> [Pos; 4] {
        [
            self.at(start),
            self.at(start + 1),
            self.at(start + 2),
            self.at(start + 3),
        ]
    }

    #[allow(unused)]
    pub fn get_5_pos(&self, start: isize) -> [Pos; 5] {
        [
            self.at(start),
            self.at(start + 1),
            self.at(start + 2),
            self.at(start + 3),
            self.at(start + 4),
        ]
    }

    fn get_split_scratch(&self, num_poss_split: isize) -> ScratchTrack {
        ScratchTrack {
            track: self.track,
            dont_go_left_of: Some(self.dont_go_left_of.unwrap_or(0) + num_poss_split),
            offset: self.offset,
        }
    }

    #[allow(unused)]
    pub fn split_1(self) -> (Pos, ScratchTrack) {
        (
            self.at(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(1),
        )
    }

    #[allow(unused)]
    pub fn split_2(self) -> ([Pos; 2], ScratchTrack) {
        (
            self.get_2_pos(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(2),
        )
    }

    #[allow(unused)]
    pub fn split_3(self) -> ([Pos; 3], ScratchTrack) {
        (
            self.get_3_pos(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(3),
        )
    }

    #[allow(unused)]
    pub fn split_4(self) -> ([Pos; 4], ScratchTrack) {
        (
            self.get_4_pos(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(4),
        )
    }

    #[allow(unused)]
    pub fn split_5(self) -> ([Pos; 5], ScratchTrack) {
        (
            self.get_5_pos(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(5),
        )
    }

    pub fn shift_so_frame_is_legal(&mut self, frame: isize) {
        if let Some(l) = self.dont_go_left_of {
            if frame + self.offset < l {
                self.offset = l - frame;
            }
        }
    }

    #[allow(unused)]
    pub fn split_register(self, size: isize) -> (Register, ScratchTrack) {
        (
            Register {
                offset: self.dont_go_left_of.unwrap_or(0) - self.offset,
                size,
                track: self.track,
            },
            self.get_split_scratch(size),
        )
    }

    #[allow(unused)]
    pub fn split_binregister(self, size: isize) -> (BinRegister, ScratchTrack) {
        (
            BinRegister {
                offset: self.dont_go_left_of.unwrap_or(0) - self.offset,
                size,
                track: self.track,
            },
            self.get_split_scratch(size),
        )
    }
}

fn all_different<T: PartialEq>(elements: &[T]) -> bool {
    for i in 0..elements.len() {
        for j in i + 1..elements.len() {
            if elements[i] == elements[j] {
                return false;
            }
        }
    }
    true
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum TrackId {
    Stack = 0,
    Heap,
    Scratch1,
    Scratch2,
    Scratch3,
    CurDataPtr,
    Register1,
}

#[derive(Clone)]
pub struct CpuConfig {
    tracks: HashMap<TrackId, TrackKind>,
}

impl CpuConfig {
    pub fn new() -> CpuConfig {
        CpuConfig {
            tracks: HashMap::new(),
        }
    }

    pub fn get_tracks(&self) -> &HashMap<TrackId, TrackKind> {
        &self.tracks
    }

    pub fn frame_size(&self) -> isize {
        self.tracks.len() as isize
    }

    pub fn add_data_track(&mut self, id: TrackId) -> Track {
        let track = Track {
            track_num: self.tracks.len() as isize,
        };
        let old = self.tracks.insert(id, TrackKind::Data(track));
        assert!(old.is_none());
        track
    }

    pub fn add_scratch_track(&mut self, id: TrackId) -> ScratchTrack {
        let track = ScratchTrack {
            track: Track {
                track_num: self.tracks.len() as isize,
            },
            offset: 0,
            dont_go_left_of: None,
        };
        let old = self.tracks.insert(id, TrackKind::Scratch(track));
        assert!(old.is_none());
        track
    }

    pub fn add_register_track(&mut self, id: TrackId, size: isize) -> Register {
        let track = Register {
            size,
            track: Track {
                track_num: self.tracks.len() as isize,
            },
            offset: 0,
        };
        let old = self.tracks.insert(id, TrackKind::Register(track));
        assert!(old.is_none());
        track
    }

    pub fn build_register_track(&mut self, id: TrackId) -> RegisterTrackBuilder {
        let track_num = self.tracks.len() as isize;
        let track = Track { track_num };
        let old = self.tracks.insert(id, TrackKind::Data(track));
        assert!(old.is_none());
        RegisterTrackBuilder {
            cur_offset: 0,
            track_num,
        }
    }
}

#[derive(Clone, Copy)]
pub enum TrackKind {
    Data(Track),
    Register(Register),
    BinRegister(BinRegister),
    Scratch(ScratchTrack),
}

pub struct RegisterTrackBuilder {
    cur_offset: isize,
    track_num: isize,
}

impl RegisterTrackBuilder {
    pub fn add_register(&mut self, size: isize) -> Register {
        let register = Register {
            size,
            track: Track {
                track_num: self.track_num,
            },
            offset: self.cur_offset,
        };
        self.cur_offset += size;
        register
    }

    pub fn add_binregister(&mut self, size: isize) -> BinRegister {
        let register = BinRegister {
            size,
            track: Track {
                track_num: self.track_num,
            },
            offset: self.cur_offset,
        };
        self.cur_offset += size;
        register
    }
}

pub struct Cpu<'c> {
    cfg: &'c CpuConfig,
    cur_track: isize,
    cur_frame: Option<isize>,
    lir: Vec<Lir>,
}

impl<'c> Cpu<'c> {
    pub fn new(cfg: &'c CpuConfig) -> Cpu<'c> {
        let mut cpu = Cpu {
            cfg,
            cur_track: 0,
            cur_frame: Some(0),
            lir: Vec::new(),
        };
        for _ in 0..(cpu.cfg.frame_size() * 3) {
            cpu.lir.push(Lir::Right);
        }
        cpu
    }

    pub fn get_cfg(&self) -> &CpuConfig {
        self.cfg
    }

    pub fn into_ops(self) -> Vec<Lir> {
        self.lir
    }

    pub fn clone_ops(self) -> Vec<Lir> {
        self.lir
    }

    pub fn inc(&mut self) {
        self.lir.push(Lir::Inc);
    }

    pub fn dec(&mut self) {
        self.lir.push(Lir::Dec);
    }

    pub fn out(&mut self) {
        self.lir.push(Lir::Out);
    }

    pub fn debug_message(&mut self, msg: impl Into<String>) {
        self.lir.push(Lir::DebugMessage(msg.into()));
    }

    pub fn crash(&mut self, msg: impl Into<String>) {
        self.lir.push(Lir::Crash(msg.into()));
    }

    pub fn breakpoint(&mut self) {
        self.lir.push(Lir::Breakpoint);
    }

    pub fn check_scratch(&mut self, scratch_track: ScratchTrack, msg: impl Into<String>) {
        self.goto(scratch_track.split_1().0);
        self.lir.push(Lir::CheckScratchIsEmptyFromHere(msg.into()));
    }

    pub fn inc_at(&mut self, pos: Pos) {
        self.goto(pos);
        self.inc();
    }

    pub fn dec_at(&mut self, pos: Pos) {
        self.goto(pos);
        self.dec();
    }

    pub fn inc_by(&mut self, x: isize) {
        if x > 0 {
            for _ in 0..x {
                self.inc();
            }
        } else {
            for _ in 0..x.abs() {
                self.dec();
            }
        }
    }

    pub fn shift_cursor_untracked(&mut self, shift: isize) {
        if shift < 0 {
            for _ in 0..shift.abs() {
                self.lir.push(Lir::Left);
            }
        } else {
            for _ in 0..shift {
                self.lir.push(Lir::Right);
            }
        }
        self.cur_frame = None;
    }

    pub fn shift_frame_untracked(&mut self, shift: isize) {
        self.shift_cursor_untracked(shift * self.cfg.frame_size());
    }

    pub fn go_clear_sentinel_left(&mut self, landing_pos: Pos) {
        assert_eq!(self.cur_track, landing_pos.track);
        self.dec();
        self.raw_loop(|cpu| {
            cpu.inc();
            cpu.shift_frame_untracked(-1);
            cpu.dec();
        });
        self.cur_frame = Some(landing_pos.frame);
    }

    pub fn go_clear_sentinel_right(&mut self, landing_pos: Pos) {
        assert_eq!(self.cur_track, landing_pos.track);
        self.dec();
        self.raw_loop(|cpu| {
            cpu.inc();
            cpu.shift_frame_untracked(1);
            cpu.dec();
        });
        self.cur_frame = Some(landing_pos.frame);
    }

    pub fn goto_sentinel_left(&mut self, landing_pos: Pos) {
        self.go_clear_sentinel_left(landing_pos);
        self.inc();
    }

    pub fn goto_sentinel_right(&mut self, landing_pos: Pos) {
        self.go_clear_sentinel_right(landing_pos);
        self.inc();
    }

    pub fn go_clear_downsentinel_left(&mut self, landing_pos: Pos) {
        assert_eq!(self.cur_track, landing_pos.track);
        self.inc();
        self.raw_loop(|cpu| {
            cpu.dec();
            cpu.shift_frame_untracked(-1);
            cpu.inc();
        });
        self.cur_frame = Some(landing_pos.frame);
    }

    pub fn go_clear_downsentinel_right(&mut self, landing_pos: Pos) {
        assert_eq!(self.cur_track, landing_pos.track);
        self.inc();
        self.raw_loop(|cpu| {
            cpu.dec();
            cpu.shift_frame_untracked(1);
            cpu.inc();
        });
        self.cur_frame = Some(landing_pos.frame);
    }

    pub fn goto_downsentinel_left(&mut self, landing_pos: Pos) {
        self.go_clear_downsentinel_left(landing_pos);
        self.dec();
    }

    pub fn goto_downsentinel_right(&mut self, landing_pos: Pos) {
        self.go_clear_downsentinel_right(landing_pos);
        self.dec();
    }

    pub fn goto_track(&mut self, track: isize) {
        self.shift_cursor_untracked(track - self.cur_track);
        self.cur_track = track;
    }

    // self.cur_frame.unwrap() but better debugging messages
    fn unwrap_cur_frame(&self) -> isize {
        match self.cur_frame {
            Some(cur_frame) => cur_frame,
            None => panic!("Trying to unwrap cur_frame but it's None"),
        }
    }

    pub fn goto_frame(&mut self, frame: isize) {
        let cur_frame = self.unwrap_cur_frame();
        self.shift_cursor_untracked((frame - cur_frame) * self.cfg.frame_size());
        self.cur_frame = Some(frame);
    }

    pub fn goto(&mut self, pos: Pos) {
        let cur_frame = self.unwrap_cur_frame();
        self.shift_cursor_untracked(
            (pos.track - self.cur_track) + (pos.frame - cur_frame) * self.cfg.frame_size(),
        );
        self.cur_frame = Some(pos.frame);
        self.cur_track = pos.track;
    }

    pub fn loop_while(&mut self, at: Pos, f: impl for<'a> FnOnce(&'a mut Cpu)) {
        self.goto(at);
        let mut cpu = Cpu {
            cfg: self.cfg,
            cur_track: self.cur_track,
            cur_frame: self.cur_frame,
            lir: Vec::new(),
        };
        f(&mut cpu);
        cpu.goto(at);
        self.lir.push(Lir::Loop(cpu.lir));
    }

    pub fn raw_loop(&mut self, f: impl for<'a> FnOnce(&'a mut Cpu)) {
        let mut cpu = Cpu {
            cfg: self.cfg,
            cur_track: self.cur_track,
            cur_frame: self.cur_frame,
            lir: Vec::new(),
        };
        f(&mut cpu);
        self.cur_frame = self.cur_frame.and_then(|self_cur_frame| {
            cpu.cur_frame.and_then(|cpu_cur_frame| {
                if self_cur_frame == cpu_cur_frame {
                    Some(self_cur_frame)
                } else {
                    None
                }
            })
        });
        assert_eq!(self.cur_track, cpu.cur_track);
        self.lir.push(Lir::Loop(cpu.lir));
    }

    /*pub fn move_frame(&mut self, shift: isize) {
        for (_,track) in self.tracks.iter_mut() {
            match track {
                TrackKind::Register(register) => {
                    self.move_slice_onto_zero_slice(
                        register.at(0),
                        register.size,
                        register.at(shift)
                    );
                },
                _ => {}
            }
        }
    }*/

    pub fn get_pos_on_track_between(&self, a: Pos, b: Pos, track: Track) -> Pos {
        let eval = |p: Pos| {
            (a.index(&self.cfg) - p.index(&self.cfg)).abs()
                + (b.index(&self.cfg) - p.index(&self.cfg)).abs()
        };
        let mut best_pos = track.at(a.frame);
        let mut best_score = eval(best_pos);
        for i in std::cmp::min(a.frame, b.frame)..=std::cmp::max(a.frame, b.frame) + 1 {
            let pos = track.at(i);
            let score = eval(pos);
            if score < best_score {
                best_pos = pos;
                best_score = score;
            }
        }
        best_pos
    }

    /// Get a position on the scratch track which is near (but not the same as) a and/or b
    pub fn get_autoscratch(&self, a: Pos, b: Pos, scratch_track: ScratchTrack) -> Pos {
        let mut scratch = self.get_pos_on_track_between(a, b, scratch_track.track);
        if let Some(l) = scratch_track.dont_go_left_of {
            if scratch.frame < l {
                scratch.frame = l;
            }
        }
        if a == scratch {
            scratch.frame += 1;
            if b == scratch {
                scratch.frame += 1;
            }
        } else if b == scratch {
            scratch.frame += 1;
            if a == scratch {
                scratch.frame += 1;
            }
        }
        scratch
    }

    pub fn zero_byte(&mut self, pos: Pos) {
        self.loop_while(pos, |cpu| {
            cpu.dec();
        });
    }

    pub fn clr(&mut self) {
        self.raw_loop(|cpu| {
            cpu.dec();
        });
    }

    pub fn clr_at(&mut self, at: Pos) {
        self.goto(at);
        self.clr();
    }

    pub fn add_const_to_byte(&mut self, pos: Pos, val: u8) {
        if val <= 128 {
            for _ in 0..val {
                self.inc_at(pos);
            }
        } else {
            for _ in val..=255 {
                self.dec_at(pos);
            }
        }
    }

    pub fn add_const_to_byte_with_carry_slow(
        &mut self,
        pos: Pos,
        val: u8,
        carry: Pos,
        scratch1: Pos,
        scratch2: Pos,
    ) {
        assert!(all_different(&[pos, scratch1, scratch2]));
        self.set_byte(scratch1, val);
        self.moveadd_byte_with_carry_slow(scratch1, pos, carry, scratch2);
    }

    pub fn sub_const_from_byte(&mut self, pos: Pos, val: u8) {
        for _ in 0..val {
            self.dec_at(pos);
        }
    }

    pub fn set_byte(&mut self, pos: Pos, val: u8) {
        self.clr_at(pos);
        self.add_const_to_byte(pos, val);
    }

    pub fn zero_slice(&mut self, slice: Pos, size: isize) {
        for i in 0..size {
            self.zero_byte(Pos {
                frame: slice.frame + i,
                track: slice.track,
            });
        }
    }

    pub fn zero_register(&mut self, register: Register) {
        self.zero_slice(register.at(0), register.size);
    }

    pub fn clear_register_track_to_scratch_track(&mut self, register: Register) -> ScratchTrack {
        self.zero_register(register);
        ScratchTrack {
            track: register.track,
            offset: 0,
            dont_go_left_of: None,
        }
    }

    /// Does
    /// to += from
    /// from = 0
    pub fn moveadd_byte(&mut self, from: Pos, to: Pos) {
        if from == to {
            return;
        }
        self.loop_while(from, |cpu| {
            cpu.dec();
            cpu.inc_at(to);
        });
    }

    pub fn movesub_byte(&mut self, from: Pos, to: Pos) {
        if from == to {
            self.clr_at(from);
        }
        self.loop_while(from, |cpu| {
            cpu.dec();
            cpu.dec_at(to);
        });
    }

    pub fn now_were_actually_at(&mut self, pos: Pos) {
        self.cur_frame = Some(pos.frame);
    }

    pub fn now_if_we_were_at_a_wed_actually_be_at_b(&mut self, a: Pos, b: Pos) {
        assert_eq!(a.track, b.track);
        if let Some(cur_frame) = self.cur_frame {
            self.cur_frame = Some(cur_frame + b.frame - a.frame);
        } else {
            panic!("now_if_we_were_at_a_wed_actually_be_at_b() called while cur_frame is None");
        }
    }

    pub fn if_nonzero_else(
        &mut self,
        cond: Pos,
        scratch_track: ScratchTrack,
        if_nonzero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
        if_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
    ) {
        let ([byte_cpy, one, zero, zero2], new_scratch) = scratch_track.split_4();
        self.copy_byte(cond, byte_cpy, zero);
        self.inc_at(one);
        self.goto(byte_cpy);
        self.raw_loop(move |cpu| {
            cpu.clr();
            if_nonzero(cpu, new_scratch);
            cpu.goto(zero);
        });
        // we could be at byte_cpy (if 0) or at zero
        assert_eq!(self.cur_frame, None);
        assert_eq!(self.cur_track, scratch_track.track.track_num);
        self.shift_frame_untracked(1);
        // now we're at one or zero2
        self.raw_loop(move |cpu| {
            cpu.cur_frame = Some(one.frame);
            if_zero(cpu, new_scratch);
            cpu.goto(zero2);
        });
        self.cur_frame = Some(zero2.frame);
        self.dec_at(one);
    }

    pub fn not(&mut self, pos: Pos, scratch_track: ScratchTrack) {
        self.if_nonzero_else(
            pos,
            scratch_track,
            |cpu, _| {
                cpu.clr_at(pos);
            },
            |cpu, _| {
                cpu.inc_at(pos);
            },
        );
    }

    pub fn if_register_nonzero_else(
        &mut self,
        register: Register,
        scratch_track: ScratchTrack,
        if_nonzero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
        if_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
    ) {
        let (acc, scratch_track) = scratch_track.split_1();
        for i in 0..register.size {
            self.if_nonzero_else(
                register.at(i),
                scratch_track,
                |cpu, _| {
                    cpu.inc_at(acc);
                },
                |_, _| {},
            );
        }
        self.if_nonzero_else(acc, scratch_track, if_nonzero, if_zero);
        self.clr_at(acc);
    }

    pub fn cmp_register(
        &mut self,
        register: Register,
        scratch_track: ScratchTrack,
        if_lt_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
        if_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
        if_gt_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
    ) {
        self.inc_at(register.at(0));
        self.if_nonzero_else(
            register.at(0),
            scratch_track,
            |cpu, scratch_track| {
                cpu.dec_at(register.at(0));
                if_lt_zero(cpu, scratch_track)
            },
            |cpu, scratch_track| {
                cpu.dec_at(register.at(0));
                cpu.if_register_nonzero_else(register, scratch_track, if_gt_zero, if_zero)
            },
        );
    }

    pub fn movesub_byte_clamped(&mut self, from: Pos, to: Pos, scratch_track: ScratchTrack) {
        if from == to {
            self.clr_at(from);
            self.clr_at(to);
        }
        self.loop_while(from, |cpu| {
            cpu.dec();
            cpu.if_nonzero_else(
                to,
                scratch_track,
                |cpu, _| {
                    cpu.dec_at(to);
                },
                |cpu, _| {
                    cpu.clr_at(from);
                },
            );
        });
    }

    pub fn move_slice_onto_zero_slice(&mut self, slice: Pos, size: isize, to: Pos) {
        if slice == to {
            return;
        }
        if slice.frame < to.frame {
            for i in 0..size {
                self.moveadd_byte(
                    Pos {
                        frame: slice.frame + i,
                        track: slice.track,
                    },
                    Pos {
                        frame: to.frame + i,
                        track: to.track,
                    },
                );
            }
        } else {
            for i in (0..size).rev() {
                self.moveadd_byte(
                    Pos {
                        frame: slice.frame + i,
                        track: slice.track,
                    },
                    Pos {
                        frame: to.frame + i,
                        track: to.track,
                    },
                );
            }
        }
    }

    pub fn move_onto_zero_register(&mut self, from: Register, to: Register) {
        assert_eq!(from.size, to.size);
        self.move_slice_onto_zero_slice(from.at(0), from.size, to.at(0));
    }

    pub fn copy_byte(&mut self, from: Pos, to: Pos, scratch: Pos) {
        if from == to {
            return;
        }
        assert!(from != scratch);
        assert!(to != scratch);
        self.moveadd_byte(from, scratch);
        self.loop_while(scratch, |cpu| {
            cpu.dec();
            cpu.inc_at(from);
            cpu.inc_at(to);
        });
    }

    pub fn sub_byte_from(&mut self, sub: Pos, from: Pos, scratch: Pos) {
        if sub == from {
            self.clr_at(sub);
        }
        assert!(sub != scratch);
        assert!(from != scratch);
        self.moveadd_byte(sub, scratch);
        self.loop_while(scratch, |cpu| {
            cpu.dec();
            cpu.inc_at(sub);
            cpu.dec_at(from);
        });
    }

    pub fn copy_byte_autoscratch(&mut self, from: Pos, to: Pos, scratch_track: ScratchTrack) {
        if from == to {
            return;
        }
        let scratch = self.get_autoscratch(from, to, scratch_track);
        self.copy_byte(from, to, scratch);
    }

    pub fn sub_byte_autoscratch(&mut self, sub: Pos, from: Pos, scratch_track: ScratchTrack) {
        if sub == from {
            self.clr_at(sub);
        }
        let scratch = self.get_autoscratch(sub, from, scratch_track);
        self.sub_byte_from(sub, from, scratch);
    }

    pub fn copy_slice(&mut self, slice: Pos, size: isize, to: Pos, scratch_track: ScratchTrack) {
        if slice == to {
            return;
        }
        if slice.frame < to.frame {
            for i in 0..size {
                self.copy_byte_autoscratch(
                    Pos {
                        frame: slice.frame + i,
                        track: slice.track,
                    },
                    Pos {
                        frame: to.frame + i,
                        track: to.track,
                    },
                    scratch_track,
                );
            }
        } else {
            for i in (0..size).rev() {
                self.copy_byte_autoscratch(
                    Pos {
                        frame: slice.frame + i,
                        track: slice.track,
                    },
                    Pos {
                        frame: to.frame + i,
                        track: to.track,
                    },
                    scratch_track,
                );
            }
        }
    }

    pub fn copy_register(&mut self, from: Register, to: Register, scratch_track: ScratchTrack) {
        assert_eq!(from.size, to.size);
        self.copy_slice(from.at(0), from.size, to.at(0), scratch_track);
    }

    pub fn moveadd_byte_with_carry_slow(&mut self, a: Pos, b: Pos, carry: Pos, scratch: Pos) {
        // slow, but unfortunately I don't know a much faster alternative...
        self.loop_while(a, |cpu| {
            cpu.dec();
            cpu.inc_at(carry);
            cpu.inc_at(b);
            cpu.loop_while(b, |cpu| {
                cpu.moveadd_byte(b, scratch);
                cpu.dec_at(carry);
            });
            cpu.moveadd_byte(scratch, b);
        });
    }

    // untested!
    pub fn moveadd_registers_slow(
        &mut self,
        a: Register,
        b: Register,
        mut scratch_track: ScratchTrack,
    ) {
        assert_eq!(a.size, b.size);
        scratch_track.shift_so_frame_is_legal(0);
        for i in (0..a.size).rev() {
            if i > 0 {
                self.moveadd_byte_with_carry_slow(
                    a.at(i),
                    b.at(i),
                    scratch_track.at(i - 1),
                    scratch_track.at(i),
                );
                if i > 1 {
                    self.moveadd_byte_with_carry_slow(
                        scratch_track.at(i - 1),
                        b.at(i - 1),
                        scratch_track.at(i - 2),
                        scratch_track.at(i),
                    );
                } else {
                    self.moveadd_byte(scratch_track.at(i - 1), b.at(i - 1));
                }
            } else {
                self.moveadd_byte(a.at(i), b.at(i));
            }
        }
    }

    /// Does:
    /// b += a
    /// a = 0
    /// untested!
    pub fn moveadd_registers(
        &mut self,
        a: Register,
        b: Register,
        scratch_track1: ScratchTrack,
        scratch_track2: ScratchTrack,
    ) {
        assert_eq!(a.size, b.size);
        assert!(all_different(&[
            a.track,
            b.track,
            scratch_track1.track,
            scratch_track2.track
        ]));

        let mut sentinel_track = scratch_track1;
        let mut carry_track = scratch_track2;

        sentinel_track.shift_so_frame_is_legal(-2);
        carry_track.shift_so_frame_is_legal(-2);

        // make offsets of both scratch tracks the same
        let offset = std::cmp::min(sentinel_track.offset, carry_track.offset);
        sentinel_track.offset = offset;
        carry_track.offset = offset;

        self.inc_at(sentinel_track.at(-2));
        for i in (1..a.size).rev() {
            self.inc_at(sentinel_track.at(i));
            self.loop_while(a.at(i), |cpu| {
                cpu.dec();
                let x = i; // x starts at i, then keeps shifting left
                cpu.inc_at(carry_track.at(x));
                cpu.loop_while(carry_track.at(x), |cpu| {
                    cpu.dec();
                    cpu.inc_at(carry_track.at(x - 1));
                    cpu.inc_at(b.at(x));
                    cpu.raw_loop(|cpu| {
                        cpu.dec_at(carry_track.at(x - 1));
                        cpu.goto(sentinel_track.at(x - 1));
                        cpu.goto_sentinel_left(sentinel_track.at(x - 1)); // ??
                        cpu.goto_track(b.track.track_num);
                    });
                    cpu.shift_frame_untracked(-1);
                    cpu.cur_frame = Some(carry_track.at(x).frame);
                });
                cpu.goto(sentinel_track.at(x + 1));
                cpu.goto_sentinel_right(carry_track.at(i));
            });
            self.dec_at(sentinel_track.at(i));
        }
        self.moveadd_byte(a.at(0), b.at(0));
        self.zero_byte(b.at(-1));
        self.dec_at(sentinel_track.at(-2));
    }

    pub fn movediv_byte_onto_zeros(
        &mut self,
        a: Pos,
        divisor: u8,
        div_result: Pos,
        rem_result: Pos,
        mut scratch_track: ScratchTrack,
    ) {
        scratch_track.shift_so_frame_is_legal(0);

        // scratch structure:
        // - 0: divisor (kind of)
        // - 1: remainder
        // - 2: always 0
        // - 3: always 0
        assert_ne!(divisor, 0);
        assert_ne!(divisor, 1);
        self.add_const_to_byte(scratch_track.at(0), divisor - 1);
        self.loop_while(a, |cpu| {
            cpu.dec();
            cpu.goto(scratch_track.at(0));
            cpu.raw_loop(|cpu| {
                // we must be at 0
                assert_eq!(cpu.cur_frame, Some(scratch_track.at(0).frame));
                cpu.dec();
                cpu.inc_at(scratch_track.at(1));
                cpu.goto(scratch_track.at(2));
            });
            assert!(cpu.cur_frame.is_none()); // could be 0 or 2
            cpu.shift_frame_untracked(1); // we're at 1 or 3
            cpu.raw_loop(|cpu| {
                // we must be at 1, divisor == 0, remainder > 0
                cpu.cur_frame = Some(scratch_track.at(1).frame);
                cpu.moveadd_byte(scratch_track.at(1), scratch_track.at(0));
                cpu.inc_at(div_result);
                cpu.goto(scratch_track.at(3));
            });
            cpu.cur_frame = Some(scratch_track.at(3).frame);
        });
        self.moveadd_byte(scratch_track.at(1), rem_result);
        self.zero_byte(scratch_track.at(0));
    }

    pub fn moveprint_digit(&mut self, pos: Pos) {
        self.add_const_to_byte(pos, 48);
        self.out();
        self.clr();
    }

    pub fn moveprint_digit_if_nonzero(&mut self, pos: Pos) {
        self.loop_while(pos, |cpu| {
            cpu.add_const_to_byte(pos, 48);
            cpu.out();
            cpu.clr();
        });
    }

    pub fn moveprint_hex_digit(&mut self, pos: Pos, scratch_track: ScratchTrack) {
        let ([byte_cpy, zero], scratch_track) = scratch_track.split_2();
        self.copy_byte(pos, byte_cpy, zero);
        self.add_const_to_byte(pos, 48);

        self.add_const_to_byte(zero, 9);
        self.movesub_byte_clamped(zero, byte_cpy, scratch_track);

        self.if_nonzero_else(
            byte_cpy,
            scratch_track,
            |cpu, _| {
                cpu.goto(pos);
                cpu.inc_by(7);
                cpu.out();
                cpu.clr();
            },
            |cpu, _| {
                cpu.goto(pos);
                cpu.out();
                cpu.clr();
            },
        );
        self.clr_at(byte_cpy);
        self.clr_at(pos);
    }

    pub fn print_char(&mut self, c: char, scratch: Pos) {
        let x = c as u32;
        if x <= 8 || x >= 127 {
            panic!("Printing unprintable char");
        }
        let x = x as u8;
        self.add_const_to_byte(scratch, x);
        self.out();
        self.clr();
    }

    pub fn print_text(&mut self, s: &str, scratch_track: ScratchTrack) {
        let (pos, _) = scratch_track.split_1();
        for c in s.chars() {
            self.print_char(c, pos);
        }
    }

    pub fn moveprint_byte(&mut self, pos: Pos, scratch_track: ScratchTrack) {
        let ([singles, temp, tens, hundreds], div_scratch_track) = scratch_track.split_4();
        self.movediv_byte_onto_zeros(pos, 10, temp, singles, div_scratch_track);
        self.movediv_byte_onto_zeros(temp, 10, hundreds, tens, div_scratch_track);
        self.moveprint_digit_if_nonzero(hundreds);
        self.moveprint_digit_if_nonzero(tens);
        self.moveprint_digit(singles);
    }

    pub fn moveprint_register_hex(&mut self, register: Register, scratch_track: ScratchTrack) {
        self.print_text("0x", scratch_track);
        for i in 0..register.size {
            let ([left, right], scratch_track) = scratch_track.split_2();
            self.movediv_byte_onto_zeros(register.at(i), 16, left, right, scratch_track);
            self.moveprint_hex_digit(left, scratch_track);
            self.moveprint_hex_digit(right, scratch_track);
        }
    }

    pub fn print_newline(&mut self, scratch_track: ScratchTrack) {
        self.print_char('\n', scratch_track.split_1().0);
    }

    pub fn set_register(&mut self, register: Register, val: BigUint) {
        let two_fifty_six = BigUint::from(256u64);
        let zero = BigUint::zero();
        let mut div = val;
        let mut i = 0;
        while &div != &zero {
            if i >= register.size {
                panic!("Value too big to fit in register");
            }
            let (new_div, rem) = div.div_rem(&two_fifty_six);
            div = new_div;
            let rem_bytes = rem.to_bytes_be();
            assert_eq!(rem_bytes.len(), 1);
            let rem = rem_bytes.last().unwrap();
            self.set_byte(register.at(register.size - i - 1), *rem);
            i += 1;
        }
    }

    pub fn add_const_to_register(
        &mut self,
        register: Register,
        val: BigUint,
        scratch_track: ScratchTrack,
    ) {
        let ([carry, scratch1, scratch2], _) = scratch_track.split_3();
        let two_fifty_six = BigUint::from(256u64);
        let zero = BigUint::zero();
        let mut div = val;
        let mut i = 0;
        while &div != &zero {
            if i >= register.size {
                panic!("Value too big to fit in register");
            }
            let (new_div, rem) = div.div_rem(&two_fifty_six);
            div = new_div;
            let rem_bytes = rem.to_bytes_be();
            assert_eq!(rem_bytes.len(), 1);
            let rem = rem_bytes.last().unwrap();
            if i == register.size - 1 {
                self.add_const_to_byte_with_carry_slow(
                    register.at(register.size - i - 1),
                    *rem,
                    carry,
                    scratch1,
                    scratch2,
                );
                self.clr_at(carry);
            } else {
                self.add_const_to_byte_with_carry_slow(
                    register.at(register.size - i - 1),
                    *rem,
                    register.at(register.size - i - 2),
                    scratch1,
                    scratch2,
                );
            }
            i += 1;
        }
    }

    /// In callbacks, the frame will shift every time, so if you want to keep scratch data
    /// across iterations you need to shift it right at the end of every callback
    pub fn foreach_pos_of_binregister(
        &mut self,
        register: BinRegister,
        scratch_track: ScratchTrack,
        init: Option<impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack)>,
        f: impl for<'a> FnOnce(&'a mut Cpu, Pos, ScratchTrack),
        fin: Option<impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack)>,
    ) {
        let ([counter, sentinel1, sentinel2], scratch_track) = scratch_track.split_3();
        self.inc_at(sentinel1);
        self.inc_at(sentinel2);
        if let Some(init) = init {
            init(self, scratch_track);
        }
        self.add_const_to_byte(counter, register.size as u8);
        self.loop_while(counter, |cpu| {
            cpu.dec_at(counter);
            cpu.goto(sentinel2);
            cpu.go_clear_sentinel_right(sentinel2);
            f(cpu, register.at(0), scratch_track);
            cpu.inc_at(sentinel2.get_shifted(1));
            cpu.goto(sentinel1);
            cpu.goto_sentinel_left(sentinel1);
        });
        self.goto(sentinel2);
        self.go_clear_sentinel_right(sentinel2);
        if let Some(fin) = fin {
            fin(self, scratch_track);
        }
        self.goto(sentinel1);
        self.go_clear_sentinel_left(sentinel1);
    }

    /// In callbacks, the frame will shift every time, so if you want to keep scratch data
    /// across iterations you need to shift it left at the end of every callback
    pub fn foreach_pos_of_binregister_rev(
        &mut self,
        register: BinRegister,
        scratch_track: ScratchTrack,
        init: Option<impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack)>,
        f: impl for<'a> FnOnce(&'a mut Cpu, Pos, ScratchTrack),
        fin: Option<impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack)>,
    ) {
        let (sentinel1, scratch_track) = scratch_track.split_1();
        self.dec_at(sentinel1);
        let scratch_track = scratch_track.get_split_scratch(register.size - 1);
        let (sentinel2, scratch_track) = scratch_track.split_1();
        if let Some(init) = init {
            init(self, scratch_track);
        }
        self.inc_at(sentinel2);
        self.loop_while(sentinel2, |cpu| {
            cpu.dec();
            f(cpu, register.at(register.size - 1), scratch_track);
            let cur_frame = cpu
                .cur_frame
                .expect("in foreach_byte: f() reset cur_frame!");
            cpu.cur_frame = Some(cur_frame + 1);
            cpu.inc_at(sentinel2);
        });
        if let Some(fin) = fin {
            fin(self, scratch_track);
        }
        self.now_if_we_were_at_a_wed_actually_be_at_b(sentinel2, sentinel1);
    }

    /// In callbacks, the frame will be at the same position every time
    /// so you can access other registers/scratch variables
    pub fn foreach_val_of_binregister(
        &mut self,
        register: BinRegister,
        scratch_track: ScratchTrack,
        f: impl for<'a> FnOnce(&'a mut Cpu, Pos, ScratchTrack),
    ) {
        let ([_zero, sentinel1], scratch_track) = scratch_track.split_2();
        let scratch_track = scratch_track.get_split_scratch(register.size - 1);
        let ([sentinel2, val], scratch_track) = scratch_track.split_2();
        self.inc_at(sentinel2);
        self.dec_at(sentinel1);
        self.loop_while(sentinel1, |cpu| {
            cpu.inc();
            let backup_pos = sentinel1.get_shifted(-1);
            cpu.copy_byte(register.at(0), backup_pos, sentinel1);
            cpu.loop_while(register.at(0), |cpu| {
                cpu.dec();
                cpu.dec_at(sentinel1);
                cpu.goto(sentinel1.get_shifted(1));
                cpu.goto_sentinel_right(sentinel2);
                cpu.inc_at(val);
                cpu.goto(sentinel2.get_shifted(-1));
                cpu.go_clear_downsentinel_left(sentinel1);
            });
            cpu.moveadd_byte(backup_pos, register.at(0));
            cpu.dec_at(sentinel1);
            cpu.goto(sentinel1.get_shifted(1));
            cpu.goto_sentinel_right(sentinel2);
            f(cpu, val, scratch_track);
            cpu.clr_at(val);
            cpu.goto(sentinel2.get_shifted(-1));
            cpu.go_clear_downsentinel_left(sentinel1);
            cpu.dec_at(sentinel1.get_shifted(1));
            cpu.goto(sentinel1.get_shifted(1));
            cpu.cur_frame = Some(sentinel1.frame);
        });
        self.cur_frame = Some(sentinel2.frame);
    }

    /// In callbacks, the frame will be at the same position every time
    /// so you can access other registers/scratch variables
    pub fn foreach_val_of_binregister_rev(
        &mut self,
        register: BinRegister,
        scratch_track: ScratchTrack,
        f: impl for<'a> FnOnce(&'a mut Cpu, Pos, ScratchTrack),
    ) {
        let (sentinel1, scratch_track) = scratch_track.split_1();
        let scratch_track = scratch_track.get_split_scratch(register.size - 1);
        let ([sentinel2, _zero, val], scratch_track) = scratch_track.split_3();
        self.inc_at(sentinel1);
        self.dec_at(sentinel2);
        self.loop_while(sentinel2, |cpu| {
            cpu.inc();
            let backup_pos = sentinel2.get_shifted(1);
            cpu.copy_byte(register.last_pos(), backup_pos, sentinel2);
            cpu.loop_while(register.last_pos(), |cpu| {
                cpu.dec();
                cpu.dec_at(sentinel2);
                cpu.goto(sentinel2.get_shifted(-1));
                cpu.goto_sentinel_left(sentinel1);
                cpu.inc_at(val);
                cpu.goto(sentinel1.get_shifted(1));
                cpu.go_clear_downsentinel_right(sentinel2);
            });
            cpu.moveadd_byte(backup_pos, register.last_pos());
            cpu.dec_at(sentinel2);
            cpu.goto(sentinel2.get_shifted(-1));
            cpu.goto_sentinel_left(sentinel1);
            f(cpu, val, scratch_track);
            cpu.clr_at(val);
            cpu.goto(sentinel1.get_shifted(1));
            cpu.go_clear_downsentinel_right(sentinel2);
            cpu.dec_at(sentinel2.get_shifted(-1));
            cpu.goto(sentinel2.get_shifted(-1));
            cpu.cur_frame = Some(sentinel2.frame);
        });
        self.cur_frame = Some(sentinel1.frame);
    }

    pub fn clr_binregister(&mut self, register: BinRegister, scratch_track: ScratchTrack) {
        self.foreach_pos_of_binregister(
            register,
            scratch_track,
            None::<fn(&mut Cpu, ScratchTrack)>,
            |cpu, pos, _| {
                cpu.clr_at(pos);
            },
            None::<fn(&mut Cpu, ScratchTrack)>,
        )
    }

    pub fn set_binregister(
        &mut self,
        register: BinRegister,
        val: BigUint,
        scratch_track: ScratchTrack,
    ) {
        self.clr_binregister(register, scratch_track);
        let two = BigUint::from(2u64);
        let zero = BigUint::zero();
        let mut div = val;
        let mut i = 0;
        while &div != &zero {
            if i >= register.size {
                panic!("Value too big to fit in register");
            }
            let (new_div, rem) = div.div_rem(&two);
            div = new_div;
            let rem_bytes = rem.to_bytes_be();
            assert_eq!(rem_bytes.len(), 1);
            let rem = rem_bytes.last().unwrap();
            self.add_const_to_byte(register.at(register.size - i - 1), *rem);
            i += 1;
        }
    }

    pub fn inc_binregister_raw(
        &mut self,
        bin_register: BinRegister,
        result_carry_pos: Option<Pos>,
        scratch_track: ScratchTrack,
    ) {
        let (carry, scratch_track) = scratch_track.split_1();
        self.inc_at(carry);
        for i in (0..bin_register.size).rev() {
            self.if_nonzero_else(
                carry,
                scratch_track,
                |cpu, scratch_track| {
                    cpu.clr_at(carry);
                    cpu.if_nonzero_else(
                        bin_register.at(i),
                        scratch_track,
                        |cpu, _scratch_track| {
                            cpu.clr_at(bin_register.at(i));
                            cpu.inc_at(carry);
                        },
                        |cpu, _scratch_track| {
                            cpu.inc_at(bin_register.at(i));
                        },
                    );
                },
                |_, _| {},
            );
        }
        if let Some(result_carry_pos) = result_carry_pos {
            self.moveadd_byte(carry, result_carry_pos);
        } else {
            self.clr_at(carry);
        }
    }

    pub fn inc_binregister(&mut self, bin_register: BinRegister, scratch_track: ScratchTrack) {
        let ([byte_backup, sentinel1], scratch_track) = scratch_track.split_2();
        let scratch_track = scratch_track.get_split_scratch(bin_register.size + 1);
        let ([new_carry, carry], scratch_track) = scratch_track.split_2();
        self.moveadd_byte(bin_register.at_unchecked(-1), byte_backup);
        self.inc_at(sentinel1);
        self.inc_at(carry);
        self.loop_while(carry, |cpu| {
            cpu.dec();
            cpu.if_nonzero_else(
                bin_register.at(bin_register.size - 1),
                scratch_track,
                |cpu, _scratch_track| {
                    cpu.clr_at(bin_register.at(bin_register.size - 1));
                    cpu.inc_at(new_carry);
                },
                |cpu, _scratch_track| {
                    cpu.inc_at(bin_register.at(bin_register.size - 1));
                },
            );
            cpu.goto(new_carry);
            cpu.cur_frame = Some(carry.frame);
        });
        self.go_clear_sentinel_left(sentinel1);
        self.moveadd_byte(byte_backup, bin_register.at_unchecked(-1));
    }

    pub fn move_unpack_byte_onto_zeros(
        &mut self,
        byte_pos: Pos,
        result_pos: Pos,
        scratch_track: ScratchTrack,
    ) -> BinRegister {
        let result_register = BinRegister {
            track: Track {
                track_num: result_pos.track,
            },
            size: 8,
            offset: result_pos.frame,
        };
        self.loop_while(byte_pos, |cpu| {
            cpu.dec_at(byte_pos);
            cpu.inc_binregister(result_register, scratch_track);
        });
        result_register
    }

    pub fn unpack_register_onto_zeros(
        &mut self,
        register: Register,
        result_register: BinRegister,
        scratch_track: ScratchTrack,
    ) -> BinRegister {
        assert_eq!(register.size * 8, result_register.size);
        let (scratch_pos, scratch_track) = scratch_track.split_1();
        for i in 0..register.size {
            self.copy_byte_autoscratch(register.at(i), scratch_pos, scratch_track);
            self.move_unpack_byte_onto_zeros(scratch_pos, result_register.at(i * 8), scratch_track);
        }
        result_register
    }

    pub fn print_binregister_in_binary(
        &mut self,
        binregister: BinRegister,
        scratch_track: ScratchTrack,
    ) {
        self.print_text("0b", scratch_track);
        self.foreach_pos_of_binregister(
            binregister,
            scratch_track,
            None::<fn(&mut Cpu, ScratchTrack)>,
            |cpu, pos, scratch_track| {
                let ([prnt, scratch1], _scratch_track) = scratch_track.split_2();
                cpu.copy_byte(pos, prnt, scratch1);
                cpu.add_const_to_byte(prnt, '0' as u8);
                cpu.out();
                cpu.clr();
            },
            None::<fn(&mut Cpu, ScratchTrack)>,
        );
    }

    pub fn if_binregister_nonzero_else(
        &mut self,
        register: BinRegister,
        scratch_track: ScratchTrack,
        if_nonzero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
        if_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
    ) {
        let ([acc, sentinel1], scratch_track) = scratch_track.split_2();
        self.inc_at(sentinel1);
        let scratch_track = scratch_track.get_split_scratch(register.size - 1);
        let (sentinel2, scratch_track) = scratch_track.split_1();
        self.inc_at(sentinel2);
        self.loop_while(sentinel2, |cpu| {
            cpu.dec();
            let new_sentinel2 = sentinel2.get_shifted(-1);
            cpu.if_nonzero_else(
                register.at(register.size - 1),
                scratch_track,
                |cpu, _| {
                    cpu.inc_at(sentinel2);
                    cpu.goto(new_sentinel2);
                    cpu.goto_sentinel_left(sentinel1);
                    cpu.inc_at(acc);
                    cpu.goto(sentinel1.get_shifted(1));
                    cpu.go_clear_sentinel_right(sentinel2);
                },
                |_, _| {},
            );
            cpu.goto(new_sentinel2);
            cpu.cur_frame = Some(sentinel2.frame);
            cpu.not(sentinel2, scratch_track);
        });
        self.cur_frame = Some(sentinel1.frame);
        self.if_nonzero_else(acc, scratch_track, if_nonzero, if_zero);
        self.clr_at(acc);
    }

    pub fn add_binregister_to_binregister(
        &mut self,
        reg1: BinRegister,
        reg2: BinRegister,
        scratch_track: ScratchTrack,
    ) {
        assert_eq!(reg1.size, reg2.size);
        self.foreach_pos_of_binregister_rev(
            reg1,
            scratch_track,
            None::<fn(&mut Cpu, ScratchTrack)>,
            |cpu, pos, scratch_track| {
                let (carry, scratch_track) = scratch_track.split_1();
                cpu.copy_byte_autoscratch(pos, carry, scratch_track);
                cpu.copy_byte_autoscratch(reg2.last_pos(), carry, scratch_track);
                cpu.if_nonzero_else(
                    carry,
                    scratch_track,
                    |cpu, scratch_track| {
                        cpu.dec_at(carry);
                        cpu.if_nonzero_else(
                            carry,
                            scratch_track,
                            |cpu, scratch_track| {
                                cpu.dec_at(carry);
                                let new_carry = carry.get_shifted(-1);
                                cpu.set_byte(new_carry, 1);
                                cpu.if_nonzero_else(
                                    carry,
                                    scratch_track,
                                    |cpu, _| {
                                        cpu.dec_at(carry);
                                        cpu.set_byte(reg2.last_pos(), 1);
                                    },
                                    |cpu, _| {
                                        cpu.set_byte(reg2.last_pos(), 0);
                                    },
                                );
                            },
                            |cpu, _| {
                                cpu.set_byte(reg2.last_pos(), 1);
                            },
                        );
                    },
                    |cpu, _| {
                        cpu.set_byte(reg2.last_pos(), 0);
                    },
                );
            },
            Some(|cpu: &mut Cpu, scratch_track: ScratchTrack| {
                let (carry, _) = scratch_track.split_1();
                cpu.clr_at(carry);
            }),
        );
    }

    pub fn sub_binregister_from_binregister(
        &mut self,
        reg1: BinRegister,
        reg2: BinRegister,
        scratch_track: ScratchTrack,
    ) {
        assert_eq!(reg1.size, reg2.size);
        self.foreach_pos_of_binregister_rev(
            reg1,
            scratch_track,
            None::<fn(&mut Cpu, ScratchTrack)>,
            |cpu, pos, scratch_track| {
                let ([carry, acc], scratch_track) = scratch_track.split_2();
                cpu.copy_byte_autoscratch(reg2.last_pos(), acc, scratch_track);
                cpu.add_const_to_byte(acc, 2);
                cpu.sub_byte_autoscratch(pos, acc, scratch_track);
                cpu.movesub_byte(carry, acc);
                let new_carry = carry.get_shifted(-1);
                cpu.if_nonzero_else(
                    acc,
                    scratch_track,
                    |cpu, scratch_track| {
                        cpu.dec_at(acc);
                        cpu.if_nonzero_else(
                            acc,
                            scratch_track,
                            |cpu, scratch_track| {
                                cpu.dec_at(acc);
                                cpu.if_nonzero_else(
                                    acc,
                                    scratch_track,
                                    |cpu, _| {
                                        cpu.dec_at(acc);
                                        cpu.set_byte(reg2.last_pos(), 1);
                                    },
                                    |cpu, _| {
                                        cpu.set_byte(reg2.last_pos(), 0);
                                    },
                                );
                            },
                            |cpu, _| {
                                cpu.set_byte(new_carry, 1);
                                cpu.set_byte(reg2.last_pos(), 1);
                            },
                        );
                    },
                    |cpu, _| {
                        cpu.set_byte(new_carry, 1);
                        cpu.set_byte(reg2.last_pos(), 0);
                    },
                );
            },
            Some(|cpu: &mut Cpu, scratch_track: ScratchTrack| {
                let (carry, _) = scratch_track.split_1();
                cpu.clr_at(carry);
            }),
        );
    }

    pub fn shift_binregister_left(&mut self, register: BinRegister, scratch_track: ScratchTrack) {
        self.foreach_pos_of_binregister_rev(
            register,
            scratch_track,
            None::<fn(&mut Cpu, ScratchTrack)>,
            |cpu, pos, scratch_track| {
                let (cpy, _) = scratch_track.split_1();
                let newcpy = cpy.get_shifted(-1);
                cpu.moveadd_byte(pos, newcpy);
                cpu.moveadd_byte(cpy, pos);
            },
            Some(|cpu: &mut Cpu, scratch_track: ScratchTrack| {
                let (cpy, _) = scratch_track.split_1();
                cpu.clr_at(cpy);
            }),
        );
    }

    pub fn shift_binregister_right(&mut self, register: BinRegister, scratch_track: ScratchTrack) {
        self.foreach_pos_of_binregister(
            register,
            scratch_track,
            None::<fn(&mut Cpu, ScratchTrack)>,
            |cpu, pos, scratch_track| {
                let (cpy, _) = scratch_track.split_1();
                let newcpy = cpy.get_shifted(1);
                cpu.moveadd_byte(pos, newcpy);
                cpu.moveadd_byte(cpy, pos);
            },
            Some(|cpu: &mut Cpu, scratch_track: ScratchTrack| {
                let (cpy, _) = scratch_track.split_1();
                cpu.clr_at(cpy);
            }),
        );
    }

    pub fn copy_binregister(
        &mut self,
        from: BinRegister,
        to: BinRegister,
        scratch_track: ScratchTrack,
        clear_to_first: bool,
    ) {
        self.foreach_pos_of_binregister(
            from,
            scratch_track,
            None::<fn(&mut Cpu, ScratchTrack)>,
            |cpu, pos, scratch_track| {
                if clear_to_first {
                    cpu.clr_at(to.at(0));
                }
                cpu.copy_byte_autoscratch(pos, to.at(0), scratch_track);
            },
            None::<fn(&mut Cpu, ScratchTrack)>,
        );
    }
    pub fn cmp_binregister(
        &mut self,
        register: BinRegister,
        scratch_track: ScratchTrack,
        if_lt_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
        if_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
        if_gt_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
    ) {
        self.if_nonzero_else(
            register.at(0),
            scratch_track,
            |cpu, scratch_track| if_lt_zero(cpu, scratch_track),
            |cpu, scratch_track| {
                cpu.if_binregister_nonzero_else(register, scratch_track, if_gt_zero, if_zero)
            },
        );
    }

    /// Adds a*b to out
    pub fn mul_binregisters(
        &mut self,
        a: BinRegister,
        b: BinRegister,
        out: BinRegister,
        scratch_track: ScratchTrack,
    ) {
        assert!(b.offset != out.offset || b.track != out.track);
        let (a_shifted, scratch_track) = scratch_track.split_binregister(a.size);
        self.copy_binregister(a, a_shifted, scratch_track, false);
        self.foreach_val_of_binregister_rev(b, scratch_track, |cpu, val, scratch_track| {
            cpu.if_nonzero_else(
                val,
                scratch_track,
                |cpu, scratch_track| {
                    cpu.add_binregister_to_binregister(a_shifted, out, scratch_track);
                },
                |_, _| {},
            );
            cpu.shift_binregister_left(a_shifted, scratch_track);
        });
    }

    /// Adds a/b to out
    pub fn div_binregisters(
        &mut self,
        a: BinRegister,
        b: BinRegister,
        div: BinRegister,
        rem: BinRegister,
        scratch_track: ScratchTrack,
    ) {
        assert_eq!(a.size, b.size);
        let w = a.size;
        let (rem2, scratch_track) = scratch_track.split_binregister(w * 2);
        self.copy_binregister(a, rem2.subview(w, w), scratch_track, false);
        let (b_shifted, scratch_track) = scratch_track.split_binregister(w * 2);
        self.copy_binregister(b, b_shifted.subview(1, w), scratch_track, false);
        let (cur_digit, scratch_track) = scratch_track.split_binregister(w);
        self.inc_at(cur_digit.at(0));
        let (counter, scratch_track) = scratch_track.split_1();
        self.set_byte(counter, w as u8);
        self.loop_while(counter, |cpu| {
            cpu.dec();
            cpu.sub_binregister_from_binregister(b_shifted, rem2, scratch_track);
            cpu.cmp_binregister(
                rem2,
                scratch_track,
                |cpu, scratch_track| {
                    cpu.add_binregister_to_binregister(b_shifted, rem2, scratch_track);
                },
                |cpu, scratch_track| {
                    cpu.add_binregister_to_binregister(cur_digit, div, scratch_track);
                },
                |cpu, scratch_track| {
                    cpu.add_binregister_to_binregister(cur_digit, div, scratch_track);
                },
            );
            cpu.shift_binregister_right(b_shifted, scratch_track);
            cpu.shift_binregister_right(cur_digit, scratch_track);
        });
        self.add_binregister_to_binregister(rem2.subview(w, w), rem, scratch_track);
        self.clr_binregister(rem2, scratch_track);
        self.clr_binregister(b_shifted, scratch_track);
        self.clr_at(cur_digit.last_pos());
    }

    /*/// b -= a
    /// carry = 1 if b < a
    pub fn movesub_byte_with_carry(&mut self, a: Pos, b: Pos, carry: Pos, scratch: Pos) {
        self.loop_while(a, |cpu| {
            cpu.dec();
            cpu.inc_at(carry);
            cpu.inc_at(b);
            cpu.loop_while(b, |cpu| {
                cpu.moveadd_byte(b, scratch);
                cpu.dec_at(carry);
            });
            cpu.moveadd_byte(scratch, b);
        });
    }

    pub fn movesub_registers_autoscratch(&mut self, a: &Register, b: &Register) {
        assert_eq!(a.size, b.size);
        for i in (0..a.size).rev() {
            if i > 0 {
                self.moveadd_byte_with_carry(a.at(i), b.at(i), self.cfg.scratch_at(i-1), self.cfg.scratch_at(i));
                if i > 1 {
                    self.moveadd_byte_with_carry(self.cfg.scratch_at(i-1), b.at(i-1), self.cfg.scratch_at(i-2), self.cfg.scratch_at(i));
                } else {
                    self.moveadd_byte(self.cfg.scratch_at(i-1), b.at(i-1));
                }
            } else {
                self.moveadd_byte(a.at(i), b.at(i));
            }
        }
    }

    pub fn is_byte_le_x(&mut self, pos: Pos, result: Pos, scratch: Pos, x: isize) {
        if x < 0 {
            return;
        }
        fn rec<'c>(cpu: &mut Cpu<'c>, pos: Pos, result: Pos, scratch: Pos, x: isize) {
            cpu.loop_while(pos, |cpu| {
                if x > 0 {
                    cpu.dec();
                    cpu.inc_at(scratch);
                    rec(cpu, pos, result, scratch, x-1);
                } else {
                    cpu.moveadd_byte(pos, scratch);
                    cpu.dec_at(result);
                }
            });
        }
        self.inc_at(result);
        rec(self, pos, result, scratch, x);
        self.moveadd_byte(scratch, pos);
    }

    /*pub fn movesub_byte_with_carry(&mut self, a: Pos, b: Pos, carry: Pos, scratch: Pos) {
        self.loop_while(a, |cpu| {
            cpu.dec();
            cpu.goto(carry);
            cpu.dec();
            cpu.goto(b);
            cpu.dec();
            cpu.loop_while(b, |cpu| {
                cpu.goto(carry);
                cpu.dec();
                cpu.moveadd_byte(b, scratch);
            });
            cpu.moveadd_byte(scratch, b);
        });
    }*/*/
}
