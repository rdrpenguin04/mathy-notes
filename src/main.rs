#![feature(anonymous_lifetime_in_impl_trait)]
#![windows_subsystem = "windows"]

use eframe::egui::{self, Modifiers, Ui};
use std::{fmt, iter::Peekable};

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Notes",
        native_options,
        Box::new(|cc| Box::new(NotesApp::new(cc))),
    )
    .unwrap();
}

#[derive(Default)]
struct NotesApp {
    notes_text: String,
}

impl NotesApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.storage.map_or_else(Self::default, |storage| Self {
            notes_text: storage.get_string("notes_text").unwrap_or_default(),
        })
    }
}

#[derive(Debug)]
enum Error {
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

type Result<T> = std::result::Result<T, Error>;

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

#[derive(Debug)]
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

impl fmt::Debug for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fn(_) => write!(f, "fn()"),
            Self::Pos => write!(f, "+"),
            Self::Neg => write!(f, "-"),
        }
    }
}

#[derive(Debug)]
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
    match iter.next() {
        Some(Lexeme::Token(Token {
            ty: TokenType::Num,
            text,
        })) => Ok(Expression::Num(parse_num(text)?)),
        Some(Lexeme::Token(Token {
            ty: TokenType::Id,
            text,
        })) => match &**text {
            "sin" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.sin()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "cos" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.cos()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "tan" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.tan()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "sec" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(1.0 / x.cos()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "csc" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(1.0 / x.sin()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "cot" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(1.0 / x.tan()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "asin" | "arcsin" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.asin()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "acos" | "arccos" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.acos()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "atan" | "arctan" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.atan()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "asec" | "arcsec" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok((1.0 / x).acos()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "acsc" | "arccsc" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok((1.0 / x).asin()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "acot" | "arccot" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok((1.0 / x).atan()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "loge" | "ln" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.ln()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "log" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.log10()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "log2" | "lb" => Ok(Expression::UnOp {
                op: UnOp::Fn(Box::new(|x| Ok(x.log2()))),
                inner: Box::new(parse_arg(iter)?),
            }),
            "e" => Ok(Expression::Num(core::f64::consts::E)),
            "pi" => Ok(Expression::Num(core::f64::consts::PI)),
            "tau" => Ok(Expression::Num(core::f64::consts::TAU)),
            _ => Err(Error::Unrecognized),
        },
        Some(Lexeme::Group(Group { inner })) => parse_bp(&mut inner.iter().peekable(), 0),
        Some(Lexeme::Token(Token {
            ty: TokenType::Sym,
            text,
        })) if text == "+" => Ok(Expression::UnOp {
            op: UnOp::Pos,
            inner: Box::new(parse_bp(iter, 7)?),
        }),
        Some(Lexeme::Token(Token {
            ty: TokenType::Sym,
            text,
        })) if text == "-" => Ok(Expression::UnOp {
            op: UnOp::Neg,
            inner: Box::new(parse_bp(iter, 7)?),
        }),
        Some(Lexeme::Token(Token {
            ty: TokenType::Sym,
            text,
        })) if ["*", "/", "^"].contains(&&**text) => Err(Error::Invalid),
        _ => Err(Error::Unrecognized),
    }
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

fn evaluate(text: &str) -> Result<f64> {
    parse(text)?.eval()
}

impl eframe::App for NotesApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let eval = ui.input_mut(|x| {
                x.consume_key(Modifiers::CTRL, egui::Key::Enter)
                    || x.consume_key(Modifiers::SHIFT, egui::Key::Enter)
            });
            ui.add_sized(ui.available_size(), move |ui: &mut Ui| {
                let text_edit = egui::TextEdit::multiline(&mut self.notes_text);
                let mut output = text_edit.show(ui);
                if eval {
                    if let Some(cursor) = output.cursor_range {
                        let p_idx = cursor.primary.ccursor.index;
                        let s_idx = cursor.secondary.ccursor.index;
                        let start = if p_idx == s_idx {
                            self.notes_text[..p_idx]
                                .rfind(|x| x == '=' || x == '\n')
                                .map_or(0, |x| x + 1)
                        } else {
                            p_idx.min(s_idx)
                        };
                        let end = p_idx.max(s_idx);
                        let text = &self.notes_text[start..end];
                        let result = evaluate(text);
                        let insertion = format!(
                            " = {}",
                            match result {
                                Ok(x) => x.to_string(),
                                Err(x) => x.to_string(),
                            }
                        );
                        output.state.set_ccursor_range(Some(
                            egui::widgets::text_edit::CCursorRange {
                                primary: egui::text::CCursor {
                                    index: end + insertion.len(),
                                    prefer_next_row: true,
                                },
                                secondary: egui::text::CCursor {
                                    index: end + insertion.len(),
                                    prefer_next_row: true,
                                },
                            },
                        ));
                        output.state.store(ctx, output.response.id);
                        self.notes_text.insert_str(end, &insertion);
                    }
                }
                output.response
            });
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("notes_text", self.notes_text.clone());
        storage.flush();
    }
}
