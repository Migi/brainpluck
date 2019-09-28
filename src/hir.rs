use nom::character::complete::digit1;
#[allow(unused_imports)]
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take, take_while},
    character::complete::{alphanumeric1 as alphanumeric, anychar, one_of},
    combinator::{complete, map, opt},
    error::{context, convert_error, ErrorKind, ParseError, VerboseError},
    multi::{separated_list, many0, many1, fold_many1},
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated},
    Err, IResult,
};
use std::collections::HashMap;
use num::BigUint;
use num::Num;

#[derive(Debug,Copy,Clone,Eq,PartialEq)]
pub enum BinOpKind {
    Plus,
    Minus,
    Mul,
    Div,
}

#[derive(Debug,Clone)]
pub struct BinOp {
    pub args: Box<(Expr, Expr)>,
    pub kind: BinOpKind,
}

#[derive(Debug,Clone)]
pub struct FnCall {
    pub fn_name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug,Clone)]
pub enum Expr {
    Literal(BigUint),
    VarRef(String),
    BinOp(BinOp),
    FnCall(FnCall),
    Scope(Scope),
    IfElse(Box<IfElse>)
}

#[derive(Debug,Copy,Clone,Eq,PartialEq)]
pub enum VarType {
    Unit,
    U8,
    U32
}

#[derive(Debug,Clone)]
pub struct VarDecl {
    pub var_name: String,
    pub typ: VarType,
    pub init: Expr,
}

#[derive(Debug,Clone)]
pub struct VarAssign {
    pub var_name: String,
    pub expr: Expr,
}

#[derive(Debug,Clone)]
pub struct Scope {
    pub stmts: Vec<Stmt>,
    pub final_expr: Option<Box<Expr>>
}

#[derive(Debug,Clone)]
pub struct IfMaybeElse {
    pub cond: Expr,
    pub if_true: Expr,
    pub if_false: Option<Expr>
}

#[derive(Debug,Clone)]
pub struct IfElse {
    pub cond: Expr,
    pub if_true: Expr,
    pub if_false: Expr
}

#[derive(Debug,Clone)]
pub enum Stmt {
    Expr(Expr),
    VarDecl(VarDecl),
    VarAssign(VarAssign),
    IfMaybeElse(IfMaybeElse)
}

#[derive(Debug,Clone)]
pub struct FnArgDecl {
    pub name: String,
    pub typ: VarType
}

#[derive(Debug,Clone)]
pub struct FnDecl {
    pub name: String,
    pub args: Vec<FnArgDecl>,
    pub ret: VarType,
    pub scope: Scope
}

#[derive(Debug)]
pub struct Program {
    pub fns: HashMap<String, FnDecl>
}

pub fn parse_hir<'a>(i: &'a str) -> Result<Program, nom::Err<VerboseError<&'a str>>> {
    let (i, stmts) = program::<VerboseError<&str>>(i)?;
    let (i, _) = ws::<VerboseError<&str>>(i)?;
    if i.len() > 0 {
        Err(nom::Err::Failure(VerboseError::from_error_kind(
            i,
            nom::error::ErrorKind::Complete,
        )))
    } else {
        Ok(stmts)
    }
}

fn ws<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let chars = " \t\r\n";
    take_while(move |c| chars.contains(c))(i)
}

fn biguint<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, BigUint, E> {
    let (i, _) = ws(i)?;
    map(digit1, |s| {
        Num::from_str_radix(s, 10).expect("nom::digit matched a non-int?")
    })(i)
}

fn factor<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Expr, E> {
    alt((
        map(biguint, |u| Expr::Literal(u)),
        map(if_else, |i| Expr::IfElse(Box::new(i))),
        map(fncall, |c| Expr::FnCall(c)),
        map(ident, |s| Expr::VarRef(s.to_owned())),
    ))(i)
}

fn term<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Expr, E> {
    let (i, a) = factor(i)?;
    let (i, _) = ws(i)?;
    let (i, kind) = opt(alt((tag("*"), tag("/"))))(i)?;
    let (i, _) = ws(i)?;
    match kind {
        Some(kind) => {
            let (i, b) = factor(i)?;
            let kind = match kind {
                "*" => BinOpKind::Mul,
                "/" => BinOpKind::Div,
                _ => unreachable!(),
            };
            Ok((
                i,
                Expr::BinOp(BinOp {
                    args: Box::new((a, b)),
                    kind,
                }),
            ))
        }
        None => Ok((i, a)),
    }
}

fn expr<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Expr, E> {
    let (i, a) = term(i)?;
    let (i, _) = ws(i)?;
    let (i, kind) = opt(alt((tag("+"), tag("-"))))(i)?;
    let (i, _) = ws(i)?;
    match kind {
        Some(kind) => {
            let (i, b) = term(i)?;
            let kind = match kind {
                "+" => BinOpKind::Plus,
                "-" => BinOpKind::Minus,
                _ => unreachable!(),
            };
            Ok((
                i,
                Expr::BinOp(BinOp {
                    args: Box::new((a, b)),
                    kind,
                }),
            ))
        }
        None => Ok((i, a)),
    }
}

fn ident<'a, E: nom::error::ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let (i, _) = ws(i)?;
    // check first char
    {
        let (i, first_char) = anychar(i)?;
        if first_char != '_' && !first_char.is_alphanumeric() {
            return Err(Err::Error(E::from_error_kind(
                i,
                nom::error::ErrorKind::AlphaNumeric,
            )));
        }
    }
    take_while(|c: char| c.is_alphanumeric() || c == '_')(i)
}

fn fncall<'a, E: nom::error::ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FnCall, E> {
    let (i, fn_name) = ident(i)?;
    let (i, _) = tag("(")(i)?;
    let (i, args) = separated_list(preceded(ws, tag(",")), expr)(i)?;
    let (i, _) = tag(")")(i)?;

    Ok((
        i,
        FnCall {
            fn_name: fn_name.to_owned(),
            args,
        },
    ))
}

fn type_name<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, VarType, E> {
    let (i, typ) = ident(i)?;
    let typ = {
        if typ == "u8" {
            VarType::U8
        } else if typ == "u32" {
            VarType::U32
        } else {
            panic!("Unknown variable type")
        }
    };
    Ok((i, typ))
}

fn scope<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Scope, E> {
    let (i, _) = ws(i)?;
    let (i, _) = tag("{")(i)?;
    let (i, stmts) = many0(stmt)(i)?;
    let (i, final_expr) = opt(expr)(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag("}")(i)?;
    Ok((
        i,
        Scope {
            stmts,
            final_expr: final_expr.map(|e| Box::new(e))
        }
    ))
}

fn if_maybe_else<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, IfMaybeElse, E> {
    let (i, _) = ws(i)?;
    let (i, _) = tag("if")(i)?;
    let (i, cond) = expr(i)?;
    let (i, if_true) = scope(i)?;
    let (i, if_false) = opt(preceded(ws, preceded(tag("else"), scope)))(i)?;
    Ok((
        i,
        IfMaybeElse {
            cond,
            if_true: Expr::Scope(if_true),
            if_false: if_false.map(|s| Expr::Scope(s))
        }
    ))
}

fn if_else<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, IfElse, E> {
    let (i, _) = ws(i)?;
    let (i, _) = tag("if")(i)?;
    let (i, cond) = expr(i)?;
    let (i, if_true) = scope(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag("else")(i)?;
    let (i, if_false) = scope(i)?;
    Ok((
        i,
        IfElse {
            cond,
            if_true: Expr::Scope(if_true),
            if_false: Expr::Scope(if_false)
        }
    ))
}

fn var_decl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, VarDecl, E> {
    let (i, _) = ws(i)?;
    let (i, _) = tag("let ")(i)?;
    let (i, var_name) = ident(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag(":")(i)?;
    let (i, typ) = type_name(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag("=")(i)?;
    let (i, init) = expr(i)?;

    Ok((
        i,
        VarDecl {
            var_name: var_name.to_owned(),
            typ,
            init
        }
    ))
}

fn var_assign<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, VarAssign, E> {
    let (i, var_name) = ident(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag("=")(i)?;
    let (i, expr) = expr(i)?;

    Ok((
        i,
        VarAssign {
            var_name: var_name.to_owned(),
            expr
        }
    ))
}

fn stmt<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Stmt, E> {
    let (i, stmt) = alt((
        map(if_maybe_else, |i| Stmt::IfMaybeElse(i)),
        map(var_decl, |d| Stmt::VarDecl(d)),
        map(var_assign, |a| Stmt::VarAssign(a)),
        map(expr, |e| Stmt::Expr(e)),
    ))(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag(";")(i)?;
    Ok((
        i,
        stmt
    ))
}

fn fn_arg_decl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FnArgDecl, E> {
    let (i, arg_name) = ident(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag(":")(i)?;
    let (i, typ) = type_name(i)?;
    Ok((
        i,
        FnArgDecl {
            name: arg_name.to_owned(),
            typ
        }
    ))
}

fn fn_decl<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, FnDecl, E> {
    let (i, _) = ws(i)?;
    let (i, _) = tag("fn")(i)?;
    let (i, fn_name) = ident(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag("(")(i)?;
    let (i, args) = separated_list(preceded(ws, tag(",")), fn_arg_decl)(i)?;
    let (i, _) = ws(i)?;
    let (i, _) = tag(")")(i)?;
    let (i, ret) = opt(preceded(preceded(ws, tag("->")),type_name))(i)?;
    let ret = ret.unwrap_or(VarType::Unit);
    let (i, scope) = scope(i)?;
    Ok((
        i,
        FnDecl {
            name: fn_name.to_owned(),
            args,
            ret,
            scope
        }
    ))
}

fn program<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Program, E> {
    let (i, fns) = fold_many1(
        fn_decl,
        HashMap::new(),
        |mut fns, new_fn| {
            if fns.contains_key(&new_fn.name) {
                panic!("Double definition for function");
            }
            fns.insert(new_fn.name.clone(), new_fn);
            fns
        }
    )(i)?;
    Ok((
        i,
        Program {
            fns
        }
    ))
}
