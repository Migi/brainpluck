use num::BigUint;
use num::Integer;
use num::Zero;
use std::collections::HashMap;

pub enum Lir {
    Left,
    Right,
    Inc,
    Dec,
    In,
    Out,
    Loop(Vec<Lir>),
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
}

#[derive(Clone, Copy)]
pub enum TrackKind {
    Data(Track),
    Register(Register),
    Scratch(ScratchTrack),
}

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub enum TrackId {
    Stack = 0,
    Heap,
    Scratch1,
    Scratch2,
    Scratch3,
    CurDataPtr,
    Register1,
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
    pub offset: isize
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
    pub offset: isize
}

impl BinRegister {
    pub fn at(&self, frame: isize) -> Pos {
        assert!(frame >= 0);
        assert!(frame < self.size);
        self.track.at(frame + self.offset)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ScratchTrack {
    pub track: Track,
    pub offset: isize, // shift all accesses by this amount
    pub dont_go_left_of: Option<isize>, // don't access left of this position (at offset=0)
}

impl ScratchTrack {
    pub fn at(&self, frame: isize) -> Pos {
        if let Some(l) = self.dont_go_left_of {
            if frame + self.offset < l {
                panic!("Going left of scratchtrack's limits! {}+{} < {}", frame, self.offset, l);
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

    fn get_split_scratch(&self, num_poss_split: isize) -> ScratchTrack {
        ScratchTrack {
            track: self.track,
            dont_go_left_of: Some(self.dont_go_left_of.unwrap_or(0) + num_poss_split),
            offset: self.offset
        }
    }

    #[allow(unused)]
    pub fn split_1(self) -> (Pos, ScratchTrack) {
        (
            self.at(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(1)
        )
    }

    #[allow(unused)]
    pub fn split_2(self) -> ([Pos; 2], ScratchTrack) {
        (
            self.get_2_pos(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(2)
        )
    }

    #[allow(unused)]
    pub fn split_3(self) -> ([Pos; 3], ScratchTrack) {
        (
            self.get_3_pos(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(3)
        )
    }

    #[allow(unused)]
    pub fn split_4(self) -> ([Pos; 4], ScratchTrack) {
        (
            self.get_4_pos(self.dont_go_left_of.unwrap_or(0) - self.offset),
            self.get_split_scratch(4)
        )
    }

    pub fn shift_so_frame_is_legal(&mut self, frame: isize) {
        if let Some(l) = self.dont_go_left_of {
            if frame + self.offset < l {
                self.offset = l - frame;
            }
        }
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
                track_num: self.tracks.len() as isize
            },
            offset: 0,
            dont_go_left_of: None
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
            offset: 0
        };
        let old = self.tracks.insert(id, TrackKind::Register(track));
        assert!(old.is_none());
        track
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

    pub fn into_ops(self) -> Vec<Lir> {
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

    pub fn go_clear_sentinel_left(&mut self) {
        self.dec();
        self.raw_loop(|cpu| {
            cpu.inc();
            cpu.shift_frame_untracked(-1);
        });
    }

    pub fn go_clear_sentinel_right(&mut self) {
        self.dec();
        self.raw_loop(|cpu| {
            cpu.inc();
            cpu.shift_frame_untracked(1);
        });
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

    pub fn add_const_to_byte_with_carry_slow(&mut self, pos: Pos, val: u8, carry: Pos, scratch1: Pos, scratch2: Pos) {
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
            dont_go_left_of: None
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
            self.clr_at(to);
        }
        self.loop_while(from, |cpu| {
            cpu.dec();
            cpu.dec_at(to);
        });
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

    pub fn if_register_nonzero_else(
        &mut self,
        register: Register,
        scratch_track: ScratchTrack,
        if_nonzero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack),
        if_zero: impl for<'a> FnOnce(&'a mut Cpu, ScratchTrack)
    ) {
        let (acc, scratch_track) = scratch_track.split_1();
        for i in 0..register.size {
            self.if_nonzero_else(
                register.at(i),
                scratch_track,
                |cpu, _| {
                    cpu.inc_at(acc);
                },
                |_, _| {}
            );
        }
        self.if_nonzero_else(
            acc,
            scratch_track,
            if_nonzero,
            if_zero
        );
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
                cpu.if_register_nonzero_else(
                    register,
                    scratch_track,
                    if_gt_zero,
                    if_zero
                )
            }
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

    pub fn copy_byte_autoscratch(&mut self, from: Pos, to: Pos, scratch_track: ScratchTrack) {
        if from == to {
            return;
        }
        let mut scratch = self.get_pos_on_track_between(from, to, scratch_track.track);
        if let Some(l) = scratch_track.dont_go_left_of {
            if scratch.frame < l {
                scratch.frame = l;
            }
        }
        if from == scratch {
            scratch.frame += 1;
            if to == scratch {
                scratch.frame += 1;
            }
        } else if to == scratch {
            scratch.frame += 1;
            if from == scratch {
                scratch.frame += 1;
            }
        }
        self.copy_byte(from, to, scratch);
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
                        cpu.go_clear_sentinel_left();
                        cpu.inc();
                        cpu.goto_track(b.track.track_num);
                    });
                    cpu.shift_frame_untracked(-1);
                    cpu.cur_frame = Some(carry_track.at(x).frame);
                });
                cpu.goto(sentinel_track.at(x + 1));
                cpu.go_clear_sentinel_right();
                cpu.inc();
                cpu.cur_frame = Some(carry_track.at(i).frame);
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

    pub fn moveprint_hex_digit(
        &mut self,
        pos: Pos,
        scratch_track: ScratchTrack,
    ) {
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

    pub fn print_text(&mut self, s: &str, mut scratch_track: ScratchTrack) {
        scratch_track.shift_so_frame_is_legal(0);
        for c in s.chars() {
            self.print_char(c, scratch_track.at(0));
        }
    }

    pub fn moveprint_byte(
        &mut self,
        pos: Pos,
        scratch_track: ScratchTrack,
    ) {
        let ([singles, temp, tens, hundreds], div_scratch_track) = scratch_track.split_4();
        self.movediv_byte_onto_zeros(pos, 10, temp, singles, div_scratch_track);
        self.movediv_byte_onto_zeros(temp, 10, hundreds, tens, div_scratch_track);
        self.moveprint_digit_if_nonzero(hundreds);
        self.moveprint_digit_if_nonzero(tens);
        self.moveprint_digit(singles);
    }

    pub fn moveprint_register_hex(
        &mut self,
        register: Register,
        scratch_track: ScratchTrack
    ) {
        self.print_text("0x", scratch_track);
        for i in 0..register.size {
            let ([left, right], scratch_track) = scratch_track.split_2();
            self.movediv_byte_onto_zeros(register.at(i), 16, left, right, scratch_track);
            self.moveprint_hex_digit(left, scratch_track);
            self.moveprint_hex_digit(right, scratch_track);
        }
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

    pub fn add_const_to_register(&mut self, register: Register, val: BigUint, scratch_track: ScratchTrack) {
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
                self.add_const_to_byte_with_carry_slow(register.at(register.size - i - 1), *rem, carry, scratch1, scratch2);
                self.clr_at(carry);
            } else {
                self.add_const_to_byte_with_carry_slow(register.at(register.size - i - 1), *rem, register.at(register.size - i - 2), scratch1, scratch2);
            }
            i += 1;
        }
    }

    /*pub fn unpack_register(&mut self, register: Register, pos: Pos) -> BinRegister {

    }*/

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
