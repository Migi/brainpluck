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

#[derive(Eq,PartialEq,Copy,Clone)]
pub struct Pos {
	pub frame: isize,
	pub track: isize
}

impl Pos {
	fn index(&self, cfg: &CpuConfig) -> isize {
		self.frame * cfg.frame_size() + self.track
	}
}

#[derive(Clone,Copy)]
pub enum TrackKind {
	Data(Track),
	Register(Register),
	Scratch(ScratchTrack)
}

#[derive(Hash,Eq,PartialEq,Clone,Copy)]
pub enum TrackId {
	Stack = 0,
	Heap,
	Scratch1,
	Scratch2,
	CurDataPtr,
}

#[derive(Clone,Copy,Eq,PartialEq)]
pub struct Track {
	pub track_num: isize,
}

impl Track {
	fn at(&self, frame: isize) -> Pos {
		Pos {
			frame,
			track: self.track_num,
		}
	}
}

#[derive(Clone,Copy)]
pub struct Register {
	pub track: Track,
	pub size: isize,
}

impl Register {
	fn at(&self, frame: isize) -> Pos {
		self.track.at(frame)
	}
}

#[derive(Clone,Copy,Eq,PartialEq)]
pub struct ScratchTrack {
	pub track: Track
}

impl ScratchTrack {
	fn at(&self, frame: isize) -> Pos {
		self.track.at(frame)
	}
}

fn all_different<T: PartialEq>(elements: &[T]) -> bool {
	for i in 0..elements.len() {
		for j in i+1..elements.len() {
			if elements[i] == elements[j] {
				return false;
			}
		}
	}
	true
}

#[derive(Clone,Copy)]
pub struct CpuConfig {
	num_tracks: isize,
}

impl CpuConfig {
	fn frame_size(&self) -> isize {
		self.num_tracks
	}
}

pub struct Cpu<'c> {
	cfg: CpuConfig,
	tracks: &'c mut HashMap<TrackId, TrackKind>,
	cur_track: isize,
	cur_frame: Option<isize>,
	lir: Vec<Lir>
}

impl<'c> Cpu<'c> {
	pub fn new(tracks: &'c mut HashMap<TrackId, TrackKind>) -> Cpu<'c> {
		let mut cpu = Cpu {
			cfg: CpuConfig {
				num_tracks: (tracks.len() as isize)
			},
			tracks,
			cur_track: 0,
			cur_frame: Some(0),
			lir: Vec::new()
		};
		for i in 0..(cpu.cfg.num_tracks*3) {
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

	/*pub fn inc_by(&mut self, x: isize) {
		if x > 0 {
			for i in 0..x {
				self.lir.push(Lir::Inc);
			}
		} else {
			for i in 0..x.abs() {
				self.lir.push(Lir::Dec);
			}
		}
	}*/

	pub fn shift_cursor_untracked(&mut self, shift: isize) {
		if shift < 0 {
			for i in 0..shift.abs() {
				self.lir.push(Lir::Left);
			}
		} else {
			for i in 0..shift {
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
			None => panic!("Trying to unwrap cur_frame but it's None")
		}
	}

	pub fn goto_frame(&mut self, frame: isize) {
		let cur_frame = self.unwrap_cur_frame();
		self.shift_cursor_untracked((frame - cur_frame) * self.cfg.frame_size());
		self.cur_frame = Some(frame);
	}

	pub fn goto(&mut self, pos: Pos) {
		let cur_frame = self.unwrap_cur_frame();
		self.shift_cursor_untracked((pos.track - self.cur_track) + (pos.frame - cur_frame) * self.cfg.frame_size());
		self.cur_frame = Some(pos.frame);
		self.cur_track = pos.track;
	}

	pub fn loop_while(&mut self, at: Pos, f: impl for<'a> FnOnce(&'a mut Cpu)) {
		self.goto(at);
		let mut cpu = Cpu {
			cfg: self.cfg,
			tracks: self.tracks,
			cur_track: self.cur_track,
			cur_frame: self.cur_frame,
			lir: Vec::new()
		};
		f(&mut cpu);
		cpu.goto(at);
		self.lir.push(Lir::Loop(cpu.lir));
	}

	pub fn raw_loop(&mut self, f: impl for<'a> FnOnce(&'a mut Cpu)) {
		let mut cpu = Cpu {
			cfg: self.cfg,
			tracks: self.tracks,
			cur_track: self.cur_track,
			cur_frame: self.cur_frame,
			lir: Vec::new()
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
			(a.index(&self.cfg) - p.index(&self.cfg)).abs() + (b.index(&self.cfg) - p.index(&self.cfg)).abs()
		};
		let mut best_pos = track.at(a.frame);
		let mut best_score = eval(best_pos);
		for i in std::cmp::min(a.frame, b.frame)..=std::cmp::max(a.frame, b.frame)+1 {
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

	pub fn add_const_to_byte(&mut self, pos: Pos, val: u8) {
		for i in 0..val {
			self.inc_at(pos);
		}
	}

	pub fn zero_slice(&mut self, slice: Pos, size: isize) {
		for i in 0..size {
			self.zero_byte(Pos {
				frame: slice.frame + i,
				track: slice.track
			});
		}
	}

	pub fn zero_register(&mut self, register: Register) {
		self.zero_slice(
			register.at(0),
			register.size
		);
	}

	pub fn clear_register_track_to_scratch_track(&mut self, register: Register) -> ScratchTrack {
		self.zero_register(register);
		ScratchTrack {
			track: register.track
		}
	}

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
		assert!(from != to);
		self.loop_while(from, |cpu| {
			cpu.dec();
			cpu.dec_at(to);
		});
	}

	pub fn move_slice_onto_zero_slice(&mut self, slice: Pos, size: isize, to: Pos) {
		if slice == to {
			return;
		}
		if slice.frame < to.frame {
			for i in 0..size {
				self.moveadd_byte(Pos {
					frame: slice.frame + i,
					track: slice.track
				}, Pos {
					frame: to.frame + i,
					track: to.track
				});
			}
		} else {
			for i in (0..size).rev() {
				self.moveadd_byte(Pos {
					frame: slice.frame + i,
					track: slice.track
				}, Pos {
					frame: to.frame + i,
					track: to.track
				});
			}
		}
	}

	pub fn move_onto_zero_register(&mut self, from: Register, to: Register) {
		assert_eq!(from.size, to.size);
		self.move_slice_onto_zero_slice(
			from.at(0),
			from.size,
			to.at(0)
		);
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
						track: slice.track
					}, Pos {
						frame: to.frame + i,
						track: to.track
					},
					scratch_track
				);
			}
		} else {
			for i in (0..size).rev() {
				self.copy_byte_autoscratch(
					Pos {
						frame: slice.frame + i,
						track: slice.track
					}, Pos {
						frame: to.frame + i,
						track: to.track
					},
					scratch_track
				);
			}
		}
	}

	pub fn copy_register(&mut self, from: Register, to: Register, scratch_track: ScratchTrack) {
		assert_eq!(from.size, to.size);
		self.copy_slice(
			from.at(0),
			from.size,
			to.at(0),
			scratch_track
		);
	}

	pub fn moveadd_byte_with_carry_slow(&mut self, a: Pos, b: Pos, carry: Pos, scratch: Pos) {
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

	pub fn moveadd_registers_slow(&mut self, a: Register, b: Register, scratch_track: ScratchTrack) {
		assert_eq!(a.size, b.size);
		for i in (0..a.size).rev() {
			if i > 0 {
				self.moveadd_byte_with_carry_slow(a.at(i), b.at(i), scratch_track.at(i-1), scratch_track.at(i));
				if i > 1 {
					self.moveadd_byte_with_carry_slow(scratch_track.at(i-1), b.at(i-1), scratch_track.at(i-2), scratch_track.at(i));
				} else {
					self.moveadd_byte(scratch_track.at(i-1), b.at(i-1));
				}
			} else {
				self.moveadd_byte(a.at(i), b.at(i));
			}
		}
	}

	pub fn moveadd_registers(&mut self, a: Register, b: Register, scratch_track1: ScratchTrack, scratch_track2: ScratchTrack) {
		assert_eq!(a.size, b.size);
		assert!(all_different(&[a.track, b.track, scratch_track1.track, scratch_track2.track]));

		let sentinel_track = scratch_track1;
		let carry_track = scratch_track2;

		self.inc_at(sentinel_track.at(-2));
		for i in (1..a.size).rev() {
			self.inc_at(sentinel_track.at(i));
			self.loop_while(a.at(i), |cpu| {
				cpu.dec();
				let x = i; // x starts at i, then keeps shifting left
				cpu.inc_at(carry_track.at(x));
				cpu.loop_while(carry_track.at(x), |cpu| {
					cpu.dec();
					cpu.inc_at(carry_track.at(x-1));
					cpu.inc_at(b.at(x));
					cpu.raw_loop(|cpu| {
						cpu.dec_at(carry_track.at(x-1));
						cpu.goto(sentinel_track.at(x-1));
						cpu.go_clear_sentinel_left();
						cpu.inc();
						cpu.goto_track(b.track.track_num);
					});
					cpu.shift_frame_untracked(-1);
					cpu.cur_frame = Some(x);
				});
				cpu.goto(sentinel_track.at(x+1));
				cpu.go_clear_sentinel_right();
				cpu.inc();
				cpu.cur_frame = Some(i);
			});
			self.dec_at(sentinel_track.at(i));
		}
		self.moveadd_byte(a.at(0), b.at(0));
		self.zero_byte(b.at(-1));
		self.dec_at(sentinel_track.at(-2));
	}

	pub fn movediv_byte_onto_zeros(&mut self, a: Pos, divisor: u8, div_result: Pos, rem_result: Pos, scratch_track: ScratchTrack) {
		// scratch structure:
		// - 0: divisor (kind of)
		// - 1: remainder
		// - 2: always 0
		// - 3: always 0
		assert_ne!(divisor, 0);
		assert_ne!(divisor, 1);
		self.add_const_to_byte(scratch_track.at(0), divisor-1);
		self.loop_while(a, |cpu| {
			cpu.dec();
			cpu.goto(scratch_track.at(0));
			cpu.raw_loop(|cpu| {
				// we must be at 0
				assert_eq!(cpu.cur_frame, Some(0));
				cpu.dec();
				cpu.inc_at(scratch_track.at(1));
				cpu.goto(scratch_track.at(2));
			});
			assert!(cpu.cur_frame.is_none()); // could be 0 or 2
			cpu.shift_frame_untracked(1); // we're at 1 or 3
			cpu.raw_loop(|cpu| {
				// we must be at 1, divisor == 0, remainder > 0
				cpu.cur_frame = Some(1);
				cpu.moveadd_byte(scratch_track.at(1), scratch_track.at(0));
				cpu.inc_at(div_result);
				cpu.goto(scratch_track.at(3));
			});
			cpu.cur_frame = Some(3);
		});
		self.moveadd_byte(scratch_track.at(1), rem_result);
		self.zero_byte(scratch_track.at(0));
	}

	pub fn moveprint_byte(&mut self, pos: Pos, result_scratch_track: ScratchTrack, division_internal_scratch_track: ScratchTrack) {
		let singles = result_scratch_track.at(0);
		let temp = result_scratch_track.at(1);
		let tens = result_scratch_track.at(2);
		let hundreds = result_scratch_track.at(3);
		self.movediv_byte_onto_zeros(pos, 10, temp, singles, division_internal_scratch_track);
		self.movediv_byte_onto_zeros(temp, 10, hundreds, tens, division_internal_scratch_track);
		self.add_const_to_byte(hundreds, 48);
		self.out();
		self.clr();
		self.add_const_to_byte(tens, 48);
		self.out();
		self.clr();
		self.add_const_to_byte(singles, 48);
		self.out();
		self.clr();
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