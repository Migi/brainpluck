use crate::hir::*;
use crate::sam::*;
use num::BigUint;
use num::Num;

use std::collections::HashMap;

pub fn hir2sam<'a>(program: &'a Program) -> HashMap<String, SamFn> {
	let mut sam_fns = HashMap::new();
	for (_fn_name, function) in program.fns.iter() {
		let mut cpu = SamCpu::new(&program.fns);
		for stmt in &function.stmts {
			cpu.exec_stmt(stmt);
		}
		let prev = sam_fns.insert(function.name.clone(), SamFn {
			name: function.name.clone(),
			arg_sizes: function.args.iter().map(|x| type_size(x.typ)).collect(),
			ret_size: type_size(function.ret),
			instrs: cpu.out
		});
		assert!(prev.is_none());
	}
	sam_fns
}

fn biguint_to_u32(ui: &BigUint) -> u32 {
	let ui_bytes = ui.to_bytes_le();
	if ui_bytes.len() > 4 {
		panic!("Uint too large for u32");
	}
	let mut bytes = [0,0,0,0];
	for (i,b) in ui_bytes.iter().enumerate() {
		bytes[i] = *b;
	}
	u32::from_le_bytes(bytes)
}

fn biguint_to_u8(ui: &BigUint) -> u8 {
	let ui_bytes = ui.to_bytes_le();
	if ui_bytes.len() > 1 {
		panic!("Uint too large for u8");
	}
	*ui_bytes.last().unwrap()
}

#[derive(Copy,Clone)]
struct LocalVar<'a> {
	name: &'a str,
	typ: VarType,
	location: u32
}

#[derive(Clone)]
struct Locals<'a> {
	locals: HashMap<&'a str, LocalVar<'a>>,
	cur_stack_size: u32
}

struct NewLocal<'a> {
	name: Option<&'a str>,
	typ: VarType
}

enum Dest<'a> {
	None,
	NewLocal(NewLocal<'a>),
	ExistingLocal(LocalVar<'a>),
	A,
	X
}

impl<'a> Locals<'a> {
	fn get(&self, name: &'a str) -> &LocalVar<'a> {
		self.locals.get(name).expect("Accessing unknown local")
	}

	fn create(&mut self, newlocal: NewLocal<'a>) -> LocalVar<'a> {
		let result = LocalVar {
			name: newlocal.name.unwrap_or_else(|| "$temp"),
			typ: newlocal.typ,
			location: self.cur_stack_size
		};
		if let Some(name) = newlocal.name {
			self.locals.insert(name, result);
		}
		self.cur_stack_size += type_size(newlocal.typ);
		result
	}

	fn new_named(&mut self, name: &'a str, typ: VarType) -> LocalVar<'a> {
		self.create(NewLocal {
			name: Some(name),
			typ
		});
	}

	fn new_temp(&mut self, typ: VarType) -> LocalVar<'a> {
		self.create(NewLocal {
			name: None,
			typ
		});
	}
}

fn type_size(typ: VarType) -> u32 {
	match typ {
		VarType::U8 => 1,
		VarType::U32 => 4,
		VarType::Unit => 0,
	}
}

struct SamCpu<'a> {
	locals: Locals<'a>,
	out: Vec<SamOp>,
	cur_b_offset: u32,
	fn_decls: &'a HashMap<String, FnDecl>
}

impl<'a> SamCpu<'a> {
	pub fn new(fn_decls: &'a HashMap<String, FnDecl>) -> SamCpu<'a> {
		SamCpu {
			locals: Locals {
				locals: HashMap::new(),
				cur_stack_size: 0
			},
			out: Vec::new(),
			cur_b_offset: 0,
			fn_decls
		}
	}

	pub fn scope<R>(&mut self, f: impl for<'b> FnOnce(&'b mut SamCpu<'a>) -> R) -> R {
		let mut cpu = SamCpu {
			locals: self.locals.clone(),
			out: Vec::new(),
			cur_b_offset: self.cur_b_offset,
			fn_decls: self.fn_decls
		};
		let rust_closure_return = f(&mut cpu);
		self.out.extend(cpu.out);
		self.cur_b_offset = cpu.cur_b_offset;
		rust_closure_return
	}

	pub fn goto_b_offset(&mut self, offset: u32) {
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
			Expr::FnCall(f) => {
				Some(self.fn_decls.get(&f.fn_name).expect("Calling unknown fn").ret)
			}
		}
	}

	pub fn set_x(&mut self, val: &BigUint) {
		self.out.push(SamOp::SetX(biguint_to_u8(val)));
	}

	pub fn set_a(&mut self, val: &BigUint) {
		self.out.push(SamOp::SetA(biguint_to_u32(val)));
	}

	pub fn write_x_at(&mut self, local: LocalVar<'a>) {
		assert_eq!(local.typ, VarType::U8);
		self.goto_b_offset(local.location);
		self.out.push(SamOp::WriteXAtB);
	}

	pub fn write_a_at(&mut self, local: LocalVar<'a>) {
		assert_eq!(local.typ, VarType::U32);
		self.goto_b_offset(local.location);
		self.out.push(SamOp::WriteAAtB);
	}

	pub fn read_x_at(&mut self, local: LocalVar<'a>) {
		assert_eq!(local.typ, VarType::U8);
		self.goto_b_offset(local.location);
		self.out.push(SamOp::ReadXAtB);
	}

	pub fn read_a_at(&mut self, local: LocalVar<'a>) {
		assert_eq!(local.typ, VarType::U32);
		self.goto_b_offset(local.location);
		self.out.push(SamOp::ReadAAtB);
	}

	pub fn copy_local_to_local(&mut self, a: LocalVar<'a>, b: LocalVar<'a>) {
		assert_eq!(a.typ, b.typ);
		if a.location == b.location {
			return;
		}
		match a.typ {
			VarType::Unit => {},
			VarType::U8 => {
				self.read_x_at(a);
				self.write_x_at(b);
			},
			VarType::U32 => {
				self.read_a_at(a);
				self.write_a_at(b);
			},
		}
	}

	pub fn eval_expr(&mut self, expr: &'a Expr, dest: Dest<'a>) {
		//let expr_type = self.get_expr_type(expr);
		match expr {
			Expr::Literal(lit) => {
				match dest {
					Dest::None => {},
					Dest::X => {
						self.set_x(lit);
					},
					Dest::A => {
						self.set_a(lit);
					},
					Dest::NewLocal(local) => {
						let local = self.locals.create(local);
						match self.get_expr_type(expr) {
							Unit => unreachable!(),
							U8 => {
								self.set_x(lit);
								self.write_x_at(local);
							},
							U32 => {
								self.set_a(lit);
								self.write_a_at(local);
							}
						}
					},
					Dest::ExistingLocal(local) => {
						match self.get_expr_type(expr) {
							Unit => unreachable!(),
							U8 => {
								self.set_x(lit);
								self.write_x_at(local);
							},
							U32 => {
								self.set_a(lit);
								self.write_a_at(local);
							}
						}
					}
				}
			},
			Expr::VarRef(varref) => {
				let varref_local = *self.locals.get(&varref);
				match dest {
					Dest::None => {},
					Dest::X => {
						self.read_x_at(varref_local)
					},
					Dest::A => {
						self.read_a_at(varref_local)
					},
					Dest::NewLocal(local) => {
						let local = self.locals.create(local);
						self.copy_local_to_local(varref_local, local);
					},
					Dest::ExistingLocal(local) => {
						self.copy_local_to_local(varref_local, local);
					}
				}
			},
			Expr::FnCall(fncall) => {
				self.call(fncall, dest).unwrap();
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
				match dest.typ {
					VarType::U8 => {
						self.goto_b_offset(dest.location);
						self.out.push(SamOp::SetX(*bytes.last().unwrap()));
						self.out.push(SamOp::WriteXAtB);
					},
					VarType::U32 => {
						self.goto_b_offset(dest.location);
						self.out.push(SamOp::SetA(biguint_to_u32(lit)));
						self.out.push(SamOp::WriteAAtB);
					},
					VarType::Unit => {}
				}
			},
			Expr::VarRef(varref) => {
				let varref_local = *self.locals.get(&varref);
				match dest.typ {
					VarType::U8 => {
						self.goto_b_offset(varref_local.location);
						self.out.push(SamOp::ReadXAtB);
					},
					VarType::U32 => {
						self.goto_b_offset(varref_local.location);
						self.out.push(SamOp::ReadAAtB);
					},
					VarType::Unit => {}
				}
				match dest.typ {
					VarType::U8 => {
						self.goto_b_offset(dest.location);
						self.out.push(SamOp::WriteXAtB);
					},
					VarType::U32 => {
						self.goto_b_offset(dest.location);
						self.out.push(SamOp::WriteAAtB);
					},
					VarType::Unit => {}
				}
			},
			Expr::FnCall(fncall) => {
				let ret_local = self.call(fncall, None).unwrap();
				self.goto_b_offset(ret_local.location);
				self.out.push(SamOp::ReadXAtB);
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
				self.goto_b_offset(varref_local.location);
				self.out.push(SamOp::ReadXAtB);
			},
			Expr::FnCall(fncall) => {
				let ret_local = self.call(fncall, None).unwrap();
				self.goto_b_offset(ret_local.location);
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
				self.goto_b_offset(varref_local.location);
				self.out.push(SamOp::ReadAAtB);
			},
			Expr::FnCall(fncall) => {
				let ret_local = self.call(fncall, None).unwrap();
				self.goto_b_offset(ret_local.location);
				self.out.push(SamOp::ReadAAtB);
			},
			_ => unimplemented!()
		}
	}

	pub fn call(&mut self, fncall: &'a FnCall, dest: Dest<'a>) {
		if fncall.fn_name == "print" {
			assert_eq!(fncall.args.len(), 1);
			let arg = &fncall.args[0];
			let typ = self.get_expr_type(arg).unwrap_or_else(|| VarType::U32);
			match typ {
				VarType::U8 => {
					self.eval_expr(arg, Dest::X);
					self.out.push(SamOp::PrintX);
				},
				VarType::U32 => {
					self.eval_expr(arg, Dest::A);
					self.out.push(SamOp::PrintA);
				},
				VarType::Unit => {
					panic!("Printing unit");
				}
			}
			None
		} else {
			let fn_decl = self.fn_decls.get(&fncall.fn_name).expect("Calling unknown function");
			assert_eq!(fn_decl.args.len(), fncall.args.len());
			if let Dest::NewLocal(newlocal) = dest {
				assert_eq!(fn_decl.ret, newlocal.typ);
				self.locals.create(newlocal);
			}
			self.scope(|cpu| {
				match dest {
					Dest::NewLocal(newlocal) => {
						
					}
				};
				cpu.locals.new_temp(fn_decl.ret);
				let arg_locals : Vec<_> = fn_decl.args.iter().map(|arg| {
					cpu.locals.new_temp(arg.typ)
				}).collect();
				let cpu_ret_local = cpu.locals.new_temp(VarType::U32);
				for (arg_expr, arg_local) in fncall.args.iter().zip(arg_locals) {
					cpu.eval_expr_to_local(arg_expr, arg_local);
				}
				cpu.goto_b_offset(cpu_ret_local.location);
				cpu.out.push(SamOp::Call(fn_decl.name.clone()));
			})
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
				self.call(fncall, None);
			}
		}
	}
}