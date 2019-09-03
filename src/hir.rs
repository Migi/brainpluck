use nom::character::complete::digit1;
#[allow(unused_imports)]
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take, take_while},
    character::complete::{alphanumeric1 as alphanumeric, anychar, one_of},
    combinator::{complete, map, opt},
    error::{context, convert_error, ErrorKind, ParseError, VerboseError},
    multi::separated_list,
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated},
    Err, IResult,
};
use num::BigUint;
use num::Num;

#[derive(Debug)]
pub enum BinOpKind {
    Plus,
    Minus,
    Mul,
    Div,
}

#[derive(Debug)]
pub struct BinOp {
    args: Box<(Expr, Expr)>,
    kind: BinOpKind,
}

#[derive(Debug)]
pub struct FnCall {
    fn_name: String,
    args: Vec<Expr>,
}

#[derive(Debug)]
pub enum Expr {
    Literal(BigUint),
    BinOp(BinOp),
    FnCall(FnCall),
}

pub fn parse_hir<'a>(i: &'a str) -> Result<Expr, nom::Err<VerboseError<&'a str>>> {
    let (i, expr) = expr::<VerboseError<&str>>(i)?;
    let (i, _) = ws::<VerboseError<&str>>(i)?;
    if i.len() > 0 {
        Err(nom::Err::Failure(VerboseError::from_error_kind(
            i,
            nom::error::ErrorKind::Complete,
        )))
    } else {
        Ok(expr)
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
        map(fncall, |c| Expr::FnCall(c)),
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
