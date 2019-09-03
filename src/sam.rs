use std::io::{Read, Write};

pub type SamVal = u32;

#[derive(Debug)]
pub enum SamOp {
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
	Call(String)
}

#[derive(Debug)]
pub struct SamFn {
	pub name: String,
	pub arg_sizes: Vec<u32>,
	pub ret_size: u32,
	pub instrs: Vec<SamOp>
}

impl SamFn {
	fn total_arg_size(&self) -> u32 {
		let mut result = 0;
		for arg_size in &self.arg_sizes {
			result += arg_size;
		}
		result
	}
}

impl SamOp {
	fn encode(self) -> Vec<u8> {
		match self {
			SamOp::Halt => {
				vec![0]
			},
			SamOp::SwapXY => {
				vec![1]
			},
			SamOp::SwapAB => {
				vec![2]
			},
			SamOp::SetX(val) => {
				vec![3, val]
			},
			SamOp::SetY(val) => {
				vec![4, val]
			},
			SamOp::SetA(val) => {
				let mut res = vec![5];
				push_u32_to_vec(&mut res, val);
				res
			},
			SamOp::SetB(val) => {
				let mut res = vec![6];
				push_u32_to_vec(&mut res, val);
				res
			},
			SamOp::ReadAAtB => {
				vec![7]
			},
			SamOp::ReadXAtB => {
				vec![8]
			},
			SamOp::ReadYAtB => {
				vec![9]
			},
			SamOp::WriteAAtB => {
				vec![10]
			},
			SamOp::WriteXAtB => {
				vec![11]
			},
			SamOp::WriteYAtB => {
				vec![12]
			},
			SamOp::AddAToB => {
				vec![13]
			},
			SamOp::SubAFromB => {
				vec![14]
			},
			SamOp::PrintX => {
				vec![15]
			},
			SamOp::StdinX => {
				vec![16]
			},
			SamOp::AddConstToB(val) => {
				let mut res = vec![17];
				push_u32_to_vec(&mut res, val);
				res
			},
			SamOp::SubConstFromB(val) => {
				let mut res = vec![18];
				push_u32_to_vec(&mut res, val);
				res
			},
			SamOp::PrintA => {
				vec![19]
			},
			SamOp::Call(f) => {
				unimplemented!()
			},
		}
	}

	fn len(&self) -> usize {
		match self {
			SamOp::Halt => {
				1
			},
			SamOp::SwapXY => {
				1
			},
			SamOp::SwapAB => {
				1
			},
			SamOp::SetX(_val) => {
				2
			},
			SamOp::SetY(_val) => {
				2
			},
			SamOp::SetA(_val) => {
				5
			},
			SamOp::SetB(_val) => {
				5
			},
			SamOp::ReadAAtB => {
				1
			},
			SamOp::ReadXAtB => {
				1
			},
			SamOp::ReadYAtB => {
				1
			},
			SamOp::WriteAAtB => {
				1
			},
			SamOp::WriteXAtB => {
				1
			},
			SamOp::WriteYAtB => {
				1
			},
			SamOp::AddAToB => {
				1
			},
			SamOp::SubAFromB => {
				1
			},
			SamOp::PrintX => {
				1
			},
			SamOp::StdinX => {
				1
			},
			SamOp::AddConstToB(_val) => {
				5
			},
			SamOp::SubConstFromB(_val) => {
				5
			},
			SamOp::PrintA => {
				1
			},
			SamOp::Call(f) => {
				unimplemented!()
			},
		}
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
		0 => SamOp::Halt,
		1 => SamOp::SwapXY,
		2 => SamOp::SwapAB,
		3 => SamOp::SetX(slice[1]),
		4 => SamOp::SetY(slice[1]),
		5 => SamOp::SetA(decode_u32(&slice[1..5])),
		6 => SamOp::SetB(decode_u32(&slice[1..5])),
		7 => SamOp::ReadAAtB,
		8 => SamOp::ReadXAtB,
		9 => SamOp::ReadYAtB,
		10 => SamOp::WriteAAtB,
		11 => SamOp::WriteXAtB,
		12 => SamOp::WriteYAtB,
		13 => SamOp::AddAToB,
		14 => SamOp::SubAFromB,
		15 => SamOp::PrintX,
		16 => SamOp::StdinX,
		17 => SamOp::AddConstToB(decode_u32(&slice[1..5])),
		18 => SamOp::SubConstFromB(decode_u32(&slice[1..5])),
		19 => SamOp::PrintA,
		// TODO: Call
		_ => panic!("decoding invalid sam op!")
	}
}

pub struct SamState {
    cells: Vec<u8>,
    instr_ptr: usize,
	halted: bool,
	a: SamVal,
	b: SamVal,
	x: u8,
	y: u8
}

#[derive(Debug)]
pub enum SamRunOpError {
	Halted,
    ReaderErr(std::io::Error),
    WriterErr(std::io::Error),
}

impl SamState {
    pub fn new() -> SamState {
        SamState {
            cells: vec![0; 1000],
            instr_ptr: 0,
			halted: false,
			a: 0,
			b: 0,
			x: 0,
			y: 0
        }
    }

	fn reserve_cells(&mut self, max_cell: usize) {
		if self.cells.len() <= max_cell {
			self.cells.resize(max_cell+1, 0);
		}
	}

	pub fn read_u32_at(&mut self, at: SamVal) -> SamVal {
		self.reserve_cells(at as usize + 4);
		decode_u32(&self.cells[at as usize..])
	}

	pub fn read_u8_at(&mut self, at: SamVal) -> u8 {
		self.reserve_cells(at as usize + 1);
		self.cells[at as usize]
	}

	pub fn write_u32_at(&mut self, val: SamVal, at: SamVal) {
		self.reserve_cells(at as usize + 4);
		write_u32(&mut self.cells[at as usize..], val);
	}

	pub fn write_u8_at(&mut self, val: u8, at: SamVal) {
		self.reserve_cells(at as usize + 1);
		self.cells[at as usize] = val;
	}

	pub fn step(
        &mut self,
        reader: &mut impl Read,
        writer: &mut impl Write,
	 ) -> Result<(), SamRunOpError> {
		self.reserve_cells(self.instr_ptr + 5);
		let op = decode_sam_op(&self.cells[self.instr_ptr as usize..]);
		let op_len = op.len();
		let res = self.run_op(&op, reader, writer)?;
		self.instr_ptr += op_len;
		Ok(res)
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
			SamOp::Halt => {
				self.halted = true;
			},
			SamOp::SwapXY => {
				std::mem::swap(&mut self.x, &mut self.y);
			},
			SamOp::SwapAB => {
				std::mem::swap(&mut self.a, &mut self.b);
			},
			SamOp::SetA(val) => {
				self.a = *val;
			},
			SamOp::SetB(val) => {
				self.b = *val;
			},
			SamOp::SetX(val) => {
				self.x = *val;
			},
			SamOp::SetY(val) => {
				self.y = *val;
			},
			SamOp::ReadAAtB => {
				self.a = self.read_u32_at(self.b);
			},
			SamOp::ReadXAtB => {
				self.x = self.read_u8_at(self.b);
			},
			SamOp::ReadYAtB => {
				self.y = self.read_u8_at(self.b);
			},
			SamOp::WriteAAtB => {
				self.write_u32_at(self.a, self.b);
			},
			SamOp::WriteXAtB => {
				self.write_u8_at(self.x, self.b);
			},
			SamOp::WriteYAtB => {
				self.write_u8_at(self.y, self.b);
			},
			SamOp::AddAToB => {
				self.a += self.b;
			},
			SamOp::SubAFromB => {
				self.a -= self.b;
			},
			SamOp::PrintX => {
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
			SamOp::StdinX => {
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
			SamOp::AddConstToB(val) => {
				self.b += *val;
			},
			SamOp::SubConstFromB(val) => {
				self.b -= *val;
			},
			SamOp::PrintA => {
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
			SamOp::Call(f) => {
				unimplemented!()
			}
		}
		Ok(())
	}

    pub fn run_ops(
        &mut self,
        ops: &[SamOp],
        reader: &mut impl Read,
        writer: &mut impl Write,
    ) -> Result<(), SamRunOpError> {
		for op in ops {
			self.run_op(&op, reader, writer)?;
			if self.halted {
				return Ok(());
			}
		}
		Ok(())
	}
}
