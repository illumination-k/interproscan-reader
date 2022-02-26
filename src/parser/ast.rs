use super::lex::{lex, Token};
use std::{collections::VecDeque, error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Invert(Box<Node>),
    And { lhs: Box<Node>, rhs: Box<Node> },
    Or { lhs: Box<Node>, rhs: Box<Node> },
    Name(String),
}

#[derive(Debug)]
pub struct ParseError {
    error: String,
}

impl ParseError {
    pub fn new<S: ToString>(error: S) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ParseError: {}", self.error)
    }
}

impl Error for ParseError {}

impl Node {
    fn munch_tokens(tokens: &mut VecDeque<Token>, depth: u16) -> Result<Self, Box<dyn Error>> {
        if depth == 0 {
            let err: Box<dyn Error> = Box::new(ParseError::new("Expression too deep"));
            return Err(err);
        }

        loop {
            let next = match tokens.front() {
                Some(x) => x,
                None => {
                    let err: Box<dyn Error> =
                        Box::new(ParseError::new("unexpected end of expression"));
                    return Err(err);
                }
            };

            match next {
                Token::CloseBracket => {
                    let err: Box<dyn Error> =
                        Box::new(ParseError::new("Unexpected closing bracket"));
                    return Err(err);
                }
                Token::OpenBracket => {
                    let _ = tokens.pop_front();
                    let result = Self::munch_tokens(tokens, depth - 1)?;

                    if let Some(tk) = tokens.pop_front() {
                        if tk != Token::CloseBracket {
                            let err: Box<dyn Error> =
                                Box::new(ParseError::new("expected closing bracket"));
                            return Err(err);
                        }
                    }

                    return match tokens.front() {
                        Some(Token::And) => {
                            tokens.pop_front();
                            let result = Node::And {
                                lhs: Box::new(result),
                                rhs: Box::new(Self::munch_tokens(tokens, depth - 1)?),
                            };
                            return Ok(result);
                        }
                        Some(Token::Or) => {
                            let _ = tokens.pop_front();
                            let result = Node::Or {
                                lhs: Box::new(result),
                                rhs: Box::new(Self::munch_tokens(tokens, depth - 1)?),
                            };
                            return Ok(result);
                        }
                        None | Some(Token::CloseBracket) => Ok(result),
                        Some(_) => {
                            let err: Box<dyn Error> =
                                Box::new(ParseError::new("invald token after closing bracket"));
                            return Err(err);
                        }
                    };
                }
                Token::Invert => {
                    let _ = tokens.pop_front();

                    match tokens.front() {
                        Some(Token::OpenBracket) => {
                            return Ok(Node::Invert(Box::new(Self::munch_tokens(
                                tokens,
                                depth - 1,
                            )?)))
                        }
                        Some(Token::Name(text)) => {
                            let inverted = Node::Invert(Box::new(Node::Name(text.clone())));
                            match tokens.get(1) {
                                Some(Token::And) | Some(Token::Or) => {
                                    // "!abc & xyz"
                                    // convert to unambiguous form and try again
                                    tokens.insert(0, Token::OpenBracket);
                                    tokens.insert(1, Token::Invert);
                                    tokens.insert(2, Token::OpenBracket);
                                    tokens.insert(4, Token::CloseBracket);
                                    tokens.insert(5, Token::CloseBracket);
                                    return Self::munch_tokens(tokens, depth - 1);
                                }
                                None | Some(Token::CloseBracket) => {
                                    // "!abc"
                                    tokens.remove(0); // remove name
                                    return Ok(inverted);
                                }
                                Some(_) => {
                                    return Err(Box::new(ParseError::new(
                                        "invalid token after inverted name",
                                    )))
                                }
                            }
                        }
                        Some(Token::Invert) => {
                            return Err(Box::new(ParseError::new(
                                "Can't double invert, that would be no mean",
                            )));
                        }
                        Some(_) => return Err(Box::new(ParseError::new("expected expression"))),
                        None => {
                            return Err(Box::new(ParseError::new(
                                "Expected token to invert, got EOF",
                            )))
                        }
                    }
                }
                Token::Name(text) => match tokens.get(1) {
                    Some(Token::And) | Some(Token::Or) => {
                        add_bracket(tokens);
                        return Self::munch_tokens(tokens, depth - 1);
                    }
                    Some(Token::CloseBracket) | None => {
                        let text = text.clone();
                        let _ = tokens.pop_front();
                        return Ok(Node::Name(text));
                    }
                    Some(_) => {
                        let err = Box::new(ParseError::new("Name followed by invalid token"));
                        return Err(err);
                    }
                },
                Token::And | Token::Or => {
                    return Err(Box::new(ParseError::new("Unexpected binary operator")))
                }
            }
        }
    }

    fn matches(&self, tags: &[&str]) -> Result<bool, Box<dyn Error>> {
        let result = match self {
            Self::Invert(inverted) => !inverted.matches(tags)?,
            Self::Name(text) => {
                // counting numbers of elements
                let splitted: Vec<&str> = text.split("$").collect();
                match splitted.len() {
                    1 => tags.contains(&&**text),
                    2 => {
                        let count = splitted[1].parse::<usize>()?;
                        count == tags.iter().filter(|x| x == &&splitted[0]).count()
                    }
                    _ => return Err(Box::new(ParseError::new("unexpected text format"))),
                }
            }
            Self::And { lhs, rhs } => lhs.matches(tags)? && rhs.matches(tags)?,
            Self::Or { lhs, rhs } => lhs.matches(tags)? || rhs.matches(tags)?,
        };

        Ok(result)
    }
}

fn add_bracket(tokens: &mut VecDeque<Token>) {
    let elem = tokens.pop_front().unwrap();
    tokens.push_front(Token::CloseBracket);
    tokens.push_front(elem);
    tokens.push_front(Token::OpenBracket);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExprData {
    Empty,
    HasNodes(Node),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr(ExprData);

pub const MAX_RECURSION: u16 = 20;

impl Expr {
    pub fn from_string(s: &str) -> Result<Self, Box<dyn Error>> {
        // lex and convert to a deque
        let mut tokens: VecDeque<Token> = VecDeque::from(lex(s)?);
        if tokens.is_empty() {
            // no tokens
            return Ok(Self(ExprData::Empty));
        }

        let ast = Node::munch_tokens(&mut tokens, MAX_RECURSION)?;
        if !tokens.is_empty() {
            return Err(Box::new(ParseError::new(
                "expected EOF, found extra tokens",
            )));
        }

        Ok(Self(ExprData::HasNodes(ast)))
    }

    pub fn matches(&self, tags: &[&str]) -> Result<bool, Box<dyn Error>> {
        match &self.0 {
            ExprData::Empty => Ok(true),
            ExprData::HasNodes(node) => node.matches(tags),
        }
    }
}

#[cfg(test)]
mod test_parser {
    use super::*;

    #[test]
    fn test_addbracket() {
        let mut vq3: VecDeque<Token> = VecDeque::from(vec![
            Token::Name("a".to_string()),
            Token::And,
            Token::Name("b".to_string()),
        ]);

        add_bracket(&mut vq3);

        let excpected = VecDeque::from(vec![
            Token::OpenBracket,
            Token::Name("a".to_string()),
            Token::CloseBracket,
            Token::And,
            Token::Name("b".to_string()),
        ]);

        assert_eq!(vq3, excpected);
    }

    #[test]
    fn or_alias() {
        assert_eq!(
            Expr::from_string("a | b").unwrap().0,
            Expr::from_string("a , b").unwrap().0,
        );
    }

    #[test]
    fn simple_and() {
        let expr = Expr::from_string("a & b").unwrap();
        assert_eq!(
            expr.to_owned().0,
            ExprData::HasNodes(Node::And {
                lhs: Box::new(Node::Name("a".to_string())),
                rhs: Box::new(Node::Name("b".to_string())),
            })
        );

        assert!(expr.matches(&["a", "b"]).unwrap());
        assert!(!expr.matches(&["a"]).unwrap());
        assert!(!expr.matches(&["c"]).unwrap());
    }

    #[test]
    fn test_simple_count_and() {
        let expr = Expr::from_string("a$2 & b").unwrap();
        assert!(expr.matches(&["a", "a", "b"]).unwrap());
    }

    #[test]
    fn simple_inversion() {
        let expr = Expr::from_string("!a & b").unwrap();
        assert_eq!(
            expr.to_owned().0,
            ExprData::HasNodes(Node::And {
                lhs: Box::new(Node::Invert(Box::new(Node::Name("a".to_string())))),
                rhs: Box::new(Node::Name("b".to_string())),
            })
        );

        assert!(expr.matches(&["b"]).unwrap());
        assert!(!expr.matches(&["a", "b"]).unwrap());
        assert!(!expr.matches(&["a"]).unwrap());
        assert!(!expr.matches(&["c"]).unwrap());
    }

    #[test]
    fn simple_and_matching() {
        assert!(Expr::from_string("a & b & c")
            .unwrap()
            .matches(&["a", "b", "c"])
            .unwrap());
        assert!(!Expr::from_string("a & b & c")
            .unwrap()
            .matches(&["a", "c"])
            .unwrap());
        assert!(!Expr::from_string("a & b & c")
            .unwrap()
            .matches(&["a", "b"])
            .unwrap());
        assert!(!Expr::from_string("a & b & c")
            .unwrap()
            .matches(&["c", "b"])
            .unwrap());
        assert!(Expr::from_string("a & b & c")
            .unwrap()
            .matches(&["a", "b", "c", "d"])
            .unwrap());
        assert!(!Expr::from_string("a & b & c")
            .unwrap()
            .matches(&["a", "c", "d"])
            .unwrap());
        assert!(!Expr::from_string("a & b & c")
            .unwrap()
            .matches(&["a", "b", "d"])
            .unwrap());
        assert!(!Expr::from_string("a & b & c")
            .unwrap()
            .matches(&["c", "b", "d"])
            .unwrap());
    }

    #[test]
    fn simple_or_matching() {
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["a", "b", "c"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["a", "c"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["a", "b"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["c", "b"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["c"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["b"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["a"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["a", "b", "c", "d"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["a", "c", "d"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["a", "b", "d"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["c", "b", "d"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["c", "d"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["b", "d"])
            .unwrap());
        assert!(Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["a", "d"])
            .unwrap());
        assert!(!Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["ddwf"])
            .unwrap());
        assert!(!Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["d"])
            .unwrap());
        assert!(!Expr::from_string("a | b | c")
            .unwrap()
            .matches(&["hdwf", "dtw"])
            .unwrap());
    }

    #[test]
    fn lone_name() {
        assert!(Expr::from_string("a").unwrap().matches(&["a"]).unwrap());
        assert!(Expr::from_string("a")
            .unwrap()
            .matches(&["a", "b"])
            .unwrap());
        assert!(!Expr::from_string("a").unwrap().matches(&["b"]).unwrap());
    }

    #[test]
    fn lone_inverted_name() {
        assert!(!Expr::from_string("!a").unwrap().matches(&["a"]).unwrap());
        assert!(!Expr::from_string("!a")
            .unwrap()
            .matches(&["a", "b"])
            .unwrap());
        assert!(Expr::from_string("!a").unwrap().matches(&["b"]).unwrap());
    }

    #[test]
    fn lone_inverted_bracketed_name() {
        assert!(!Expr::from_string("!(a)").unwrap().matches(&["a"]).unwrap());
        assert!(!Expr::from_string("!(a)")
            .unwrap()
            .matches(&["a", "b"])
            .unwrap());
        assert!(Expr::from_string("!(a)").unwrap().matches(&["b"]).unwrap());
    }

    #[test]
    fn check_expr() {
        let s = "a | b | c";
        let expr = Expr::from_string(s).unwrap();
        assert!(expr.matches(&["a"]).unwrap());
        let s = "(a | b | c) | (d & e & c)";
        let expr = Expr::from_string(s).unwrap();
        assert!(expr.matches(&["a"]).unwrap());
        assert!(expr.matches(&["d", "e", "c"]).unwrap());
        assert!(!expr.matches(&["d"]).unwrap());
    }
}
