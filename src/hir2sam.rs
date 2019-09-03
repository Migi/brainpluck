use crate::hir::*;
use crate::sam::*;
use num::BigUint;
use num::Num;

use std::collections::HashMap;

pub fn hir2sam<'a>(program: &'a Program) -> HashMap<String, SamFn> {
	let mut sam_fns = HashMap::new();
	for (_fn_name, function) in program.fns.iter() {
		let mut cpu = SamCpu::new();
		for stmt in &function.stmts {
			cpu.exec_stmt(stmt);
		}
		let prev = sam_fns.insert(function.name.clone(), SamFn {
			name: function.name.clone(),
			arg_sizes: function.args.iter().map(|x| type_size(x.arg_type)).collect(),
			ret_size: match function.ret {
				Some(ret) => type_size(ret),
				None => 0
			},
			instrs: cpu.out
		});
		assert!(prev.is_none());
	}
	sam_fns
}

#[derive(Copy,Clone)]
struct LocalVar<'a> {
	name: &'a str,
	typ: VarType,
	location: u32
}

struct Locals<'a> {
	locals: HashMap<&'a str, LocalVar<'a>>,
	cur_stack_size: u32
}

fn biguint_to_u32(ui: &BigUint) -> u32 {
	let ui_bytes = ui.to_bytes_le();
	let mut bytes = [0,0,0,0];
	for (i,b) in ui_bytes.iter().enumerate() {
		bytes[i] = *b;
	}
	u32::from_le_bytes(bytes)
}

impl<'a> Locals<'a> {
	fn get(&self, name: &'a str) -> &LocalVar<'a> {
		self.locals.get(name).expect("Accessing unknown local")
	}

	fn new_named(&mut self, name: &'a str, typ: VarType) -> LocalVar<'a> {
		let result = LocalVar {
			name,
			typ,
			location: self.cur_stack_size
		};
		self.locals.insert(name, result);
		self.cur_stack_size += type_size(typ);
		result
	}

	fn new_temp(&mut self, typ: VarType) -> LocalVar<'a> {
		let result = LocalVar {
			name: "$temp",
			typ,
			location: self.cur_stack_size
		};
		self.cur_stack_size += type_size(typ);
		result
	}
}

fn type_size(typ: VarType) -> u32 {
	match typ {
		VarType::U8 => 1,
		VarType::U32 => 4
	}
}

struct SamCpu<'a> {
	locals: Locals<'a>,
	out: Vec<SamOp>,
	cur_b_offset: u32,
}

impl<'a> SamCpu<'a> {
	pub fn new() -> SamCpu<'a> {
		SamCpu {
			locals: Locals {
				locals: HashMap::new(),
				cur_stack_size: 0
			},
			out: Vec::new(),
			cur_b_offset: 0
		}
	}

	pub fn set_b_offset(&mut self, offset: u32) {
		if self.cur_b_offset < offset {
			self.out.push(SamOp::AddConstToB(offset - self.cur_b_offset));
		} else if offset < self.cur_b_offset {
			self.out.push(SamOp::SubConstFromB(self.cur_b_offset - offset));
		}
		self.cur_b_offset = offset;
	}

	pub fn get_expr_type(&self, expr: &'a Expr) -> Option<VarType> {
		match expr {
			Expr::Literal(_lit) => {
				None
			},
			Expr::VarRef(varref) => {
				Some(self.locals.get(&varref).typ)
			},
			Expr::BinOp(binop) => {
				let a_type = self.get_expr_type(&binop.args.0);
				let b_type = self.get_expr_type(&binop.args.1);
				match a_type {
					Some(a_type) => {
						match b_type {
							Some(b_type) => {
								if a_type == b_type {
									Some(a_type)
								} else {
									panic!("Binop on incompatible types");
								}
							},
							None => Some(a_type)
						}
					},
					None => b_type
				}
			},
			_ => unimplemented!()
		}
	}

	pub fn eval_expr_to_local(&mut self, expr: &'a Expr, dest: LocalVar<'a>) {
		match expr {
			Expr::Literal(lit) => {
				let bytes = lit.to_bytes_be();
				if bytes.len() > type_size(dest.typ) as usize {
					panic!("Literal too large for type");
				}
				self.set_b_offset(dest.location);
				match dest.typ {
					VarType::U8 => {
						self.out.push(SamOp::SetX(*bytes.last().unwrap()));
						self.out.push(SamOp::WriteXAtB);
					},
					VarType::U32 => {
						self.out.push(SamOp::SetA(biguint_to_u32(lit)));
						self.out.push(SamOp::WriteAAtB);
					}
				}
			},
			Expr::VarRef(varref) => {
				let varref_local = *self.locals.get(&varref);
				self.set_b_offset(varref_local.location);
				match dest.typ {
					VarType::U8 => {
						self.out.push(SamOp::ReadXAtB);
					},
					VarType::U32 => {
						self.out.push(SamOp::ReadAAtB);
					}
				}
				self.set_b_offset(dest.location);
				match dest.typ {
					VarType::U8 => {
						self.out.push(SamOp::WriteXAtB);
					},
					VarType::U32 => {
						self.out.push(SamOp::WriteAAtB);
					}
				}
			},
			_ => unimplemented!()
		}
	}

	pub fn eval_expr_to_x(&mut self, expr: &'a Expr) {
		match expr {
			Expr::Literal(lit) => {
				let bytes = lit.to_bytes_be();
				if bytes.len() != 1 {
					panic!("Literal too large for type");
				}
				self.out.push(SamOp::SetX(*bytes.last().unwrap()));
			},
			Expr::VarRef(varref) => {
				let varref_local = *self.locals.get(&varref);
				if varref_local.typ != VarType::U8 {
					panic!("Wrong type to eval to X");
				}
				self.set_b_offset(varref_local.location);
				self.out.push(SamOp::ReadXAtB);
			},
			_ => unimplemented!()
		}
	}

	pub fn eval_expr_to_a(&mut self, expr: &'a Expr) {
		match expr {
			Expr::Literal(lit) => {
				self.out.push(SamOp::SetA(biguint_to_u32(lit)));
			},
			Expr::VarRef(varref) => {
				let varref_local = *self.locals.get(&varref);
				if varref_local.typ != VarType::U32 {
					panic!("Wrong type to eval to A");
				}
				self.set_b_offset(varref_local.location);
				self.out.push(SamOp::ReadAAtB);
			},
			_ => unimplemented!()
		}
	}

	pub fn exec_stmt(&mut self, stmt: &'a Stmt) {
		match stmt {
			Stmt::VarDecl(decl) => {
				let local = self.locals.new_named(&decl.var_name, decl.typ);
				self.eval_expr_to_local(&decl.init, local);
			},
			Stmt::VarAssign(ass) => {
				let local = *self.locals.get(&ass.var_name);
				self.eval_expr_to_local(&ass.expr, local);
			},
			Stmt::FnCall(fncall) => {
				if fncall.fn_name == "print" {
					assert_eq!(fncall.args.len(), 1);
					let arg = &fncall.args[0];
					let typ = self.get_expr_type(arg).unwrap_or_else(|| VarType::U32);
					match typ {
						VarType::U8 => {
							self.eval_expr_to_x(arg);
							self.out.push(SamOp::PrintX);
						},
						VarType::U32 => {
							self.eval_expr_to_a(arg);
							self.out.push(SamOp::PrintA);
						}
					}
				} else {
					unimplemented!()
				}
			}
		}
	}
}