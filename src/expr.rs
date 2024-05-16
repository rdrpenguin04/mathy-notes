use core::{fmt, iter::Peekable};

#[derive(Debug)]
pub enum Error {
    Unrecognized,
    Invalid,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Unrecognized => "<unrecognized operator>",
            Self::Invalid => "<invalid expression>",
        }
        .fmt(f)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Eq, PartialEq)]
enum TokenType {
    Num,
    Id,
    Sym,
}

#[derive(Debug)]
struct Token {
    text: String,
    ty: TokenType,
}

#[derive(Debug)]
struct Group {
    inner: Vec<Lexeme>,
}

#[derive(Debug)]
enum Lexeme {
    Token(Token),
    Group(Group),
}

fn lex<I: Iterator<Item = char>>(text: &mut Peekable<I>, term: char) -> Result<Vec<Lexeme>> {
    let mut result = Vec::new();
    while let Some(&x) = text.peek() {
        match x {
            x if x.is_alphabetic() => {
                let mut token = String::from(x);
                text.next();
                while let Some(x) = text.peek() {
                    if x.is_alphanumeric() {
                        token.push(*x);
                        text.next();
                    } else {
                        break;
                    }
                }
                result.push(Lexeme::Token(Token {
                    text: token,
                    ty: TokenType::Id,
                }));
            }
            x if x.is_numeric() || x == '.' => {
                let mut token = String::from(x);
                text.next();
                while let Some(x) = text.peek() {
                    if x.is_alphanumeric() || *x == '.' {
                        token.push(*x);
                        text.next();
                    } else {
                        break;
                    }
                }
                result.push(Lexeme::Token(Token {
                    text: token,
                    ty: TokenType::Num,
                }));
            }
            '+' | '-' | '/' | '^' => {
                text.next();
                result.push(Lexeme::Token(Token {
                    text: x.into(),
                    ty: TokenType::Sym,
                }));
            }
            '*' => {
                text.next();
                if text.peek() == Some(&'*') {
                    text.next();
                    result.push(Lexeme::Token(Token {
                        text: "**".into(),
                        ty: TokenType::Sym,
                    }));
                } else {
                    result.push(Lexeme::Token(Token {
                        text: "*".into(),
                        ty: TokenType::Sym,
                    }));
                }
            }
            '(' => {
                text.next();
                let inner = lex(text, ')')?;
                result.push(Lexeme::Group(Group { inner }));
            }
            x if x == term => {
                text.next();
                break;
            }
            x if x.is_whitespace() => {
                text.next();
            }
            _ => Err(Error::Unrecognized)?,
        }
    }
    Ok(result)
}

enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

enum UnOp {
    Fn(Box<dyn Fn(f64) -> Result<f64>>),
    Pos,
    Neg,
}

impl UnOp {
    fn func(func: impl Fn(f64) -> Result<f64> + 'static) -> Self {
        Self::Fn(Box::new(func))
    }
}

// impl fmt::Debug for UnOp {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Self::Fn(_) => write!(f, "fn()"),
//             Self::Pos => write!(f, "+"),
//             Self::Neg => write!(f, "-"),
//         }
//     }
// }

enum Expression {
    BinOp {
        lhs: Box<Expression>,
        op: BinOp,
        rhs: Box<Expression>,
    },
    UnOp {
        op: UnOp,
        inner: Box<Expression>,
    },
    Num(f64),
}

impl Expression {
    fn func(func: impl Fn(f64) -> Result<f64> + 'static, arg: Self) -> Self {
        Self::UnOp {
            op: UnOp::func(func),
            inner: Box::new(arg),
        }
    }
}

impl Expression {
    fn eval(&self) -> Result<f64> {
        Ok(match self {
            Self::BinOp { lhs, op, rhs } => match op {
                BinOp::Add => lhs.eval()? + rhs.eval()?,
                BinOp::Sub => lhs.eval()? - rhs.eval()?,
                BinOp::Mul => lhs.eval()? * rhs.eval()?,
                BinOp::Div => lhs.eval()? / rhs.eval()?,
                BinOp::Pow => lhs.eval()?.powf(rhs.eval()?),
            },
            Self::UnOp { op, inner } => match op {
                UnOp::Pos => inner.eval()?,
                UnOp::Neg => -inner.eval()?,
                UnOp::Fn(x) => x(inner.eval()?)?,
            },
            Self::Num(x) => *x,
        })
    }
}

fn bin_bp(op: &str) -> (u8, u8) {
    match op {
        "+" | "-" => (1, 2),
        " " => (3, 4),
        "*" | "/" => (5, 6),
        "^" | "**" => (8, 7),
        _ => unreachable!(),
    }
}

fn parse_num(text: &str) -> Result<f64> {
    let mut int_part = 0.0;
    let mut chars = text.chars();
    for c in &mut chars {
        match c {
            '0'..='9' => {
                int_part *= 10.0;
                int_part += f64::from(c as u32 - '0' as u32);
            }
            '.' => break,
            _ => Err(Error::Invalid)?,
        }
    }
    let mut float_part = 0.0;
    let mut multiplier = 0.1;
    for c in &mut chars {
        match c {
            '0'..='9' => {
                float_part += f64::from(c as u32 - '0' as u32) * multiplier;
                multiplier /= 10.0;
            }
            '.' => break,
            _ => Err(Error::Invalid)?,
        }
    }
    Ok(int_part + float_part)
}

fn parse_arg(iter: &mut Peekable<impl Iterator<Item = &Lexeme>>) -> Result<Expression> {
    match iter.peek() {
        Some(Lexeme::Group(_)) => parse_atom(iter),
        _ => parse_bp(iter, 4),
    }
}

fn parse_atom(iter: &mut Peekable<impl Iterator<Item = &Lexeme>>) -> Result<Expression> {
    Ok(match iter.next() {
        Some(Lexeme::Token(Token {
            ty: TokenType::Num,
            text,
        })) => Expression::Num(parse_num(text)?),
        Some(Lexeme::Token(Token {
            ty: TokenType::Id,
            text,
        })) => match &**text {
            "sin" => Expression::func(|x| Ok(x.sin()), parse_arg(iter)?),
            "cos" => Expression::func(|x| Ok(x.cos()), parse_arg(iter)?),
            "tan" => Expression::func(|x| Ok(x.tan()), parse_arg(iter)?),
            "sec" => Expression::func(|x| Ok(1.0 / x.cos()), parse_arg(iter)?),
            "csc" => Expression::func(|x| Ok(1.0 / x.sin()), parse_arg(iter)?),
            "cot" => Expression::func(|x| Ok(1.0 / x.tan()), parse_arg(iter)?),
            "asin" | "arcsin" => Expression::func(|x| Ok(x.asin()), parse_arg(iter)?),
            "acos" | "arccos" => Expression::func(|x| Ok(x.acos()), parse_arg(iter)?),
            "atan" | "arctan" => Expression::func(|x| Ok(x.atan()), parse_arg(iter)?),
            "asec" | "arcsec" => Expression::func(|x| Ok((1.0 / x).acos()), parse_arg(iter)?),
            "acsc" | "arccsc" => Expression::func(|x| Ok((1.0 / x).asin()), parse_arg(iter)?),
            "acot" | "arccot" => Expression::func(|x| Ok((1.0 / x).atan()), parse_arg(iter)?),
            "loge" | "ln" => Expression::func(|x| Ok(x.ln()), parse_arg(iter)?),
            "log10" | "log" => Expression::func(|x| Ok(x.log10()), parse_arg(iter)?),
            "log2" | "lb" => Expression::func(|x| Ok(x.log2()), parse_arg(iter)?),
            "sqrt" => Expression::func(|x| Ok(x.sqrt()), parse_arg(iter)?),
            "cbrt" => Expression::func(|x| Ok(x.cbrt()), parse_arg(iter)?),
            "abs" => Expression::func(|x| Ok(x.abs()), parse_arg(iter)?),
            "e" => Expression::Num(core::f64::consts::E),
            "pi" => Expression::Num(core::f64::consts::PI),
            "tau" => Expression::Num(core::f64::consts::TAU),
            _ => Err(Error::Unrecognized)?,
        },
        Some(Lexeme::Group(Group { inner })) => parse_bp(&mut inner.iter().peekable(), 0)?,
        Some(Lexeme::Token(Token {
            ty: TokenType::Sym,
            text,
        })) if text == "+" => Expression::UnOp {
            op: UnOp::Pos,
            inner: Box::new(parse_bp(iter, 7)?),
        },
        Some(Lexeme::Token(Token {
            ty: TokenType::Sym,
            text,
        })) if text == "-" => Expression::UnOp {
            op: UnOp::Neg,
            inner: Box::new(parse_bp(iter, 7)?),
        },
        Some(Lexeme::Token(Token {
            ty: TokenType::Sym,
            text,
        })) if ["*", "/", "^"].contains(&&**text) => Err(Error::Invalid)?,
        _ => Err(Error::Unrecognized)?,
    })
}

fn parse_bp(iter: &mut Peekable<impl Iterator<Item = &Lexeme>>, min_bp: u8) -> Result<Expression> {
    let mut lhs = parse_atom(iter)?;

    loop {
        match iter.peek() {
            None => break,
            Some(Lexeme::Token(Token {
                ty: TokenType::Sym,
                text,
            })) => {
                let op = text;
                let (l_bp, r_bp) = bin_bp(op);
                if l_bp < min_bp {
                    break;
                }
                iter.next();
                let rhs = parse_bp(iter, r_bp)?;
                lhs = Expression::BinOp {
                    lhs: Box::new(lhs),
                    op: match &**op {
                        "+" => BinOp::Add,
                        "-" => BinOp::Sub,
                        "*" => BinOp::Mul,
                        "/" => BinOp::Div,
                        "^" | "**" => BinOp::Pow,
                        _ => unreachable!(),
                    },
                    rhs: Box::new(rhs),
                }
            }
            _ => {
                lhs = Expression::BinOp {
                    lhs: Box::new(lhs),
                    op: BinOp::Mul,
                    rhs: Box::new(parse_arg(iter)?),
                };
            }
        }
    }

    Ok(lhs)
}

fn parse(text: &str) -> Result<Expression> {
    let lexed = lex(&mut text.chars().peekable(), '\0')?;
    parse_bp(&mut lexed.iter().peekable(), 0)
}

/// Evaluate the input expression
///
/// # Errors
/// Returns an error upon receiving either an invalid expression or encountering an unknown operator
pub fn evaluate(text: &str) -> Result<f64> {
    parse(text)?.eval()
}
