use crate::hir::*;
use crate::sam::*;
use crate::linker::*;
use num::BigUint;
use num::Num;

use std::collections::HashMap;

pub fn hir2sam<'a>(program: &'a Program) -> HashMap<String, SamFn> {
	let mut sam_fns = HashMap::new();
	for (fn_name, function) in program.fns.iter() {
		let mut cpu = SamCpu::new(&program.fns, fn_name);
		for stmt in &function.scope.stmts {
			cpu.exec_stmt(stmt);
		}
		cpu.ret(function.scope.final_expr.as_ref().map(|e| &**e));
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

#[derive(Copy, Clone)]
struct NewLocal<'a> {
	name: Option<&'a str>,
	typ: VarType
}

#[derive(Copy, Clone)]
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
		})
	}

	fn new_temp(&mut self, typ: VarType) -> LocalVar<'a> {
		self.create(NewLocal {
			name: None,
			typ
		})
	}
}

fn type_size(typ: VarType) -> u32 {
	match typ {
		VarType::U8 => 1,
		VarType::U32 => 4,
		VarType::Unit => 0,
	}
}

fn types_compatible(type1: VarType, type2: Option<VarType>) -> bool {
	match type2 {
		Some(type2) => {
			type1 == type2
		},
		None => {
			true
		}
	}
}

struct SamCpu<'a> {
	locals: Locals<'a>,
	out: Vec<SamLOp>,
	cur_b_offset: u32,
	fn_decls: &'a HashMap<String, FnDecl>,
	valret_local: LocalVar<'a>,
	iret_local: LocalVar<'a>
}

impl<'a> SamCpu<'a> {
	/*pub fn new(fn_decls: &'a HashMap<String, FnDecl>) -> SamCpu<'a> {
		SamCpu {
			locals: Locals {
				locals: HashMap::new(),
				cur_stack_size: 0
			},
			out: Vec::new(),
			cur_b_offset: 0,
			fn_decls
		}
	}*/

	pub fn new(fn_decls: &'a HashMap<String, FnDecl>, fn_name: &'a str) -> SamCpu<'a> {
		let decl = fn_decls.get(fn_name).expect("Compiling unknown function");
		let mut locals = Locals {
			locals: HashMap::new(),
			cur_stack_size: 0
		};
		let valret_local = locals.new_temp(decl.ret);
		for arg in &decl.args {
			locals.new_named(&arg.name, arg.typ);
		}
		let iret_local = locals.new_temp(VarType::U32);
		SamCpu {
			locals,
			out: Vec::new(),
			cur_b_offset: iret_local.location,
			fn_decls,
			valret_local,
			iret_local
		}
	}

	pub fn scope<R>(&mut self, f: impl for<'b> FnOnce(&'b mut SamCpu<'a>) -> R) -> R {
		let mut cpu = SamCpu {
			locals: self.locals.clone(),
			out: Vec::new(),
			cur_b_offset: self.cur_b_offset,
			fn_decls: self.fn_decls,
			valret_local: self.valret_local,
			iret_local: self.iret_local
		};
		let rust_closure_return = f(&mut cpu);
		self.out.extend(cpu.out);
		self.cur_b_offset = cpu.cur_b_offset;
		rust_closure_return
	}

	pub fn goto_b_offset(&mut self, offset: u32) {
		if self.cur_b_offset < offset {
			self.out.push(SamLOp::Simple(SamSOp::AddConstToB(offset - self.cur_b_offset)));
		} else if offset < self.cur_b_offset {
			self.out.push(SamLOp::Simple(SamSOp::SubConstFromB(self.cur_b_offset - offset)));
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
			},
			Expr::Scope(s) => {
				match &s.final_expr {
					Some(e) => self.get_expr_type(e),
					None => Some(VarType::Unit)
				}
			},
		}
	}

	pub fn set_x(&mut self, val: &BigUint) {
		self.out.push(SamLOp::Simple(SamSOp::SetX(biguint_to_u8(val))));
	}

	pub fn set_a(&mut self, val: &BigUint) {
		self.out.push(SamLOp::Simple(SamSOp::SetA(biguint_to_u32(val))));
	}

	pub fn write_x_at(&mut self, local: LocalVar<'a>) {
		assert_eq!(local.typ, VarType::U8);
		self.goto_b_offset(local.location);
		self.out.push(SamLOp::Simple(SamSOp::WriteXAtB));
	}

	pub fn write_a_at(&mut self, local: LocalVar<'a>) {
		assert_eq!(local.typ, VarType::U32);
		self.goto_b_offset(local.location);
		self.out.push(SamLOp::Simple(SamSOp::WriteAAtB));
	}

	pub fn read_x_at(&mut self, local: LocalVar<'a>) {
		assert_eq!(local.typ, VarType::U8);
		self.goto_b_offset(local.location);
		self.out.push(SamLOp::Simple(SamSOp::ReadXAtB));
	}

	pub fn read_a_at(&mut self, local: LocalVar<'a>) {
		assert_eq!(local.typ, VarType::U32);
		self.goto_b_offset(local.location);
		self.out.push(SamLOp::Simple(SamSOp::ReadAAtB));
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

	pub fn ret(&mut self, val: Option<&'a Expr>) {
		if let Some(val) = val {
			self.eval_expr(val, Dest::ExistingLocal(self.valret_local));
		}
		self.goto_b_offset(self.iret_local.location);
		self.out.push(SamLOp::Simple(SamSOp::Ret));
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
						assert!(types_compatible(local.typ, self.get_expr_type(expr)));
						match local.typ {
							VarType::Unit => unreachable!(),
							VarType::U8 => {
								self.set_x(lit);
								self.write_x_at(local);
							},
							VarType::U32 => {
								self.set_a(lit);
								self.write_a_at(local);
							}
						}
					},
					Dest::ExistingLocal(local) => {
						assert!(types_compatible(local.typ, self.get_expr_type(expr)));
						match local.typ {
							VarType::Unit => unreachable!(),
							VarType::U8 => {
								self.set_x(lit);
								self.write_x_at(local);
							},
							VarType::U32 => {
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
				self.call(fncall, dest);
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
					self.out.push(SamLOp::Simple(SamSOp::PrintX));
				},
				VarType::U32 => {
					self.eval_expr(arg, Dest::A);
					self.out.push(SamLOp::Simple(SamSOp::PrintA));
				},
				VarType::Unit => {
					panic!("Printing unit");
				}
			}
		} else {
			let fn_decl = self.fn_decls.get(&fncall.fn_name).expect("Calling unknown function");
			assert_eq!(fn_decl.args.len(), fncall.args.len());
			let self_ret_local = if let Dest::NewLocal(newlocal) = dest {
				assert_eq!(fn_decl.ret, newlocal.typ);
				Some(self.locals.create(newlocal))
			} else {
				None
			};
			let self_ret_local = self.scope(|cpu| {
				let self_ret_local = self_ret_local.unwrap_or_else(|| cpu.locals.new_temp(fn_decl.ret));
				for (arg_expr, arg_decl) in fncall.args.iter().zip(fn_decl.args.iter()) {
					let arg_local = cpu.locals.new_temp(arg_decl.typ);
					cpu.scope(|cpu| {
						cpu.eval_expr(arg_expr, Dest::ExistingLocal(arg_local));
					});
				}
				let cpu_ret_local = cpu.locals.new_temp(VarType::U32);
				cpu.goto_b_offset(cpu_ret_local.location);
				cpu.out.push(SamLOp::Call(fn_decl.name.clone()));
				self_ret_local
			});
			match dest {
				Dest::None => {},
				Dest::NewLocal(_) => {
					// should be ok
				},
				Dest::ExistingLocal(dest_local) => {
					self.copy_local_to_local(self_ret_local, dest_local);
				},
				Dest::A => {
					self.read_a_at(self_ret_local)
				},
				Dest::X => {
					self.read_x_at(self_ret_local)
				}
			}
		}
	}

	pub fn exec_stmt(&mut self, stmt: &'a Stmt) {
		match stmt {
			Stmt::VarDecl(decl) => {
				self.eval_expr(&decl.init, Dest::NewLocal(NewLocal {
					name: Some(&decl.var_name),
					typ: decl.typ
				}));
			},
			Stmt::VarAssign(ass) => {
				let local = *self.locals.get(&ass.var_name);
				self.eval_expr(&ass.expr, Dest::ExistingLocal(local));
			},
			Stmt::FnCall(fncall) => {
				self.call(fncall, Dest::None);
			}
		}
	}
}