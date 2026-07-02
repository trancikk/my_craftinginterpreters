use crate::rlox::interpreter::RLox;
use crate::rlox::parser::Expr::{Binary, Grouping, Unary};
use crate::rlox::scanner::Scanner;
use crate::rlox::token::TokenType::{
    Bang, BangEqual, Equal, False, FloatNumber, Greater, GreaterEqual, LeftParen, Less, LessEqual,
    Minus, Nil, Number, Plus, RightParen, Slash, Star, StringToken, True,
};
use crate::rlox::token::{Token, TokenType};

#[derive(Debug)]
enum Literal {
    Number(u64),
    FloatNumber(f64),
    StringL(String),
    True,
    False,
    Nil,
}

#[derive(Debug)]
enum Expr {
    Literal(Literal),
    Binary {
        left: Box<Expr>,
        op: Token,
        right: Box<Expr>,
    },
    Unary {
        op: Token,
        right: Box<Expr>,
    },
    Grouping {
        expr: Box<Expr>,
    },
    Operator {
        token: Token,
    },
}

struct Parser<'src, 'lox> {
    scanner: Scanner<'src>,
    current_token: Option<Token>,
    interpreter: &'lox mut RLox,
}

impl<'src, 'lox> Parser<'src, 'lox> {
    fn new(scanner: Scanner<'src>, interpreter: &'lox mut RLox) -> Parser<'src, 'lox> {
        Parser {
            scanner,
            current_token: None,
            interpreter,
        }
    }

    fn expression(&mut self) -> Option<Expr> {
        self.equality()
    }

    fn equality(&mut self) -> Option<Expr> {
        let mut exp = self.comparison()?;

        while let Some(item) = self.match_any_and_return(&[BangEqual, Equal]) {
            let right = self.comparison()?;
            exp = Binary {
                left: Box::new(exp),
                op: item,
                right: Box::new(right),
            }
        }
        Some(exp)
    }

    fn comparison(&mut self) -> Option<Expr> {
        let mut expr = self.term()?;

        while let Some(token) = self.match_any_and_return(&[Greater, GreaterEqual, Less, LessEqual])
        {
            let right = self.term()?;
            expr = Binary {
                left: Box::new(expr),
                op: token,
                right: Box::new(right),
            }
        }
        Some(expr)
    }

    fn term(&mut self) -> Option<Expr> {
        let mut expr = self.factor()?;

        while let Some(token) = self.match_any_and_return(&[Minus, Plus]) {
            let right = self.factor()?;
            expr = Binary {
                left: Box::new(expr),
                op: token,
                right: Box::new(right),
            }
        }
        Some(expr)
    }

    fn factor(&mut self) -> Option<Expr> {
        let mut expr = self.unary()?;

        while let Some(token) = self.match_any_and_return(&[Slash, Star]) {
            let right = self.unary()?;
            expr = Binary {
                left: Box::new(expr),
                op: token,
                right: Box::new(right),
            }
        }
        Some(expr)
    }

    fn unary(&mut self) -> Option<Expr> {
        if let Some(token) = self.match_any_and_return(&[Bang, Minus]) {
            let right = self.unary()?;
            return Some(Unary {
                op: token,
                right: Box::new(right),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Option<Expr> {
        let next = self.advance()?;
        let line = next.line;
        match next.token_type.clone() {
            False => Some(Expr::Literal(Literal::False)),
            True => Some(Expr::Literal(Literal::True)),
            Nil => Some(Expr::Literal(Literal::Nil)),
            Number(val) => Some(Expr::Literal(Literal::Number(val))),
            FloatNumber(val) => Some(Expr::Literal(Literal::FloatNumber(val))),
            StringToken(val) => Some(Expr::Literal(Literal::StringL(val.clone()))),
            LeftParen => {
                let expr = self.expression()?;
                self.consume(&RightParen, "Expect ')' after expression.");
                Some(Grouping {
                    expr: Box::new(expr),
                })
            }
            _ => {
                self.interpreter.report(line, "", "Syntax error!");
                None
            }
        }
    }

    fn consume(&mut self, token_type: &TokenType, message: &str) -> Option<Token> {
        let next = self.peek()?;
        let line = next.line;
        if next.token_type == *token_type {
            self.advance()
        } else {
            self.interpreter.error(line, message);
            None
        }
    }
    fn peek(&mut self) -> Option<&Token> {
        if self.current_token.is_none() {
            self.current_token = self.scanner.scan_token(self.interpreter);
        }
        self.current_token.as_ref()
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.current_token.take();
        if token.is_none() {
            self.scanner.scan_token(self.interpreter)
        } else {
            token
        }
    }

    fn check(&mut self, token_type: &TokenType) -> bool {
        if let Some(next) = self.peek() {
            next.token_type == *token_type
        } else {
            false
        }
    }

    fn match_any(&mut self, token_types: &[TokenType]) -> bool {
        if let Some(next) = self.peek() {
            token_types.contains(&next.token_type)
        } else {
            false
        }
    }

    fn match_any_and_return(&mut self, token_types: &[TokenType]) -> Option<Token> {
        if let Some(next) = self.peek()
            && token_types.contains(&next.token_type)
        {
            return self.advance();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rlox::*;

    #[test]
    fn it_works() {
        let mut interpreter = RLox { had_error: false };
        let prompt = "(1+2)
        ";
        let scanner = Scanner::new(prompt.lines());
        let mut parser = Parser::new(scanner, &mut interpreter);
        let result = parser.equality();
        println!("result: {result:?}");
    }
}
