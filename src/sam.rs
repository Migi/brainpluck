use std::io::{Read, Write};
use crate::linker::*;

pub type SamVal = u32;
pub type SamIVal = i32;

pub const OPCODE_HALT : u8 = 0;
pub const OPCODE_SWAP_XY : u8 = 1;
pub const OPCODE_SWAP_AB : u8 = 2;
pub const OPCODE_SET_X : u8 = 3;
pub const OPCODE_SET_Y : u8 = 4;
pub const OPCODE_SET_A : u8 = 5;
pub const OPCODE_SET_B : u8 = 6;
pub const OPCODE_READ_A_AT_B : u8 = 7;
pub const OPCODE_READ_X_AT_B : u8 = 8;
pub const OPCODE_READ_Y_AT_B : u8 = 9;
pub const OPCODE_WRITE_A_AT_B : u8 = 10;
pub const OPCODE_WRITE_X_AT_B : u8 = 11;
pub const OPCODE_WRITE_Y_AT_B : u8 = 12;
pub const OPCODE_ADD_A_TO_B : u8 = 13;
pub const OPCODE_SUB_A_FROM_B : u8 = 14;
pub const OPCODE_PRINT_CHAR_X : u8 = 15;
pub const OPCODE_STDIN_X : u8 = 16;
pub const OPCODE_ADD_CONST_TO_B : u8 = 17;
pub const OPCODE_SUB_CONST_FROM_B : u8 = 18;
pub const OPCODE_PRINT_A : u8 = 19;
pub const OPCODE_CALL : u8 = 20;
pub const OPCODE_RET : u8 = 21;
pub const OPCODE_JUMP : u8 = 22;
pub const OPCODE_JUMP_IF_X : u8 = 23;
pub const OPCODE_ADD_U8_AT_B_TO_X : u8 = 24;
pub const OPCODE_MUL_U8_AT_B_TO_X : u8 = 25;
pub const OPCODE_ADD_U32_AT_B_TO_A : u8 = 26;
pub const OPCODE_MUL_U32_AT_B_TO_A : u8 = 27;
pub const OPCODE_NEG_A : u8 = 28;
pub const OPCODE_NEG_X : u8 = 29;
pub const OPCODE_MOVE_X_TO_A : u8 = 30;

#[derive(Debug, Copy, Clone)]
pub enum SamSOp {
	Halt,
	SwapXY,
	SwapAB,
	SetX(u8),
	SetY(u8),
	SetA(SamVal),
	SetB(SamVal),
	ReadAAtB,
	ReadXAtB,
	ReadYAtB,
	WriteAAtB,
	WriteXAtB,
	WriteYAtB,
	AddAToB,
	SubAFromB,
	PrintCharX,
	StdinX,
	AddConstToB(SamVal),
	SubConstFromB(SamVal),
	PrintA,
	Ret,
	AddU32AtBToA,
	MulU32AtBToA,
	AddU8AtBToX,
	MulU8AtBToX,
	NegA,
	NegX,
	MoveXToA
}

#[derive(Debug)]
pub enum SamOp {
	Simple(SamSOp),
	Call(SamVal),
	Jmp(SamIVal),
	JmpIfX(SamIVal)
}

impl SamSOp {
	pub fn encode(&self) -> Vec<u8> {
		match self {
			SamSOp::Halt => {
				vec![OPCODE_HALT]
			},
			SamSOp::SwapXY => {
				vec![OPCODE_SWAP_XY]
			},
			SamSOp::SwapAB => {
				vec![OPCODE_SWAP_AB]
			},
			SamSOp::SetX(val) => {
				vec![OPCODE_SET_X, *val]
			},
			SamSOp::SetY(val) => {
				vec![OPCODE_SET_Y, *val]
			},
			SamSOp::SetA(val) => {
				let mut res = vec![OPCODE_SET_A];
				push_u32_to_vec(&mut res, *val);
				res
			},
			SamSOp::SetB(val) => {
				let mut res = vec![OPCODE_SET_B];
				push_u32_to_vec(&mut res, *val);
				res
			},
			SamSOp::ReadAAtB => {
				vec![OPCODE_READ_A_AT_B]
			},
			SamSOp::ReadXAtB => {
				vec![OPCODE_READ_X_AT_B]
			},
			SamSOp::ReadYAtB => {
				vec![OPCODE_READ_Y_AT_B]
			},
			SamSOp::WriteAAtB => {
				vec![OPCODE_WRITE_A_AT_B]
			},
			SamSOp::WriteXAtB => {
				vec![OPCODE_WRITE_X_AT_B]
			},
			SamSOp::WriteYAtB => {
				vec![OPCODE_WRITE_Y_AT_B]
			},
			SamSOp::AddAToB => {
				vec![OPCODE_ADD_A_TO_B]
			},
			SamSOp::SubAFromB => {
				vec![OPCODE_SUB_A_FROM_B]
			},
			SamSOp::PrintCharX => {
				vec![OPCODE_PRINT_CHAR_X]
			},
			SamSOp::StdinX => {
				vec![OPCODE_STDIN_X]
			},
			SamSOp::AddConstToB(val) => {
				let mut res = vec![OPCODE_ADD_CONST_TO_B];
				push_u32_to_vec(&mut res, *val);
				res
			},
			SamSOp::SubConstFromB(val) => {
				let mut res = vec![OPCODE_SUB_CONST_FROM_B];
				push_u32_to_vec(&mut res, *val);
				res
			},
			SamSOp::PrintA => {
				vec![OPCODE_PRINT_A]
			},
			SamSOp::Ret => {
				vec![OPCODE_RET]
			},
			SamSOp::AddU8AtBToX => {
				vec![OPCODE_ADD_U8_AT_B_TO_X]
			},
			SamSOp::MulU8AtBToX => {
				vec![OPCODE_MUL_U8_AT_B_TO_X]
			},
			SamSOp::AddU32AtBToA => {
				vec![OPCODE_ADD_U32_AT_B_TO_A]
			},
			SamSOp::MulU32AtBToA => {
				vec![OPCODE_MUL_U32_AT_B_TO_A]
			},
			SamSOp::NegA => {
				vec![OPCODE_NEG_A]
			},
			SamSOp::NegX => {
				vec![OPCODE_NEG_X]
			},
			SamSOp::MoveXToA => {
				vec![OPCODE_MOVE_X_TO_A]
			},
		}
	}

	pub fn len(&self) -> usize {
		self.encode().len()
	}
}

impl SamOp {
	pub fn encode(&self) -> Vec<u8> {
		match self {
			SamOp::Simple(op) => op.encode(),
			SamOp::Call(c) => {
				let mut res = vec![OPCODE_CALL];
				push_u32_to_vec(&mut res, *c);
				res
			},
			SamOp::Jmp(offset) => {
				let mut res = vec![OPCODE_JUMP];
				push_samival_to_vec(&mut res, *offset);
				res
			},
			SamOp::JmpIfX(offset) => {
				let mut res = vec![OPCODE_JUMP_IF_X];
				push_samival_to_vec(&mut res, *offset);
				res
			}
		}
	}

	pub fn len(&self) -> usize {
		self.encode().len()
	}
}

fn push_u32_to_vec(vec: &mut Vec<u8>, val: u32) {
	let [val0, val1, val2, val3] = val.to_be_bytes();
	vec.push(val0);
	vec.push(val1);
	vec.push(val2);
	vec.push(val3);
}

fn push_samival_to_vec(vec: &mut Vec<u8>, val: SamIVal) {
	let [val0, val1, val2, val3] = val.to_be_bytes(); // TODO
	vec.push(val0);
	vec.push(val1);
	vec.push(val2);
	vec.push(val3);
}

fn write_u32(slice: &mut [u8], val: u32) {
	let [val0, val1, val2, val3] = val.to_be_bytes();
	slice[0] = val0;
	slice[1] = val1;
	slice[2] = val2;
	slice[3] = val3;
}

fn decode_u32(slice: &[u8]) -> u32 {
	u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]])
}

fn decode_samival(slice: &[u8]) -> i32 {
	i32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]) // TODO
}

fn decode_sam_op(slice: &[u8]) -> SamOp {
	match slice[0] {
		OPCODE_HALT => SamOp::Simple(SamSOp::Halt),
		OPCODE_SWAP_XY => SamOp::Simple(SamSOp::SwapXY),
		OPCODE_SWAP_AB => SamOp::Simple(SamSOp::SwapAB),
		OPCODE_SET_X => SamOp::Simple(SamSOp::SetX(slice[1])),
		OPCODE_SET_Y => SamOp::Simple(SamSOp::SetY(slice[1])),
		OPCODE_SET_A => SamOp::Simple(SamSOp::SetA(decode_u32(&slice[1..5]))),
		OPCODE_SET_B => SamOp::Simple(SamSOp::SetB(decode_u32(&slice[1..5]))),
		OPCODE_READ_A_AT_B => SamOp::Simple(SamSOp::ReadAAtB),
		OPCODE_READ_X_AT_B => SamOp::Simple(SamSOp::ReadXAtB),
		OPCODE_READ_Y_AT_B => SamOp::Simple(SamSOp::ReadYAtB),
		OPCODE_WRITE_A_AT_B => SamOp::Simple(SamSOp::WriteAAtB),
		OPCODE_WRITE_X_AT_B => SamOp::Simple(SamSOp::WriteXAtB),
		OPCODE_WRITE_Y_AT_B => SamOp::Simple(SamSOp::WriteYAtB),
		OPCODE_ADD_A_TO_B => SamOp::Simple(SamSOp::AddAToB),
		OPCODE_SUB_A_FROM_B => SamOp::Simple(SamSOp::SubAFromB),
		OPCODE_PRINT_CHAR_X => SamOp::Simple(SamSOp::PrintCharX),
		OPCODE_STDIN_X => SamOp::Simple(SamSOp::StdinX),
		OPCODE_ADD_CONST_TO_B => SamOp::Simple(SamSOp::AddConstToB(decode_u32(&slice[1..5]))),
		OPCODE_SUB_CONST_FROM_B => SamOp::Simple(SamSOp::SubConstFromB(decode_u32(&slice[1..5]))),
		OPCODE_PRINT_A => SamOp::Simple(SamSOp::PrintA),
		OPCODE_CALL => SamOp::Call(decode_u32(&slice[1..5])),
		OPCODE_RET => SamOp::Simple(SamSOp::Ret),
		OPCODE_JUMP => SamOp::Jmp(decode_samival(&slice[1..5])),
		OPCODE_JUMP_IF_X => SamOp::JmpIfX(decode_samival(&slice[1..5])),
		OPCODE_ADD_U8_AT_B_TO_X => SamOp::Simple(SamSOp::AddU8AtBToX),
		OPCODE_MUL_U8_AT_B_TO_X => SamOp::Simple(SamSOp::MulU8AtBToX),
		OPCODE_ADD_U32_AT_B_TO_A => SamOp::Simple(SamSOp::AddU32AtBToA),
		OPCODE_MUL_U32_AT_B_TO_A => SamOp::Simple(SamSOp::MulU32AtBToA),
		OPCODE_NEG_A => SamOp::Simple(SamSOp::NegA),
		OPCODE_NEG_X => SamOp::Simple(SamSOp::NegX),
		OPCODE_MOVE_X_TO_A => SamOp::Simple(SamSOp::MoveXToA),
		_ => panic!("decoding invalid sam op!")
	}
}

#[derive(Debug)]
pub struct SamState {
    pub cells: Vec<u8>,
    pub instr_ptr: SamVal,
	pub halted: bool,
	pub a: SamVal,
	pub b: SamVal,
	pub x: u8,
	pub y: u8
}

#[derive(Debug)]
pub enum SamRunOpError {
	Halted,
    ReaderErr(std::io::Error),
    WriterErr(std::io::Error),
}

impl SamState {
    pub fn new(prog: CompiledSamProgram) -> SamState {
		let instr_ptr = *prog.fn_start_poss.get("main").expect("no main function found");
		let mut cells = prog.bytes;
		let hlt = cells.len() as SamVal;
		cells.extend(&[OPCODE_HALT]);
		let b = cells.len() as SamVal;
		push_u32_to_vec(&mut cells, hlt);
        SamState {
            cells,
            instr_ptr,
			halted: false,
			a: 0,
			b,
			x: 0,
			y: 0
        }
    }

	fn reserve_cells(&mut self, max_cell: SamVal) {
		if self.cells.len() <= max_cell as usize {
			self.cells.resize(max_cell as usize + 1, 0);
		}
	}

	pub fn read_u32_at(&mut self, at: SamVal) -> SamVal {
		self.reserve_cells(at + 4);
		decode_u32(&self.cells[at as usize..])
	}

	pub fn read_u8_at(&mut self, at: SamVal) -> u8 {
		self.reserve_cells(at + 1);
		self.cells[at as usize]
	}

	pub fn write_u32_at(&mut self, val: SamVal, at: SamVal) {
		self.reserve_cells(at + 4);
		write_u32(&mut self.cells[at as usize..], val);
	}

	pub fn write_u8_at(&mut self, val: u8, at: SamVal) {
		self.reserve_cells(at + 1);
		self.cells[at as usize] = val;
	}

	pub fn decode_next_op(&self) -> SamOp {
		decode_sam_op(&self.cells[self.instr_ptr as usize..])
	}

	pub fn step(
        &mut self,
        reader: &mut impl Read,
        writer: &mut impl Write,
	 ) -> Result<(), SamRunOpError> {
		self.reserve_cells(self.instr_ptr + 5);
		let op = self.decode_next_op();
		let res = self.run_op(&op, reader, writer)?;
		Ok(res)
	}

    pub fn run(
        &mut self,
        reader: &mut impl Read,
        writer: &mut impl Write,
    ) -> Result<(), SamRunOpError> {
		while !self.halted {
			self.step(reader, writer)?;
		}
		Ok(())
	}

    pub fn run_op(
        &mut self,
        op: &SamOp,
        reader: &mut impl Read,
        writer: &mut impl Write,
    ) -> Result<(), SamRunOpError> {
		if self.halted {
			return Err(SamRunOpError::Halted);
		}
		match op {
			SamOp::Simple(op) => {
				let mut jumped = false;
				match op {
					SamSOp::Halt => {
						self.halted = true;
					},
					SamSOp::SwapXY => {
						std::mem::swap(&mut self.x, &mut self.y);
					},
					SamSOp::SwapAB => {
						std::mem::swap(&mut self.a, &mut self.b);
					},
					SamSOp::SetA(val) => {
						self.a = *val;
					},
					SamSOp::SetB(val) => {
						self.b = *val;
					},
					SamSOp::SetX(val) => {
						self.x = *val;
					},
					SamSOp::SetY(val) => {
						self.y = *val;
					},
					SamSOp::ReadAAtB => {
						self.a = self.read_u32_at(self.b);
					},
					SamSOp::ReadXAtB => {
						self.x = self.read_u8_at(self.b);
					},
					SamSOp::ReadYAtB => {
						self.y = self.read_u8_at(self.b);
					},
					SamSOp::WriteAAtB => {
						self.write_u32_at(self.a, self.b);
					},
					SamSOp::WriteXAtB => {
						self.write_u8_at(self.x, self.b);
					},
					SamSOp::WriteYAtB => {
						self.write_u8_at(self.y, self.b);
					},
					SamSOp::AddAToB => {
						self.a += self.b;
					},
					SamSOp::SubAFromB => {
						self.a -= self.b;
					},
					SamSOp::PrintCharX => {
						let buf: [u8; 1] = [self.x];
						match writer.write_all(&buf) {
							Ok(()) => {}
							Err(e) => {
								return Err(SamRunOpError::WriterErr(e));
							}
						}
						match writer.flush() {
							Ok(()) => {}
							Err(e) => {
								return Err(SamRunOpError::WriterErr(e));
							}
						}
					},
					SamSOp::StdinX => {
						let mut buf: [u8; 1] = [0; 1];
						match reader.read_exact(&mut buf) {
							Ok(()) => {
								// simply ignore \r
								let c = buf[0];
								if c != 13 {
									self.x = c;
								}
							},
							Err(e) => match e.kind() {
								std::io::ErrorKind::UnexpectedEof => {
									self.x = 0;
								},
								_ => {
									return Err(SamRunOpError::ReaderErr(e));
								}
							},
						}
					},
					SamSOp::AddConstToB(val) => {
						self.b += *val;
					},
					SamSOp::SubConstFromB(val) => {
						self.b -= *val;
					},
					SamSOp::PrintA => {
						match write!(writer, "{}", self.a) {
							Ok(()) => {}
							Err(e) => {
								return Err(SamRunOpError::WriterErr(e));
							}
						}
						match writer.flush() {
							Ok(()) => {}
							Err(e) => {
								return Err(SamRunOpError::WriterErr(e));
							}
						}
					},
					SamSOp::Ret => {
						let p = self.read_u32_at(self.b);
						self.instr_ptr = p;
						jumped = true;
					},
					SamSOp::AddU8AtBToX => {
						self.x = self.x.wrapping_add(self.read_u8_at(self.b));
					},
					SamSOp::MulU8AtBToX => {
						self.x = self.x.wrapping_mul(self.read_u8_at(self.b));
					},
					SamSOp::AddU32AtBToA => {
						self.a = self.a.wrapping_add(self.read_u32_at(self.b));
					},
					SamSOp::MulU32AtBToA => {
						self.a = self.a.wrapping_mul(self.read_u32_at(self.b));
					},
					SamSOp::NegA => {
						self.a = 0u32.wrapping_sub(self.a);
					},
					SamSOp::NegX => {
						self.x = 0u8.wrapping_sub(self.x);
					},
					SamSOp::MoveXToA => {
						self.a = self.x as u32;
					}
				}
				if !jumped {
					self.instr_ptr += op.len() as SamVal;
				}
			},
			SamOp::Call(f) => {
				self.write_u32_at(self.instr_ptr+5, self.b);
				self.instr_ptr = *f;
			},
			SamOp::Jmp(offset) => {
				let new_instr_ptr = self.instr_ptr as SamIVal + *offset;
				if new_instr_ptr < 0 {
					panic!("Jumped left of tape!");
				}
				self.instr_ptr = new_instr_ptr as SamVal;
			},
			SamOp::JmpIfX(offset) => {
				if self.x != 0 {
					let new_instr_ptr = self.instr_ptr as SamIVal + *offset;
					if new_instr_ptr < 0 {
						panic!("Jumped left of tape!");
					}
					self.instr_ptr = new_instr_ptr as SamVal;
				} else {
					self.instr_ptr += op.len() as SamVal;
				}
			}
		}
		Ok(())
	}
}
