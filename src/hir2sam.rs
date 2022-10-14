use crate::hir::*;
use crate::linker::*;
use crate::sam::*;
use num::BigUint;
use num::Num;

use std::collections::BTreeMap;

use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}
macro_rules! _console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// calling convention (stack):
// - return value value
// - arguments
// - CALL instruction writes instruction ptr + 5 here (CALL is 5 bytes wide)

pub fn hir2sam(program: &Program) -> BTreeMap<String, SamFn> {
    let mut sam_fns = BTreeMap::new();
    for (fn_name, function) in program.fns.iter() {
        let mut sam_block_arena = SamBlockArena { blocks: Vec::new() };
        let mut cpu = SamCpu::new(&program.fns, fn_name, &mut sam_block_arena);
        for stmt in &function.scope.stmts {
            cpu.exec_stmt(stmt);
        }
        cpu.ret(function.scope.final_expr.as_deref());
        let prev = sam_fns.insert(
            function.name.clone(),
            SamFn {
                name: function.name.clone(),
                arg_sizes: function.args.iter().map(|x| type_size(&x.typ)).collect(),
                ret_size: type_size(&function.ret),
                blocks: sam_block_arena.blocks,
            },
        );
        assert!(prev.is_none());
    }
    sam_fns
}

#[derive(Clone)]
struct BuiltInFn {
    name: String,
    ret_type: VarType,
}

fn get_builtin_fn(name: &str) -> Option<BuiltInFn> {
    let mut map = BTreeMap::new();
    {
        let mut insert_builtin = |name: &str, ret_type: VarType| {
            map.insert(
                name.to_owned(),
                BuiltInFn {
                    name: name.to_owned(),
                    ret_type,
                },
            );
        };
        insert_builtin("println", VarType::Unit);
        insert_builtin("print", VarType::Unit);
        insert_builtin("print_char", VarType::Unit);
        insert_builtin("read_char", VarType::U8);
    }
    map.get(name).cloned()
}

fn biguint_to_u32(ui: &BigUint) -> u32 {
    let ui_bytes = ui.to_bytes_le();
    if ui_bytes.len() > 4 {
        panic!("Uint too large for u32");
    }
    let mut bytes = [0, 0, 0, 0];
    for (i, b) in ui_bytes.iter().enumerate() {
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

#[derive(Clone, Debug)]
struct LocalVar<'a> {
    name: &'a str,
    typ: VarType,
    location: u32,
}

#[derive(Clone, Debug)]
struct Locals<'a> {
    locals: BTreeMap<&'a str, LocalVar<'a>>,
    cur_stack_size: u32,
}

#[derive(Clone, Debug)]
enum Dest<'a> {
    None,
    Local(LocalVar<'a>),
    A,
    X,
}

#[derive(Debug)]
pub struct SamBlock {
    pub ops: Vec<SamLOp>,
    pub next_block_index: Option<usize>,
}

#[derive(Debug)]
pub struct SamBlockArena {
    pub blocks: Vec<SamBlock>,
}

impl SamBlockArena {
    pub fn new_block_writer(&mut self) -> SamBlockWriter {
        let new_block_index = self.blocks.len();
        self.blocks.push(SamBlock {
            ops: Vec::new(),
            next_block_index: None,
        });
        SamBlockWriter {
            arena: self,
            block_index: new_block_index,
        }
    }
}

#[derive(Debug)]
pub struct SamBlockWriter<'o> {
    arena: &'o mut SamBlockArena,
    block_index: usize,
}

impl<'o> SamBlockWriter<'o> {
    pub fn add_op(&mut self, op: SamLOp) {
        self.arena.blocks[self.block_index].ops.push(op);
    }

    pub fn set_next_block_index(&mut self, next_block_index: Option<usize>) {
        self.arena.blocks[self.block_index].next_block_index = next_block_index;
    }

    pub fn reborrow_mut(&mut self) -> SamBlockWriter {
        SamBlockWriter {
            arena: &mut self.arena,
            block_index: self.block_index,
        }
    }
}

impl<'a> Locals<'a> {
    fn get(&self, name: &'a str) -> LocalVar<'a> {
        self.locals
            .get(name)
            .expect(&format!("Accessing unknown local {:?}", name))
            .clone()
    }

    fn create(&mut self, name: Option<&'a str>, typ: &VarType) -> LocalVar<'a> {
        let result = LocalVar {
            name: name.unwrap_or("$temp"),
            typ: typ.clone(),
            location: self.cur_stack_size,
        };
        if let Some(name) = name {
            self.locals.insert(name, result.clone());
        }
        self.cur_stack_size += type_size(&typ);
        result
    }

    fn new_named(&mut self, name: &'a str, typ: &VarType) -> LocalVar<'a> {
        self.create(Some(name), typ)
    }

    fn new_temp(&mut self, typ: &VarType) -> LocalVar<'a> {
        self.create(None, typ)
    }
}

fn type_size(typ: &VarType) -> u32 {
    match typ {
        VarType::U8 => 1,
        VarType::Bool => 1,
        VarType::U32 => 4,
        VarType::Unit => 0,
        VarType::StringLiteral => 0,
        VarType::PtrTo(_) => 4,
    }
}

fn are_types_compatible(type1: &VarType, type2: &VarType) -> bool {
    // be generous
    type_size(type1) == type_size(type2)
}

struct SamCpu<'a, 'o> {
    locals: Locals<'a>,
    out: SamBlockWriter<'o>,
    cur_b_offset: u32,
    fn_decls: &'a BTreeMap<String, FnDecl>,
    valret_local: LocalVar<'a>,
    iret_local: LocalVar<'a>,
}

impl<'a, 'o> SamCpu<'a, 'o> {
    pub fn new(
        fn_decls: &'a BTreeMap<String, FnDecl>,
        fn_name: &'a str,
        arena: &'o mut SamBlockArena,
    ) -> SamCpu<'a, 'o> {
        let decl = fn_decls.get(fn_name).expect("Compiling unknown function");
        let mut locals = Locals {
            locals: BTreeMap::new(),
            cur_stack_size: 0,
        };
        let valret_local = locals.new_temp(&decl.ret);
        for arg in &decl.args {
            locals.new_named(&arg.name, &arg.typ);
        }
        let iret_local = locals.new_temp(&VarType::U32);
        SamCpu {
            locals,
            out: arena.new_block_writer(),
            cur_b_offset: iret_local.location,
            fn_decls,
            valret_local,
            iret_local,
        }
    }

    pub fn scope<R>(&mut self, f: impl for<'b, 'o2> FnOnce(&'b mut SamCpu<'a, 'o2>) -> R) -> R {
        let (rust_closure_return, cpu_b_offset, cpu_block_index) = {
            let mut cpu = SamCpu {
                locals: self.locals.clone(),
                out: self.out.reborrow_mut(),
                cur_b_offset: self.cur_b_offset,
                fn_decls: self.fn_decls,
                valret_local: self.valret_local.clone(),
                iret_local: self.iret_local.clone(),
            };
            (f(&mut cpu), cpu.cur_b_offset, cpu.out.block_index)
        };
        self.out.block_index = cpu_block_index;
        self.cur_b_offset = cpu_b_offset;
        rust_closure_return
    }

    pub fn block(
        &mut self,
        f: impl for<'b, 'o2> FnOnce(&'b mut SamCpu<'a, 'o2>),
    ) -> (usize, usize) {
        let child_out = self.out.arena.new_block_writer();
        let mut cpu = SamCpu {
            locals: self.locals.clone(),
            out: child_out,
            cur_b_offset: self.cur_b_offset,
            fn_decls: self.fn_decls,
            valret_local: self.valret_local.clone(),
            iret_local: self.iret_local.clone(),
        };
        let entry_index = cpu.out.block_index;
        f(&mut cpu);
        self.cur_b_offset = cpu.cur_b_offset;
        (entry_index, cpu.out.block_index)
    }

    pub fn split_to_new_block(&mut self) -> (usize, usize) {
        let old_block_index = self.out.block_index;
        let new_block_index = self.out.arena.new_block_writer().block_index;
        self.out.set_next_block_index(Some(new_block_index));
        self.out.block_index = new_block_index;
        (old_block_index, new_block_index)
    }

    pub fn goto_b_offset(&mut self, offset: u32) {
        if self.cur_b_offset < offset {
            self.out.add_op(SamLOp::Simple(SamSOp::AddConstToB(
                offset - self.cur_b_offset,
            )));
        } else if offset < self.cur_b_offset {
            self.out.add_op(SamLOp::Simple(SamSOp::SubConstFromB(
                self.cur_b_offset - offset,
            )));
        }
        self.cur_b_offset = offset;
    }

    pub fn get_expr_type(&self, expr: &'a Expr) -> Option<VarType> {
        match expr {
            Expr::Literal(_lit) => None,
            Expr::VarRef(varref) => Some(self.locals.get(varref).typ.clone()),
            Expr::BinOp(binop) => {
                if let BinOpKind::Cmp(_) = binop.kind {
                    Some(VarType::U8)
                } else {
                    let a_type = self.get_expr_type(&binop.args.0);
                    let b_type = self.get_expr_type(&binop.args.1);
                    match a_type {
                        Some(a_type) => match b_type {
                            Some(b_type) => {
                                if a_type == b_type {
                                    Some(a_type)
                                } else {
                                    panic!("Binop on incompatible types");
                                }
                            }
                            None => Some(a_type),
                        },
                        None => b_type,
                    }
                }
            }
            Expr::FnCall(f) => Some(if let Some(builtin_fn) = get_builtin_fn(&f.fn_name) {
                builtin_fn.ret_type
            } else {
                self.fn_decls
                    .get(&f.fn_name)
                    .expect(&format!("Calling unknown function {:?}", f.fn_name))
                    .ret
                    .clone()
            }),
            Expr::Scope(s) => match &s.final_expr {
                Some(e) => self.get_expr_type(e),
                None => Some(VarType::Unit),
            },
            Expr::IfElse(s) => {
                let true_type = self.get_expr_type(&s.if_true);
                let false_type = self.get_expr_type(&s.if_false);
                match true_type {
                    Some(true_type) => match false_type {
                        Some(false_type) => {
                            if true_type == false_type {
                                Some(true_type)
                            } else {
                                panic!("Incompatible match arms in if/else");
                            }
                        }
                        None => Some(true_type),
                    },
                    None => false_type,
                }
            }
            Expr::StringLiteral(_) => Some(VarType::StringLiteral),
            Expr::Deref(e) => {
                let ptr_type = self.get_expr_type(e);
                if let Some(ptr_type) = ptr_type {
                    match ptr_type {
                        VarType::PtrTo(pointed_type) => Some((*pointed_type).clone()),
                        _ => panic!(
                            "Dereferencing {:?} which is not a pointer but a {:?}",
                            e, ptr_type
                        ),
                    }
                } else {
                    None
                }
            }
            Expr::AddressOf(ident) => {
                let local = self.locals.get(&ident);
                Some(VarType::PtrTo(Box::new(local.typ.clone())))
            }
        }
    }

    pub fn set_x(&mut self, val: &BigUint) {
        self.out
            .add_op(SamLOp::Simple(SamSOp::SetX(biguint_to_u8(val))));
    }

    pub fn set_a(&mut self, val: &BigUint) {
        self.out
            .add_op(SamLOp::Simple(SamSOp::SetA(biguint_to_u32(val))));
    }

    pub fn write_x_at(&mut self, local: &LocalVar<'a>) {
        assert!(are_types_compatible(&local.typ, &VarType::U8));
        self.goto_b_offset(local.location);
        self.out.add_op(SamLOp::Simple(SamSOp::WriteXAtB));
    }

    pub fn write_a_at(&mut self, local: &LocalVar<'a>) {
        assert!(are_types_compatible(&local.typ, &VarType::U32));
        self.goto_b_offset(local.location);
        self.out.add_op(SamLOp::Simple(SamSOp::WriteAAtB));
    }

    pub fn read_x_at(&mut self, local: &LocalVar<'a>) {
        assert!(are_types_compatible(&local.typ, &VarType::U8));
        self.goto_b_offset(local.location);
        self.out.add_op(SamLOp::Simple(SamSOp::ReadXAtB));
    }

    pub fn read_a_at(&mut self, local: &LocalVar<'a>) {
        assert!(are_types_compatible(&local.typ, &VarType::U32));
        self.goto_b_offset(local.location);
        self.out.add_op(SamLOp::Simple(SamSOp::ReadAAtB));
    }

    pub fn copy_local_to_local(&mut self, a: &LocalVar<'a>, b: &LocalVar<'a>) {
        assert!(are_types_compatible(&a.typ, &b.typ));
        if a.location == b.location {
            return;
        }
        match &a.typ {
            VarType::Unit => {}
            VarType::U8 | VarType::Bool => {
                self.read_x_at(a);
                self.write_x_at(b);
            }
            VarType::U32 | VarType::PtrTo(_) => {
                self.read_a_at(a);
                self.write_a_at(b);
            }
            VarType::StringLiteral => {}
        }
    }

    pub fn ret(&mut self, val: Option<&'a Expr>) {
        if let Some(val) = val {
            self.eval_expr(val, &Dest::Local(self.valret_local.clone()));
        }
        self.goto_b_offset(self.iret_local.location);
        self.out.add_op(SamLOp::Simple(SamSOp::Ret));
    }

    pub fn eval_expr(&mut self, expr: &'a Expr, dest: &Dest<'a>) {
        //let expr_type = self.get_expr_type(expr);
        match expr {
            Expr::Literal(lit) => match dest {
                Dest::None => {}
                Dest::X => {
                    self.set_x(lit);
                }
                Dest::A => {
                    self.set_a(lit);
                }
                Dest::Local(local) => {
                    if let Some(expr_type) = self.get_expr_type(expr) {
                        assert!(are_types_compatible(&local.typ, &expr_type));
                    }
                    match &local.typ {
                        VarType::Unit => unreachable!(),
                        VarType::U8 | VarType::Bool => {
                            self.set_x(lit);
                            self.write_x_at(local);
                        }
                        VarType::U32 | VarType::PtrTo(_) => {
                            self.set_a(lit);
                            self.write_a_at(local);
                        }
                        VarType::StringLiteral => unreachable!(),
                    }
                }
            },
            Expr::VarRef(varref) => {
                let varref_local = self.locals.get(varref);
                match dest {
                    Dest::None => {}
                    Dest::X => self.read_x_at(&varref_local),
                    Dest::A => self.read_a_at(&varref_local),
                    Dest::Local(local) => {
                        self.copy_local_to_local(&varref_local, local);
                    }
                }
            }
            Expr::BinOp(binop) => {
                let maybe_typ = self.get_expr_type(expr);
                let result_typ = match dest {
                    Dest::None => {
                        if let Some(typ) = maybe_typ {
                            typ
                        } else {
                            panic!("Unknown type for binop!");
                        }
                    }
                    Dest::X => {
                        if let Some(typ) = maybe_typ {
                            assert!(are_types_compatible(&typ, &VarType::U8));
                            typ
                        } else {
                            VarType::U8
                        }
                    }
                    Dest::A => {
                        if let Some(typ) = maybe_typ {
                            assert!(are_types_compatible(&typ, &VarType::U32));
                            typ
                        } else {
                            VarType::U32
                        }
                    }
                    Dest::Local(local) => {
                        if let Some(typ) = maybe_typ {
                            assert!(are_types_compatible(&typ, &local.typ));
                            typ
                        } else {
                            local.typ.clone()
                        }
                    }
                };
                let arg_typ = if let BinOpKind::Cmp(_) = binop.kind {
                    let a_type = self.get_expr_type(&binop.args.0);
                    let b_type = self.get_expr_type(&binop.args.1);
                    match a_type {
                        Some(a_type) => match b_type {
                            Some(b_type) => {
                                if a_type == b_type {
                                    a_type
                                } else {
                                    panic!("Comparison on incompatible types");
                                }
                            }
                            None => a_type,
                        },
                        None => b_type.unwrap_or(result_typ.clone()),
                    }
                } else {
                    result_typ.clone()
                };
                self.scope(|cpu| {
                    let lhs_local = cpu.locals.new_temp(&arg_typ);
                    cpu.eval_expr(&binop.args.0, &Dest::Local(lhs_local.clone()));
                    match &arg_typ {
                        VarType::U8 | VarType::Bool => {
                            cpu.eval_expr(&binop.args.1, &Dest::X);
                            cpu.goto_b_offset(lhs_local.location);
                            match binop.kind {
                                BinOpKind::Plus => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::AddU8AtBToX));
                                }
                                BinOpKind::Minus => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::NegX));
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::AddU8AtBToX));
                                }
                                BinOpKind::Mul => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::MulU8AtBToX));
                                }
                                BinOpKind::Div => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::SetXToU8AtBDivByX));
                                }
                                BinOpKind::Mod => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::SetXToU8AtBModX));
                                }
                                BinOpKind::Cmp(cmp_kind) => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::CmpU8AtBWithX));
                                    process_cmp_result(cpu, cmp_kind);
                                }
                            }
                        }
                        VarType::U32 | VarType::PtrTo(_) => {
                            cpu.eval_expr(&binop.args.1, &Dest::A);
                            cpu.goto_b_offset(lhs_local.location);
                            match binop.kind {
                                BinOpKind::Plus => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::AddU32AtBToA));
                                }
                                BinOpKind::Minus => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::NegA));
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::AddU32AtBToA));
                                }
                                BinOpKind::Mul => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::MulU32AtBToA));
                                }
                                BinOpKind::Div => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::SetAToU32AtBDivByA));
                                }
                                BinOpKind::Mod => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::SetAToU32AtBModA));
                                }
                                BinOpKind::Cmp(cmp_kind) => {
                                    cpu.out.add_op(SamLOp::Simple(SamSOp::CmpU32AtBWithA));
                                    process_cmp_result(cpu, cmp_kind);
                                }
                            }
                        }
                        VarType::Unit => {
                            panic!("Unit binop?")
                        }
                        VarType::StringLiteral => {
                            panic!("Scope with type string literal?")
                        }
                    }
                });
                match &result_typ {
                    VarType::U8 | VarType::Bool => {
                        match dest {
                            Dest::None => {}
                            Dest::X => {
                                // result is already in x
                            }
                            Dest::A => {
                                panic!("Writing U8 to A?")
                            }
                            Dest::Local(l) => {
                                self.write_x_at(l);
                            }
                        }
                    }
                    VarType::U32 | VarType::PtrTo(_) => {
                        match dest {
                            Dest::None => {}
                            Dest::X => {
                                panic!("Writing U32 to X?")
                            }
                            Dest::A => {
                                // result is already in a
                            }
                            Dest::Local(l) => {
                                self.write_a_at(l);
                            }
                        }
                    }
                    VarType::Unit => {
                        panic!("Unit binop?")
                    }
                    VarType::StringLiteral => {
                        panic!("Binop involving string literal not implemented")
                    }
                }
            }
            Expr::FnCall(fncall) => {
                self.call(fncall, dest);
            }
            Expr::Scope(s) => {
                self.scope(|cpu| {
                    for stmt in &s.stmts {
                        cpu.exec_stmt(stmt);
                    }
                    if let Some(final_expr) = &s.final_expr {
                        cpu.eval_expr(final_expr, dest);
                    } else {
                        match dest {
                            Dest::None => {}
                            Dest::X => {
                                panic!("Scope has no final expression but evals to X!");
                            }
                            Dest::A => {
                                panic!("Scope has no final expression but evals to A!");
                            }
                            Dest::Local(local) => {
                                assert_eq!(local.typ, VarType::Unit);
                            }
                        }
                    }
                });
            }
            Expr::IfElse(i) => {
                self.eval_expr(&i.cond, &Dest::X);
                let start_b_offset = self.cur_b_offset;
                let (true_entry_index, true_exit_index) = self.block(|cpu| {
                    cpu.eval_expr(&i.if_true, dest);
                });
                let end_b_offset = self.cur_b_offset;
                self.cur_b_offset = start_b_offset;
                let (false_entry_index, false_exit_index) = self.block(|cpu| {
                    cpu.eval_expr(&i.if_false, dest);
                    cpu.goto_b_offset(end_b_offset);
                });
                self.out.add_op(SamLOp::JmpToBlockIfX(true_entry_index));
                let (old_index, new_index) = self.split_to_new_block();
                self.out.arena.blocks[old_index].next_block_index = Some(false_entry_index);
                self.out.arena.blocks[true_exit_index].next_block_index = Some(new_index);
                self.out.arena.blocks[false_exit_index].next_block_index = Some(new_index);
            }
            Expr::StringLiteral(_) => {}
            Expr::Deref(e) => {
                self.eval_expr(e, &Dest::A);
                self.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                self.out.add_op(SamLOp::Simple(SamSOp::CopyAToB));
                match dest {
                    Dest::None => {}
                    Dest::X => {
                        self.out.add_op(SamLOp::Simple(SamSOp::ReadXAtB));
                        self.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                    }
                    Dest::A => {
                        self.out.add_op(SamLOp::Simple(SamSOp::ReadAAtB));
                        self.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                    }
                    Dest::Local(local) => match &local.typ {
                        VarType::Unit => {}
                        VarType::U8 | VarType::Bool => {
                            self.out.add_op(SamLOp::Simple(SamSOp::ReadXAtB));
                            self.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                            self.write_x_at(local);
                        }
                        VarType::U32 | VarType::PtrTo(_) => {
                            self.out.add_op(SamLOp::Simple(SamSOp::ReadAAtB));
                            self.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                            self.write_a_at(local);
                        }
                        VarType::StringLiteral => {}
                    },
                }
            }
            Expr::AddressOf(i) => {
                let local = self.locals.get(i);
                self.goto_b_offset(local.location);
                self.out.add_op(SamLOp::Simple(SamSOp::CopyBToA));
                match dest {
                    Dest::None => {}
                    Dest::X => {
                        panic!("Reading address of {} into X?", i)
                    }
                    Dest::A => {
                        // it's already in A
                    }
                    Dest::Local(local) => match &local.typ {
                        VarType::Unit => {}
                        VarType::U8 => {
                            panic!("Writing address of {} into U8 local {}", i, local.name)
                        }
                        VarType::Bool => {
                            panic!("Writing address of {} into bool local {}", i, local.name)
                        }
                        VarType::U32 => {
                            self.write_a_at(local);
                        }
                        VarType::StringLiteral => {}
                        VarType::PtrTo(_) => {
                            self.write_a_at(local);
                        }
                    },
                }
            }
        }
    }

    pub fn call(&mut self, fncall: &'a FnCall, dest: &Dest<'a>) {
        if fncall.fn_name == "print"
            || fncall.fn_name == "println"
            || fncall.fn_name == "print_char"
        {
            assert_eq!(fncall.args.len(), 1);
            let arg = &fncall.args[0];
            let typ = self.get_expr_type(arg).unwrap_or(VarType::U32);
            match typ {
                VarType::StringLiteral => {
                    if let Expr::StringLiteral(s) = arg {
                        for b in s.bytes() {
                            self.out.add_op(SamLOp::Simple(SamSOp::SetX(b)));
                            self.out.add_op(SamLOp::Simple(SamSOp::PrintCharX));
                        }
                    }
                    if fncall.fn_name == "println" {
                        self.out.add_op(SamLOp::Simple(SamSOp::SetX(10)));
                        self.out.add_op(SamLOp::Simple(SamSOp::PrintCharX));
                    }
                }
                VarType::U8 | VarType::Bool => {
                    self.eval_expr(arg, &Dest::X);
                    if let VarType::Bool = &typ {
                        self.out.add_op(SamLOp::Simple(SamSOp::NotX));
                        self.out.add_op(SamLOp::Simple(SamSOp::NotX));
                    }
                    if fncall.fn_name == "print" {
                        self.out.add_op(SamLOp::Simple(SamSOp::MoveXToA));
                        self.out.add_op(SamLOp::Simple(SamSOp::PrintA));
                    } else if fncall.fn_name == "println" {
                        self.out.add_op(SamLOp::Simple(SamSOp::MoveXToA));
                        self.out.add_op(SamLOp::Simple(SamSOp::PrintA));
                        self.out.add_op(SamLOp::Simple(SamSOp::SetX(10)));
                        self.out.add_op(SamLOp::Simple(SamSOp::PrintCharX));
                    } else if fncall.fn_name == "print_char" {
                        self.out.add_op(SamLOp::Simple(SamSOp::PrintCharX));
                    } else {
                        unimplemented!()
                    }
                }
                VarType::U32 | VarType::PtrTo(_) => {
                    self.eval_expr(arg, &Dest::A);
                    if fncall.fn_name == "print" {
                        self.out.add_op(SamLOp::Simple(SamSOp::PrintA));
                    } else if fncall.fn_name == "println" {
                        self.out.add_op(SamLOp::Simple(SamSOp::PrintA));
                        self.out.add_op(SamLOp::Simple(SamSOp::SetX(10)));
                        self.out.add_op(SamLOp::Simple(SamSOp::PrintCharX));
                    } else if fncall.fn_name == "print_char" {
                        panic!("U32 is not a character")
                    } else {
                        unimplemented!()
                    }
                }
                VarType::Unit => {
                    panic!("Printing unit");
                }
            }
        } else if fncall.fn_name == "read_char" {
            self.out.add_op(SamLOp::Simple(SamSOp::StdinX));
            match dest {
                Dest::None => {}
                Dest::X => {
                    // it's already in X
                }
                Dest::A => {
                    self.out.add_op(SamLOp::Simple(SamSOp::MoveXToA));
                }
                Dest::Local(local) => match &local.typ {
                    VarType::Unit => {}
                    VarType::U8 | VarType::Bool => {
                        self.write_x_at(local);
                    }
                    VarType::U32 | VarType::PtrTo(_) => {
                        self.out.add_op(SamLOp::Simple(SamSOp::StdinX));
                        self.write_a_at(local);
                    }
                    VarType::StringLiteral => {}
                },
            }
        } else {
            let fn_decl = self
                .fn_decls
                .get(&fncall.fn_name)
                .expect(&format!("Calling unknown function {}", fncall.fn_name));
            assert_eq!(fn_decl.args.len(), fncall.args.len());
            let valret_local = self.scope(|cpu| {
                let valret_local = cpu.locals.new_temp(&fn_decl.ret);
                for (arg_expr, arg_decl) in fncall.args.iter().zip(fn_decl.args.iter()) {
                    let arg_local = cpu.locals.new_temp(&arg_decl.typ);
                    cpu.scope(|cpu| {
                        cpu.eval_expr(arg_expr, &Dest::Local(arg_local));
                    });
                }
                let iret_local = cpu.locals.new_temp(&VarType::U32);
                cpu.goto_b_offset(iret_local.location);
                cpu.out.add_op(SamLOp::Call(fn_decl.name.clone()));
                valret_local
            });
            match dest {
                Dest::None => {}
                Dest::Local(dest_local) => {
                    self.copy_local_to_local(&valret_local, dest_local);
                }
                Dest::A => self.read_a_at(&valret_local),
                Dest::X => self.read_x_at(&valret_local),
            }
        }
    }

    pub fn exec_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::VarDecl(decl) => {
                let local = self.locals.new_named(&decl.var_name, &decl.typ);
                self.eval_expr(&decl.init, &Dest::Local(local));
            }
            Stmt::VarAssign(ass) => match &ass.lhs {
                Expr::VarRef(s) => {
                    let local = self.locals.get(&s);
                    self.eval_expr(&ass.expr, &Dest::Local(local));
                }
                Expr::Deref(ptr_expr) => {
                    let lhs_typ = self.get_expr_type(&Expr::Deref(ptr_expr.clone()));
                    let rhs_typ = self.get_expr_type(&ass.expr);
                    let typ = if let Some(lhs_typ) = lhs_typ {
                        if let Some(rhs_typ) = rhs_typ {
                            assert!(
                                are_types_compatible(&lhs_typ, &rhs_typ),
                                "assigning incompatible types {:?} and {:?}",
                                &lhs_typ,
                                &rhs_typ
                            );
                            lhs_typ.clone()
                        } else {
                            lhs_typ.clone()
                        }
                    } else {
                        if let Some(rhs_typ) = rhs_typ {
                            rhs_typ.clone()
                        } else {
                            panic!("Assigning to pointer of unknown type: {:?}", ass);
                        }
                    };
                    self.scope(|cpu| {
                        //let ptr_local = cpu.locals.new_temp(&VarType::PtrTo(Box::new(typ.clone())));
                        let val_local = cpu.locals.new_temp(&typ);
                        cpu.eval_expr(&ass.expr, &Dest::Local(val_local.clone()));
                        cpu.eval_expr(ptr_expr, &Dest::A);
                        match typ {
                            VarType::Unit => {}
                            VarType::U8 | VarType::Bool => {
                                cpu.read_x_at(&val_local);
                                cpu.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                                cpu.out.add_op(SamLOp::Simple(SamSOp::CopyAToB));
                                cpu.out.add_op(SamLOp::Simple(SamSOp::WriteXAtB));
                                cpu.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                            }
                            VarType::U32 | VarType::PtrTo(_) => {
                                cpu.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                                cpu.out.add_op(SamLOp::Simple(SamSOp::CopyAToB));
                                cpu.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                                // now B is restored, C contains the ptr
                                cpu.read_a_at(&val_local);
                                cpu.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                                cpu.out.add_op(SamLOp::Simple(SamSOp::WriteAAtB));
                                cpu.out.add_op(SamLOp::Simple(SamSOp::SwapBAndC));
                            }
                            VarType::StringLiteral => {}
                        }
                    });
                }
                other => panic!("Invalid lhs {:?}", other),
            },
            Stmt::Expr(e) => {
                self.eval_expr(e, &Dest::None);
            }
            Stmt::IfMaybeElse(i) => {
                self.eval_expr(&i.cond, &Dest::X);
                let start_b_offset = self.cur_b_offset;
                let (true_entry_index, true_exit_index) = self.block(|cpu| {
                    cpu.eval_expr(&i.if_true, &Dest::None);
                });
                let end_b_offset = self.cur_b_offset;
                self.cur_b_offset = start_b_offset;
                let (false_entry_index, false_exit_index) = self.block(|cpu| {
                    if let Some(if_false) = &i.if_false {
                        cpu.eval_expr(if_false, &Dest::None);
                    }
                    cpu.goto_b_offset(end_b_offset);
                });
                self.out.add_op(SamLOp::JmpToBlockIfX(true_entry_index));
                let (old_index, new_index) = self.split_to_new_block();
                self.out.arena.blocks[old_index].next_block_index = Some(false_entry_index);
                self.out.arena.blocks[true_exit_index].next_block_index = Some(new_index);
                self.out.arena.blocks[false_exit_index].next_block_index = Some(new_index);
            }
            Stmt::WhileLoop(w) => {
                let start_b_offset = self.cur_b_offset;
                let (inner_entry_index, inner_exit_index) = self.block(|cpu| {
                    cpu.eval_expr(&w.inner, &Dest::None);
                    cpu.goto_b_offset(start_b_offset);
                });
                let (cond_entry_index, cond_exit_index) = self.block(|cpu| {
                    cpu.eval_expr(&w.cond, &Dest::X);
                    cpu.goto_b_offset(start_b_offset);
                    cpu.out.add_op(SamLOp::JmpToBlockIfX(inner_entry_index));
                });
                let (old_index, new_index) = self.split_to_new_block();
                self.out.arena.blocks[old_index].next_block_index = Some(cond_entry_index);
                self.out.arena.blocks[inner_exit_index].next_block_index = Some(cond_entry_index);
                self.out.arena.blocks[cond_exit_index].next_block_index = Some(new_index);
            }
            Stmt::Return(s) => {
                if let Some(ret_expr) = &s.expr {
                    if let Some(ret_expr_type) = self.get_expr_type(ret_expr) {
                        assert!(are_types_compatible(&ret_expr_type, &self.valret_local.typ));
                    }
                    self.ret(Some(ret_expr));
                } else {
                    assert_eq!(&self.valret_local.typ, &VarType::Unit);
                    self.ret(None);
                }
            }
        }
    }
}

fn process_cmp_result(cpu: &mut SamCpu, cmp_kind: CmpKind) {
    match cmp_kind {
        CmpKind::GT => {
            cpu.out.add_op(SamLOp::Simple(SamSOp::AddConstToX(255)));
            cpu.out.add_op(SamLOp::Simple(SamSOp::NotX));
        }
        CmpKind::GE => {
            cpu.out.add_op(SamLOp::Simple(SamSOp::AddConstToX(1)));
        }
        CmpKind::EQ => {
            cpu.out.add_op(SamLOp::Simple(SamSOp::NotX));
        }
        CmpKind::LT => {
            cpu.out.add_op(SamLOp::Simple(SamSOp::AddConstToX(1)));
            cpu.out.add_op(SamLOp::Simple(SamSOp::NotX));
        }
        CmpKind::LE => {
            cpu.out.add_op(SamLOp::Simple(SamSOp::AddConstToX(255)));
        }
        CmpKind::NE => {}
    }
}
