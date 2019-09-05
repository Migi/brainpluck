use std::io::{Read, Write};
use crate::linker::*;

pub type SamVal = u32;

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
	PrintX,
	StdinX,
	AddConstToB(SamVal),
	SubConstFromB(SamVal),
	PrintA,
	Ret
}

#[derive(Debug)]
pub enum SamOp {
	Simple(SamSOp),
	Call(SamVal)
}

impl SamSOp {
	pub fn encode(&self) -> Vec<u8> {
		match self {
			SamSOp::Halt => {
				vec![0]
			},
			SamSOp::SwapXY => {
				vec![1]
			},
			SamSOp::SwapAB => {
				vec![2]
			},
			SamSOp::SetX(val) => {
				vec![3, *val]
			},
			SamSOp::SetY(val) => {
				vec![4, *val]
			},
			SamSOp::SetA(val) => {
				let mut res = vec![5];
				push_u32_to_vec(&mut res, *val);
				res
			},
			SamSOp::SetB(val) => {
				let mut res = vec![6];
				push_u32_to_vec(&mut res, *val);
				res
			},
			SamSOp::ReadAAtB => {
				vec![7]
			},
			SamSOp::ReadXAtB => {
				vec![8]
			},
			SamSOp::ReadYAtB => {
				vec![9]
			},
			SamSOp::WriteAAtB => {
				vec![10]
			},
			SamSOp::WriteXAtB => {
				vec![11]
			},
			SamSOp::WriteYAtB => {
				vec![12]
			},
			SamSOp::AddAToB => {
				vec![13]
			},
			SamSOp::SubAFromB => {
				vec![14]
			},
			SamSOp::PrintX => {
				vec![15]
			},
			SamSOp::StdinX => {
				vec![16]
			},
			SamSOp::AddConstToB(val) => {
				let mut res = vec![17];
				push_u32_to_vec(&mut res, *val);
				res
			},
			SamSOp::SubConstFromB(val) => {
				let mut res = vec![18];
				push_u32_to_vec(&mut res, *val);
				res
			},
			SamSOp::PrintA => {
				vec![19]
			},
			SamSOp::Ret => {
				vec![21]
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
				let mut res = vec![20];
				push_u32_to_vec(&mut res, *c);
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

fn decode_sam_op(slice: &[u8]) -> SamOp {
	match slice[0] {
		0 => SamOp::Simple(SamSOp::Halt),
		1 => SamOp::Simple(SamSOp::SwapXY),
		2 => SamOp::Simple(SamSOp::SwapAB),
		3 => SamOp::Simple(SamSOp::SetX(slice[1])),
		4 => SamOp::Simple(SamSOp::SetY(slice[1])),
		5 => SamOp::Simple(SamSOp::SetA(decode_u32(&slice[1..5]))),
		6 => SamOp::Simple(SamSOp::SetB(decode_u32(&slice[1..5]))),
		7 => SamOp::Simple(SamSOp::ReadAAtB),
		8 => SamOp::Simple(SamSOp::ReadXAtB),
		9 => SamOp::Simple(SamSOp::ReadYAtB),
		10 => SamOp::Simple(SamSOp::WriteAAtB),
		11 => SamOp::Simple(SamSOp::WriteXAtB),
		12 => SamOp::Simple(SamSOp::WriteYAtB),
		13 => SamOp::Simple(SamSOp::AddAToB),
		14 => SamOp::Simple(SamSOp::SubAFromB),
		15 => SamOp::Simple(SamSOp::PrintX),
		16 => SamOp::Simple(SamSOp::StdinX),
		17 => SamOp::Simple(SamSOp::AddConstToB(decode_u32(&slice[1..5]))),
		18 => SamOp::Simple(SamSOp::SubConstFromB(decode_u32(&slice[1..5]))),
		19 => SamOp::Simple(SamSOp::PrintA),
		20 => SamOp::Call(decode_u32(&slice[1..5])),
		21 => SamOp::Simple(SamSOp::Ret),
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
		cells.extend(&[15]); // halt
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
					SamSOp::PrintX => {
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
					}
				}
				self.instr_ptr += op.len() as SamVal;
			},
			SamOp::Call(f) => {
				self.write_u32_at(self.instr_ptr+4, self.b);
				self.instr_ptr = *f;
			}
		}
		Ok(())
	}
}
