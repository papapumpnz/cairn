use crate::{BinaryOp, Expr, UnaryOp, VarName};
use std::fmt;

/// Debug-fixture parser failure with byte offset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse the intentionally small S-expression fixture syntax.
///
/// Supported forms are literals, variables, unary `not`/`neg`, binary scalar
/// operations, `if`, `let`, `tuple`, and `count`. This parser is a debugging
/// aid, not the persistent representation.
///
/// # Errors
///
/// Returns the first deterministic lexical, arity, or syntax error.
pub fn parse_debug_expr(source: &str) -> Result<Expr, ParseError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser { tokens, index: 0 };
    let expression = parser.expression()?;
    if let Some(token) = parser.tokens.get(parser.index) {
        return Err(ParseError {
            offset: token.offset,
            message: "trailing token".to_owned(),
        });
    }
    Ok(expression)
}

#[derive(Clone, Debug)]
struct Token {
    offset: usize,
    text: String,
}

fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (offset, character) in source.char_indices() {
        if character == '(' || character == ')' || character.is_whitespace() {
            if let Some(begin) = start.take() {
                tokens.push(Token {
                    offset: begin,
                    text: source[begin..offset].to_owned(),
                });
            }
            if character == '(' || character == ')' {
                tokens.push(Token {
                    offset,
                    text: character.to_string(),
                });
            }
        } else if character.is_control() {
            return Err(ParseError {
                offset,
                message: "control character".to_owned(),
            });
        } else if start.is_none() {
            start = Some(offset);
        }
    }
    if let Some(begin) = start {
        tokens.push(Token {
            offset: begin,
            text: source[begin..].to_owned(),
        });
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn expression(&mut self) -> Result<Expr, ParseError> {
        let token = self.next()?.clone();
        if token.text != "(" {
            return atom(&token);
        }
        let operation = self.next()?.clone();
        let expression = match operation.text.as_str() {
            "not" => Expr::Unary {
                op: UnaryOp::Not,
                argument: Box::new(self.expression()?),
            },
            "neg" => Expr::Unary {
                op: UnaryOp::Negate,
                argument: Box::new(self.expression()?),
            },
            "if" => Expr::If {
                condition: Box::new(self.expression()?),
                then_branch: Box::new(self.expression()?),
                else_branch: Box::new(self.expression()?),
            },
            "let" => {
                let binder = self.next()?.clone();
                if matches!(binder.text.as_str(), "(" | ")") {
                    return Err(ParseError {
                        offset: binder.offset,
                        message: "expected binder name".to_owned(),
                    });
                }
                Expr::Let {
                    binder: VarName(binder.text),
                    value: Box::new(self.expression()?),
                    body: Box::new(self.expression()?),
                }
            }
            "tuple" => {
                let mut values = Vec::new();
                while self.peek_text() != Some(")") {
                    values.push(self.expression()?);
                }
                Expr::Tuple(values)
            }
            "count" => Expr::Count(Box::new(self.expression()?)),
            other => {
                let op = binary_operator(other).ok_or_else(|| ParseError {
                    offset: operation.offset,
                    message: format!("unknown operation `{other}`"),
                })?;
                Expr::Binary {
                    op,
                    left: Box::new(self.expression()?),
                    right: Box::new(self.expression()?),
                }
            }
        };
        let close = self.next()?;
        if close.text != ")" {
            return Err(ParseError {
                offset: close.offset,
                message: "expected closing parenthesis".to_owned(),
            });
        }
        Ok(expression)
    }

    fn next(&mut self) -> Result<&Token, ParseError> {
        let token = self.tokens.get(self.index).ok_or_else(|| ParseError {
            offset: self
                .tokens
                .last()
                .map_or(0, |token| token.offset + token.text.len()),
            message: "unexpected end of input".to_owned(),
        })?;
        self.index += 1;
        Ok(token)
    }

    fn peek_text(&self) -> Option<&str> {
        self.tokens.get(self.index).map(|token| token.text.as_str())
    }
}

fn atom(token: &Token) -> Result<Expr, ParseError> {
    match token.text.as_str() {
        "true" => Ok(Expr::Bool(true)),
        "false" => Ok(Expr::Bool(false)),
        value => value.parse::<i64>().map_or_else(
            |_| {
                if valid_name(value) {
                    Ok(Expr::Var(VarName(value.to_owned())))
                } else {
                    Err(ParseError {
                        offset: token.offset,
                        message: "invalid atom".to_owned(),
                    })
                }
            },
            |value| Ok(Expr::Int(value)),
        ),
    }
}

fn valid_name(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn binary_operator(value: &str) -> Option<BinaryOp> {
    match value {
        "add" => Some(BinaryOp::Add),
        "sub" => Some(BinaryOp::Subtract),
        "mul" => Some(BinaryOp::Multiply),
        "min" => Some(BinaryOp::Minimum),
        "max" => Some(BinaryOp::Maximum),
        "and" => Some(BinaryOp::And),
        "or" => Some(BinaryOp::Or),
        "eq" => Some(BinaryOp::Equal),
        "lt" => Some(BinaryOp::LessThan),
        "union" => Some(BinaryOp::Union),
        "intersection" => Some(BinaryOp::Intersection),
        "difference" => Some(BinaryOp::Difference),
        _ => None,
    }
}
