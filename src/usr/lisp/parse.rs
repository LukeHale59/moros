use super::{Err, Exp, Number};
use crate::could_not;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;

use nom::Err::Error;
use nom::IResult;
use nom::branch::alt;
use nom::bytes::complete::escaped_transform;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_while1;
use nom::character::complete::char;
use nom::character::complete::multispace0;
use nom::combinator::map;
use nom::combinator::opt;
use nom::combinator::recognize;
use nom::combinator::value;
use nom::multi::many0;
use nom::sequence::delimited;
use nom::sequence::preceded;
use nom::sequence::tuple;
use nom::character::complete::one_of;
use nom::multi::many1;
use nom::sequence::terminated;

// https://docs.rs/nom/latest/nom/recipes/index.html#hexadecimal
fn hexadecimal(input: &str) -> IResult<&str, &str> {
    preceded(
        tag("0x"),
        recognize(
            many1(
                terminated(one_of("0123456789abcdefABCDEF"), many0(char('_')))
            )
        )
    )(input)
}

// https://docs.rs/nom/latest/nom/recipes/index.html#decimal
fn decimal(input: &str) -> IResult<&str, &str> {
    recognize(
        many1(
            terminated(one_of("0123456789"), many0(char('_')))
        )
    )(input)
}

// https://docs.rs/nom/latest/nom/recipes/index.html#binary
fn binary(input: &str) -> IResult<&str, &str> {
    preceded(
        tag("0b"),
        recognize(
            many1(
                terminated(one_of("01"), many0(char('_')))
            )
        )
    )(input)
}

// https://docs.rs/nom/latest/nom/recipes/index.html#floating-point-numbers
fn float(input: &str) -> IResult<&str, &str> {
    alt((
        recognize( // .42
            tuple((
                char('.'),
                decimal,
                opt(tuple((
                    one_of("eE"),
                    opt(one_of("+-")),
                    decimal
                )))
            ))
        ),
        recognize( // 42e42 and 42.42e42
            tuple((
                decimal,
                opt(preceded(
                    char('.'),
                    decimal,
                )),
                one_of("eE"),
                opt(one_of("+-")),
                decimal
            ))
        ),
        recognize( // 42. and 42.42
            tuple((
                decimal,
                char('.'),
                opt(decimal)
            ))
        )
    ))(input)
}

fn is_symbol_letter(c: char) -> bool {
    let chars = "<>=-+*/%^?.";
    c.is_alphanumeric() || chars.contains(c)
}

fn parse_str(input: &str) -> IResult<&str, Exp> {
    let escaped = map(opt(escaped_transform(is_not("\\\""), '\\', alt((
        value("\\", tag("\\")),
        value("\"", tag("\"")),
        value("\n", tag("n")),
        value("\r", tag("r")),
        value("\t", tag("t")),
    )))), |inner| inner.unwrap_or("".to_string()));
    let (input, s) = delimited(char('"'), escaped, char('"'))(input)?;
    Ok((input, Exp::Str(s)))
}

fn parse_sym(input: &str) -> IResult<&str, Exp> {
    let (input, sym) = take_while1(is_symbol_letter)(input)?;
    Ok((input, Exp::Sym(sym.to_string())))
}

fn parse_num(input: &str) -> IResult<&str, Exp> {
    let (input, num) = recognize(tuple((
        opt(alt((char('+'), char('-')))),
        alt((float, hexadecimal, binary, decimal))
    )))(input)?;
    Ok((input, Exp::Num(Number::from(num))))
}

fn parse_bool(input: &str) -> IResult<&str, Exp> {
    let (input, s) = alt((tag("true"), tag("false")))(input)?;
    Ok((input, Exp::Bool(s == "true")))
}

fn parse_list(input: &str) -> IResult<&str, Exp> {
    let (input, list) = delimited(char('('), many0(parse_exp), char(')'))(input)?;
    Ok((input, Exp::List(list)))
}

fn parse_quote(input: &str) -> IResult<&str, Exp> {
    let (input, list) = preceded(char('\''), parse_exp)(input)?;
    let list = vec![Exp::Sym("quote".to_string()), list];
    Ok((input, Exp::List(list)))
}

fn parse_unquote_splice(input: &str) -> IResult<&str, Exp> {
    let (input, list) = preceded(tag(",@"), parse_exp)(input)?;
    let list = vec![Exp::Sym("unquote-splice".to_string()), list];
    Ok((input, Exp::List(list)))
}

fn parse_splice(input: &str) -> IResult<&str, Exp> {
    let (input, list) = preceded(tag("@"), parse_exp)(input)?;
    let list = vec![Exp::Sym("splice".to_string()), list];
    Ok((input, Exp::List(list)))
}

fn parse_unquote(input: &str) -> IResult<&str, Exp> {
    let (input, list) = preceded(char(','), parse_exp)(input)?;
    let list = vec![Exp::Sym("unquote".to_string()), list];
    Ok((input, Exp::List(list)))
}

fn parse_quasiquote(input: &str) -> IResult<&str, Exp> {
    let (input, list) = preceded(char('`'), parse_exp)(input)?;
    let list = vec![Exp::Sym("quasiquote".to_string()), list];
    Ok((input, Exp::List(list)))
}

fn parse_comment(input: &str) -> IResult<&str, &str> {
    preceded(multispace0, preceded(char('#'), is_not("\n")))(input)
}

fn parse_exp(input: &str) -> IResult<&str, Exp> {
    let (input, _) = opt(many0(parse_comment))(input)?;
    delimited(multispace0, alt((
        parse_num, parse_bool, parse_str, parse_list, parse_quote, parse_quasiquote, parse_unquote_splice, parse_unquote, parse_splice, parse_sym
    )), alt((parse_comment, multispace0)))(input)
}

pub fn parse(input: &str)-> Result<(String, Exp), Err> {
    match parse_exp(input) {
        Ok((input, exp)) => Ok((input.to_string(), exp)),
        Err(Error(err)) => {
            if err.input.is_empty() {
                Ok(("".to_string(), Exp::List(vec![Exp::Sym("quote".to_string()), Exp::List(vec![])])))
            } else {
                let line = err.input.lines().next().unwrap();
                could_not!("parse '{}'", line)
            }
        }
        _ => could_not!("parse input"),
    }
}
