use crate::sam::*;
use crate::hir2sam::SamBlock;
use std::collections::HashMap;

#[derive(Debug)]
pub enum SamLOp {
	Simple(SamSOp),
	Call(String),
	JmpToBlockIfX(usize)
}

impl SamLOp {
	pub fn len(&self) -> usize {
		match self {
			SamLOp::Simple(op) => op.len(),
			SamLOp::Call(_) => 5,
			SamLOp::JmpToBlockIfX(_) => 5,
		}
	}
}

#[derive(Debug)]
pub struct SamFn {
	pub name: String,
	pub arg_sizes: Vec<u32>,
	pub ret_size: u32,
	pub blocks: Vec<SamBlock>,
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
	#[derive(Debug)]
	enum SamFnOp {
		Simple(SamSOp),
		Call(String),
		JmpToByteOffset(SamIVal),
		JmpToByteOffsetIfX(SamIVal)
	}

	impl SamFnOp {
		pub fn len(&self) -> usize {
			match self {
				SamFnOp::Simple(op) => op.len(),
				SamFnOp::Call(_) => 5,
				SamFnOp::JmpToByteOffset(_) => 5,
				SamFnOp::JmpToByteOffsetIfX(_) => 5,
			}
		}
	}

	let mut fn_ops = HashMap::new();
	{
		for (f_name, f) in &fns {
			// greedily find a good order for the blocks (with few unnecessary jumps)
			let mut pre_to_post_num = f.blocks.iter().map(|_| None).collect::<Vec<_>>();
			let mut post_to_pre_num = Vec::new();
			while post_to_pre_num.len() < f.blocks.len() {
				// find first unincluded block
				let mut pre_num = 0;
				while pre_to_post_num[pre_num].is_some() {
					pre_num += 1;
				}
				// include the block, then its next block (if any), then its next block, etc
				while pre_to_post_num[pre_num].is_none() {
					pre_to_post_num[pre_num] = Some(post_to_pre_num.len());
					post_to_pre_num.push(pre_num);
					if let Some(next) = f.blocks[pre_num].next_block_index {
						pre_num = next;
					} else {
						break;
					}
				}
			}
			let pre_to_post_num = pre_to_post_num.into_iter().map(|x| x.unwrap()).collect::<Vec<_>>();
			// calculate all blocks' first byte positions (relative to start of function)
			let mut block_start_poss = Vec::new();
			let mut cur_num_bytes = 0;
			for post_num in 0..f.blocks.len() {
				block_start_poss.push(cur_num_bytes as u32);
				for op in &f.blocks[post_to_pre_num[post_num]].ops {
					cur_num_bytes += op.len();
				}
				match f.blocks[post_to_pre_num[post_num]].next_block_index {
					Some(next_block_index) => {
						if post_num < f.blocks.len()-1 && next_block_index == post_to_pre_num[post_num+1] {
							// no jmp needed
						} else {
							cur_num_bytes += SamFnOp::JmpToByteOffset(0).len();
						}
					},
					None => {
						//cur_num_bytes += SamSOp::Ret.len();
					}
				}
			}
			// create function
			let mut ops = Vec::new();
			cur_num_bytes = 0;
			for post_num in 0..f.blocks.len() {
				assert_eq!(cur_num_bytes as u32, block_start_poss[post_num]);
				for op in &f.blocks[post_to_pre_num[post_num]].ops {
					let new_op = match op {
						SamLOp::Simple(op) => SamFnOp::Simple(*op),
						SamLOp::Call(f) => SamFnOp::Call(f.clone()),
						SamLOp::JmpToBlockIfX(b) => SamFnOp::JmpToByteOffsetIfX(
							(block_start_poss[pre_to_post_num[*b]] as SamIVal) - (cur_num_bytes as SamIVal)
						),
					};
					cur_num_bytes += new_op.len();
					ops.push(new_op);
				}
				match f.blocks[post_to_pre_num[post_num]].next_block_index {
					Some(next_block_index) => {
						if post_num < f.blocks.len()-1 && next_block_index == post_to_pre_num[post_num+1] {
							// no jmp needed
						} else {
							let new_op = SamFnOp::JmpToByteOffset(
								(block_start_poss[pre_to_post_num[next_block_index]] as SamIVal) - (cur_num_bytes as SamIVal)
							);
							cur_num_bytes += new_op.len();
							ops.push(new_op);
						}
					},
					None => {
						/*let new_op = SamFnOp::Simple(SamSOp::Ret);
						cur_num_bytes += new_op.len();
						ops.push(new_op);*/
					}
				}
			}
			fn_ops.insert(f_name.clone(), ops);
		}
	}

	// calculate all functions' first byte positions
	let mut fn_start_poss = HashMap::new();
	{
		let mut cur_num_bytes = 0;
		for (f_name, _f) in &fns {
			fn_start_poss.insert(f_name.clone(), cur_num_bytes as u32);
			for op in &fn_ops[f_name] {
				cur_num_bytes += op.len();
			}
		}
	}

	let mut bytes = Vec::with_capacity(1000);
	for (f_name, _f) in &fns {
		for op in &fn_ops[f_name] {
			let sam_op = match op {
				SamFnOp::Simple(op) => {
					SamOp::Simple(*op)
				},
				SamFnOp::Call(called_f_name) => {
					SamOp::Call(*fn_start_poss.get(called_f_name).expect("Linking to unknown function"))
				},
				SamFnOp::JmpToByteOffset(offset) => {
					SamOp::Jmp(*offset)
				},
				SamFnOp::JmpToByteOffsetIfX(offset) => {
					SamOp::JmpIfX(*offset)
				},
			};
			let num_bytes = sam_op.encode();
			bytes.extend(num_bytes);
		}
	}

	CompiledSamProgram {
		bytes,
		fn_start_poss
	}
}