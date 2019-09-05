use crate::sam::*;
use std::collections::HashMap;

#[derive(Debug)]
pub enum SamLOp {
	Simple(SamSOp),
	Call(String)
}

impl SamLOp {
	pub fn len(&self) -> usize {
		match self {
			SamLOp::Simple(op) => op.len(),
			SamLOp::Call(_) => 5
		}
	}
}

#[derive(Debug)]
pub struct SamFn {
	pub name: String,
	pub arg_sizes: Vec<u32>,
	pub ret_size: u32,
	pub instrs: Vec<SamLOp>
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

#[derive(Debug)]
pub struct CompiledSamProgram {
	pub bytes: Vec<u8>,
	pub fn_start_poss: HashMap<String, u32>
}

pub fn link_sam_fns(fns: HashMap<String, SamFn>) -> CompiledSamProgram {
	let mut fn_start_poss = HashMap::new();
	{
		let mut cur_num_bytes = 0;
		for (f_name, f) in &fns {
			fn_start_poss.insert(f_name.clone(), cur_num_bytes as u32);
			for instr in &f.instrs {
				cur_num_bytes += instr.len();
			}
		}
	}

	let mut bytes = Vec::with_capacity(1000);
	for (_f_name, f) in &fns {
		for instr in &f.instrs {
			let sam_op = match instr {
				SamLOp::Simple(op) => {
					SamOp::Simple(*op)
				},
				SamLOp::Call(called_f_name) => {
					SamOp::Call(*fn_start_poss.get(called_f_name).expect("Linking to unknown function"))
				}
			};
			let instr_bytes = sam_op.encode();
			bytes.extend(instr_bytes);
		}
	}

	CompiledSamProgram {
		bytes,
		fn_start_poss
	}
}