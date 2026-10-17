use crate::rlox::eval::Value;
use crate::rlox::interpreter::RloxError;
use crate::rlox::interpreter::RloxError::ParserError;
use crate::rlox::parser::Expr::{
    AssignmentExpression, Binary, Grouping, MissingLeftOperand, Ternary, Unary,
};
use crate::rlox::parser::Statement::{
    BlockStatement, DeclarationStatement, IfStatement, ReturnStatement, WhileStatement,
};
use crate::rlox::scanner::Scanner;
use crate::rlox::token::TokenType::{
    And, Bang, BangEqual, Break, Colon, Comma, EOF, Equal, EqualEqual, False, FloatNumber, For,
    Fun, Greater, GreaterEqual, Identifier, If, LeftBrace, LeftParen, Less, LessEqual, Minus, Nil,
    Number, Or, Plus, Print, Question, Return, RightBrace, RightParen, Semicolon, Slash, Star,
    StringToken, True, Var, While,
};
use crate::rlox::token::{Token, TokenType};
use crate::rlox::var_resolver::Resolution;
use crate::rlox::var_resolver::Resolution::Unresolved;

const MAX_ARGS: usize = 255;

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Number(u64),
    FloatNumber(f64),
    StringL(String),
    True,
    False,
    Nil,
}

#[derive(Debug)]
pub enum ExecFlow {
    Normal(Option<Value>),
    Break(Token),
    Return(Option<Value>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IdentifierExpr {
    pub(crate) token: Token,
    pub(crate) hops: Resolution,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    Print(Expr),
    ExprStatement(Expr),
    DeclarationStatement(Token, Option<Expr>),
    BlockStatement(Block),
    IfStatement {
        expr: Expr,
        then: Box<Statement>,
        else_stmnt: Option<Box<Statement>>,
    },
    WhileStatement {
        expr: Expr,
        body: Box<Statement>,
    },
    BreakStatement(Token),
    ReturnStatement(Token, Option<Expr>),
    FunctionStatement {
        args: Vec<Token>,
        name: IdentifierExpr,
        body: Block,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    SyntaxLiteral(Literal),
    IdentifierExpression(IdentifierExpr),
    Binary {
        left: Box<Expr>,
        op: Token,
        right: Box<Expr>,
    },
    Ternary {
        predicate: Box<Expr>,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: Token,
        right: Box<Expr>,
    },
    Grouping {
        expr: Box<Expr>,
    },
    AssignmentExpression {
        left: Box<IdentifierExpr>,
        right: Box<Expr>,
    },
    MissingLeftOperand {
        token: Token,
    },
    CallableExpression {
        callee: Box<Expr>,
        paren: Token,
        args: Vec<Expr>,
    },
    FunctionExpression {
        args: Vec<Token>,
        body: Block,
    },
}

#[derive(PartialEq, Copy, Clone)]
pub enum ParserMode {
    Script,
    Repl,
}

pub struct Parser<'src> {
    scanner: Scanner<'src>,
    current_token: Option<Token>,
    next_token: Option<Token>,
    parser_mode: ParserMode,
    loop_nesting: u16,
    fun_nesting: u16,
}

impl<'src> Parser<'src> {
    pub fn new(scanner: Scanner<'src>) -> Parser<'src> {
        Self::new_with_mode(scanner, ParserMode::Script)
    }

    pub fn new_with_mode(scanner: Scanner<'src>, parser_mode: ParserMode) -> Parser<'src> {
        Parser {
            scanner,
            current_token: None,
            next_token: None,
            parser_mode,
            loop_nesting: 0,
            fun_nesting: 0,
        }
    }

    // pub fn new_repl(scanner: Scanner<'src>) -> Parser<'src> {
    //     Self::new_with_mode(scanner, ParserMode::Repl)
    // }

    // pub fn parse(&mut self) {
    //     while self.expression().is_ok() {}
    // }

    pub fn parse_statements(&mut self) -> Result<Statement, RloxError> {
        let next_token = self.peek()?;
        {
            match next_token.token_type {
                If => self.parse_if(),
                While => self.parse_while(),
                For => self.parse_for(),
                Fun => self.parse_fun_declaration(),
                LeftBrace => self.parse_block_statement(),
                Var => self.parse_declarations(),
                Print => self.print_statement(),
                Return => self.return_statement(),
                Break => {
                    let next_token = self.advance()?;
                    self.consume_statement_end()?;
                    if self.loop_nesting == 0 {
                        eprintln!("Break outside of a loop!");
                        Err(ParserError(
                            next_token,
                            "Break outside of a loop!".to_string(),
                        ))
                    } else {
                        Ok(Statement::BreakStatement(next_token))
                    }
                }
                _ => self.parse_expr_statement(),
            }
        }
        .inspect_err(|_| {
            let _ = self.synchronize();
        })
    }

    fn return_statement(&mut self) -> Result<Statement, RloxError> {
        let token = self.consume(&Return, "Expect Return keyword for return")?;
        let expr = if !self.check(&Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        let statement_end = self.consume_statement_end()?;
        if self.fun_nesting == 0 {
            Err(ParserError(
                statement_end,
                String::from("Return at top level is not allowed"),
            ))
        } else {
            Ok(ReturnStatement(token, expr))
        }
    }

    fn parse_fun_declaration(&mut self) -> Result<Statement, RloxError> {
        self.consume(&Fun, "Expected fun keyword")?;
        let name = self.match_and_return(&Identifier);
        if let Ok(name) = name {
            self.fun_nesting += 1;
            self.consume(&LeftParen, "Expect '(' after function name")?;
            let params = self.parse_fun_args()?;
            let body = self.parse_block()?;
            self.fun_nesting -= 1;
            Ok(Statement::FunctionStatement {
                args: params,
                name: IdentifierExpr {
                    token: name,
                    hops: Unresolved,
                },
                body,
            })
        } else {
            let fun_expr = self.parse_fun_expression()?;
            self.expr_to_statement(fun_expr)
        }
    }
    fn parse_for(&mut self) -> Result<Statement, RloxError> {
        self.loop_nesting += 1;
        let result = {
            self.advance()?;
            self.consume(&LeftParen, "Expected ( after for")?;
            let init = {
                let next_token = self.peek()?;
                match next_token.token_type {
                    Semicolon => {
                        self.advance()?;
                        None
                    }
                    Var => Some(self.parse_declarations()?),
                    _ => Some(self.parse_expr_statement()?),
                }
            };
            // self.consume(&Semicolon, "Expected ; after for init")?;
            let expr = {
                if !self.check(&Semicolon) {
                    self.parse_expression()?
                } else {
                    Expr::SyntaxLiteral(Literal::True)
                }
            };
            self.consume(&Semicolon, "Expected ; after for expression")?;
            let increment = {
                if !self.check(&RightParen) {
                    Some(self.parse_expression()?)
                } else {
                    None
                }
            };
            self.consume(&RightParen, "Expected ) after for for loop")?;
            let statement = self.parse_statements()?;

            let mut while_block = vec![statement];
            if let Some(increment) = increment {
                let incr_stmnt = Statement::ExprStatement(increment);
                while_block.push(incr_stmnt);
            };
            let while_block = BlockStatement(Block {
                statements: while_block,
            });
            let while_loop = WhileStatement {
                expr,
                body: Box::new(while_block),
            };
            let final_block = if let Some(init) = init {
                BlockStatement(Block {
                    statements: vec![init, while_loop],
                })
            } else {
                while_loop
            };
            Ok(final_block)
        };
        self.loop_nesting -= 1;
        result
    }

    fn parse_if(&mut self) -> Result<Statement, RloxError> {
        self.advance()?;
        self.consume(&LeftParen, "Expected ( after if")?;
        let expr = self.parse_expression()?;
        self.consume(&RightParen, "Expected ) after if expression")?;
        let then = self.parse_statements()?;
        let else_stmnt = if self.check(&TokenType::Else) {
            self.advance()?;
            Some(Box::new(self.parse_statements()?))
        } else {
            None
        };
        Ok(IfStatement {
            expr,
            then: Box::new(then),
            else_stmnt,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, RloxError> {
        self.loop_nesting += 1;
        let result = {
            self.advance()?;
            self.consume(&LeftParen, "Expected ( after while")?;
            let expr = self.parse_expression()?;
            self.consume(&RightParen, "Expected ) after while expression")?;
            let body = self.parse_statements()?;
            Ok(WhileStatement {
                expr,
                body: Box::new(body),
            })
        };
        self.loop_nesting -= 1;
        result
    }

    fn parse_block_statement(&mut self) -> Result<Statement, RloxError> {
        self.parse_block().map(BlockStatement)
    }

    fn parse_block(&mut self) -> Result<Block, RloxError> {
        self.consume(&LeftBrace, "Expected '{' before block")?;
        let mut res: Vec<Statement> = Vec::new();
        let token_match = [RightBrace, EOF];
        while !self.match_any(&token_match) {
            let next_stmtn = self.parse_statements()?;
            res.push(next_stmtn);
        }
        self.consume(&RightBrace, "Expected } at the end of a block!")?;
        Ok(Block { statements: res })
    }

    fn parse_declarations(&mut self) -> Result<Statement, RloxError> {
        self.advance()?;
        let name = self.match_and_return(&Identifier)?;
        let initializer = if self.check(&Equal) {
            self.advance()?;
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.consume_statement_end()?;
        Ok(DeclarationStatement(name, initializer))
    }

    fn print_statement(&mut self) -> Result<Statement, RloxError> {
        self.advance()?;
        let exp = self.parse_expression()?;
        self.consume_statement_end()?;
        Ok(Statement::Print(exp))
    }

    fn parse_expr_statement(&mut self) -> Result<Statement, RloxError> {
        let expr = self.parse_expression()?;
        self.expr_to_statement(expr)
    }

    fn expr_to_statement(&mut self, expr: Expr) -> Result<Statement, RloxError> {
        self.consume_statement_end()?;
        Ok(Statement::ExprStatement(expr))
    }

    // fn report_binary_operation_error(token: Token) -> RloxError {
    //     let msg = format!("Binary operation {token:?} is missing left argument!");
    //     ParserError(token, msg)
    // }

    fn produce_error_for_expression(token: Token, msg: &str) -> Result<Expr, RloxError> {
        Err(ParserError(token, String::from(msg)))
    }

    fn produce_error_for_token(token: Token, msg: &str) -> Result<Token, RloxError> {
        Err(ParserError(token, String::from(msg)))
    }

    fn produce_and_report_binary_error(expr: Expr, token: Token) -> Result<Expr, RloxError> {
        Ok(Binary {
            left: Box::new(MissingLeftOperand {
                token: token.clone(),
            }),
            op: token,
            right: Box::new(expr),
        })
    }

    // fn produce_and_report_binary_error(expr: Expr, token: Token) -> Result<Expr, RloxError> {
    //     drop(expr);
    //     // self.produce_binary_error(expr, token);
    //     Err(Self::report_binary_operation_error(token))
    // }

    fn loop_inside_same_level(
        &mut self,
        ops: &[TokenType],
        next_level: fn(&mut Self) -> Result<Expr, RloxError>,
    ) -> Result<Expr, RloxError> {
        let mut expr = next_level(self)?;
        loop {
            let token = self.peek()?;
            if token.token_type == EOF {
                break;
            } else if ops.contains(&token.token_type) {
                let token = self.advance()?;
                let right = next_level(self)?;
                expr = Binary {
                    left: Box::new(expr),
                    op: token,
                    right: Box::new(right),
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    pub fn parse_expression(&mut self) -> Result<Expr, RloxError> {
        if let Ok(token) = self.match_and_return(&Comma) {
            let expr = self.parse_expression()?;
            return Self::produce_and_report_binary_error(expr, token);
        }
        self.loop_inside_same_level(&[Comma], Self::parse_assignment)
    }

    fn parse_assignment(&mut self) -> Result<Expr, RloxError> {
        let l_value = self.ternary()?;

        if self.check(&Equal) {
            match l_value {
                Expr::IdentifierExpression(ident_expr) => {
                    self.advance()?;
                    let r_value = self.parse_assignment()?;
                    Ok(AssignmentExpression {
                        left: Box::new(ident_expr),
                        right: Box::new(r_value),
                    })
                }
                _ => Self::produce_error_for_expression(
                    self.current_token
                        .clone()
                        .expect("Token should be present at this stage"),
                    "Invalid assignment target.",
                ),
            }
        } else {
            Ok(l_value)
        }
    }

    fn ternary(&mut self) -> Result<Expr, RloxError> {
        let mut expr = self.logical_or()?;
        if self.match_and_return(&Question).is_ok() {
            let left = self.parse_expression()?;
            self.consume(
                &Colon,
                "Expected ':' in a ternary operator after the predicate",
            )?;
            let right = self.parse_assignment()?;
            expr = Ternary {
                predicate: Box::new(expr),
                left: Box::new(left),
                right: Box::new(right),
            }
        }
        Ok(expr)
    }

    fn logical_or(&mut self) -> Result<Expr, RloxError> {
        if let Ok(token) = self.match_and_return(&Or) {
            let expr = self.logical_or()?;
            return Self::produce_and_report_binary_error(expr, token);
        }
        self.loop_inside_same_level(&[Or], Self::logical_and)
    }

    fn logical_and(&mut self) -> Result<Expr, RloxError> {
        if let Ok(token) = self.match_and_return(&And) {
            let expr = self.logical_and()?;
            return Self::produce_and_report_binary_error(expr, token);
        }
        self.loop_inside_same_level(&[And], Self::equality)
    }

    fn equality(&mut self) -> Result<Expr, RloxError> {
        let ops = [BangEqual, EqualEqual];
        if let Ok(Some(token)) = self.match_any_and_return(&ops) {
            let expr = self.equality()?;
            return Self::produce_and_report_binary_error(expr, token);
        }
        self.loop_inside_same_level(&ops, Self::comparison)
    }

    fn comparison(&mut self) -> Result<Expr, RloxError> {
        let ops = [Greater, GreaterEqual, Less, LessEqual];
        if let Ok(Some(token)) = self.match_any_and_return(&ops) {
            let expr = self.comparison()?;
            return Self::produce_and_report_binary_error(expr, token);
        }

        self.loop_inside_same_level(&ops, Self::term)
    }

    fn term(&mut self) -> Result<Expr, RloxError> {
        if let Ok(token) = self.match_and_return(&Plus) {
            let expr = self.term()?;
            return Self::produce_and_report_binary_error(expr, token);
        }

        self.loop_inside_same_level(&[Minus, Plus], Self::factor)
    }

    fn factor(&mut self) -> Result<Expr, RloxError> {
        let ops = [Slash, Star];

        if let Ok(Some(token)) = self.match_any_and_return(&ops) {
            let expr = self.factor()?;
            return Self::produce_and_report_binary_error(expr, token);
        }

        self.loop_inside_same_level(&ops, Self::unary)
    }

    fn unary(&mut self) -> Result<Expr, RloxError> {
        if let Ok(Some(token)) = self.match_any_and_return(&[Bang, Minus]) {
            let right = self.unary()?;
            return Ok(Unary {
                op: token,
                right: Box::new(right),
            });
        }
        self.callable()
    }

    fn callable(&mut self) -> Result<Expr, RloxError> {
        let mut expr = self.primary()?;
        // expr = self.parse_callable(expr)?;
        if self.check_and_advance(&LeftParen) {
            // loop {
            expr = self.parse_callable(expr)?;
            // if !self.check_and_advance(&Comma) {
            //     break;
            // }
            // }
        }

        Ok(expr)
    }

    fn parse_callable(&mut self, expr: Expr) -> Result<Expr, RloxError> {
        let mut args = vec![];
        loop {
            if let Ok(paren) = self.match_and_return(&RightParen) {
                return Ok(Expr::CallableExpression {
                    callee: Box::new(expr),
                    args,
                    paren,
                });
            }
            let ass_expr = self.parse_assignment()?;
            // println!("Pushing {:?}", ass_expr);
            args.push(ass_expr);
            if args.len() > MAX_ARGS {
                let next_token = self.peek()?;
                return Err(ParserError(
                    next_token.clone(),
                    "Can't have more than 255 arguments.".to_string(),
                ));
            } else {
                if self.check_and_advance(&Comma)
                    && let Ok(Some(token)) = self.match_any_and_return(&[RightParen, EOF])
                {
                    return Err(ParserError(
                        token,
                        "Extra trailing coma after function arguments!".to_string(),
                    ));
                }
            }
        }
    }

    fn parse_fun_expression(&mut self) -> Result<Expr, RloxError> {
        self.fun_nesting += 1;
        self.should_not_match(&Identifier)?;
        self.consume(&LeftParen, "Expect '(' after function name")?;
        let params = self.parse_fun_args()?;
        let body = self.parse_block()?;
        self.fun_nesting -= 1;
        Ok(Expr::FunctionExpression { args: params, body })
    }

    fn primary(&mut self) -> Result<Expr, RloxError> {
        let next = self.advance()?;
        match next.token_type.clone() {
            Identifier => Ok(Expr::IdentifierExpression(IdentifierExpr {
                token: next,
                hops: Unresolved,
            })),
            False => Ok(Expr::SyntaxLiteral(Literal::False)),
            True => Ok(Expr::SyntaxLiteral(Literal::True)),
            Nil => Ok(Expr::SyntaxLiteral(Literal::Nil)),
            Fun => self.parse_fun_expression(),
            Number(val) => Ok(Expr::SyntaxLiteral(Literal::Number(val))),
            FloatNumber(val) => Ok(Expr::SyntaxLiteral(Literal::FloatNumber(val))),
            StringToken(val) => Ok(Expr::SyntaxLiteral(Literal::StringL(val.clone()))),
            LeftParen => {
                let expr = self.parse_expression()?;
                self.consume(&RightParen, "Expect ')' after expression.")?;
                Ok(Grouping {
                    expr: Box::new(expr),
                })
            }
            EOF => Err(RloxError::EOF),
            _ => {
                Self::produce_error_for_expression(next, "Syntax error! Expected expression")
                // self.interpreter
                //     .report_token(&next, );
                // None
            }
        }
    }

    fn parse_fun_args(&mut self) -> Result<Vec<Token>, RloxError> {
        let mut params = vec![];
        if !self.check_and_advance(&RightParen) {
            loop {
                let param = self.consume(&Identifier, "Expected parameter name")?;
                if params.len() >= MAX_ARGS {
                    let next_token = self.peek()?;
                    return Err(ParserError(
                        next_token.clone(),
                        "Can't have more than 255 arguments.".to_string(),
                    ));
                }
                params.push(param);
                if self.check_and_advance(&Comma) {
                    if let Ok(Some(token)) = self.match_any_and_return(&[RightParen, EOF]) {
                        return Err(ParserError(
                            token,
                            "Extra trailing coma after function parameters!".to_string(),
                        ));
                    }
                } else {
                    break;
                }
            }
            self.consume(&RightParen, "Expect ')' after function args")?;
        }
        Ok(params)
    }

    fn consume(&mut self, token_type: &TokenType, message: &str) -> Result<Token, RloxError> {
        let next = self.peek().cloned()?;
        if next.token_type == *token_type {
            self.advance()
        } else {
            Self::produce_error_for_token(next, message)
        }
    }

    fn consume_statement_end(&mut self) -> Result<Token, RloxError> {
        let next = self.peek()?.clone();
        match next.token_type {
            Semicolon => self.advance(),
            EOF if self.parser_mode == ParserMode::Repl => Ok(next),
            _ => Err(ParserError(
                next,
                String::from("Expected semicolon ; or EOF as the end of the statement!"),
            )),
        }
    }

    fn peek(&mut self) -> Result<&Token, RloxError> {
        if self.next_token.is_none() {
            let next_token = self.scanner.scan_token()?;
            self.next_token = Some(next_token);
        }
        Ok(self.next_token.as_ref().unwrap())
    }

    // TODO maybe return just token or EOF
    /// Synchronize parser state after a parse error (panic mode recovery).
    ///
    /// This implements the synchronization strategy from the Lox book (Chapter 6):
    /// after an error, skip tokens until we hit a statement boundary — either
    /// a semicolon (hard boundary) or a keyword that starts a statement (soft boundary).
    ///
    /// CRITICAL BUG FIX: The original implementation had three issues:
    /// 1. **Unconditional `advance()` first** — The error token (e.g., `;`) was already
    ///    consumed by the calling parse function. Advancing again skipped the boundary.
    /// 2. **Checked wrong token for keywords** — After seeing a semicolon, it peeked at
    ///    the *next* token instead of checking the current position for statement keywords.
    /// 3. **Consumed statement keywords** — When breaking on a keyword, it had already
    ///    consumed it, so the next `parse_statements()` call would miss it entirely.
    ///
    /// CORRECT BEHAVIOR:
    /// - **Semicolon/EOF = hard boundary**: consume it and stop (statement ended)
    /// - **Statement keyword = soft boundary**: STOP but DON'T consume (next parse needs it)
    /// - **Other tokens**: skip (garbage from malformed statement)
    ///
    /// This ensures that after `print;` (missing expression), sync stops at `;`,
    /// and the next `parse_statements()` correctly sees the following `print 42;`.
    fn synchronize(&mut self) -> Result<Token, RloxError> {
        // Start from current position (peek), don't advance first.
        // The error token may have already been consumed by the parsing function
        // that called us (e.g., parse_declarations consumed ';', then called sync).
        loop {
            let token = self.peek()?;

            // Break on semicolon (statement boundary), statement keyword, or EOF
            match token.token_type {
                Semicolon | TokenType::EOF => {
                    // HARD BOUNDARY: Consume the semicolon/EOF and stop.
                    // The semicolon marks the end of the bad statement.
                    self.advance()?;
                    break;
                }
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => {
                    // SOFT BOUNDARY: Statement keyword found.
                    // IMPORTANT: Do NOT consume it. Leave it for the next
                    // parse_statements() call to handle as a new statement.
                    break;
                }
                _ => {
                    // Garbage token from malformed statement — skip it.
                    self.advance()?;
                }
            }
        }
        // Return the token at the sync point (semicolon, EOF, or statement keyword).
        self.peek().cloned()
    }
    fn advance(&mut self) -> Result<Token, RloxError> {
        let token = self.next_token.take();
        if let Some(token) = token {
            self.current_token = Some(token.clone());
            Ok(token)
        } else {
            let next_token = self.scanner.scan_token()?;
            self.current_token = Some(next_token.clone());
            Ok(next_token)
        }
    }

    fn check(&mut self, token_type: &TokenType) -> bool {
        if let Ok(next) = self.peek() {
            next.token_type == *token_type
        } else {
            false
        }
    }

    fn should_not_match(&mut self, token_type: &TokenType) -> Result<(), RloxError> {
        if let Ok(next) = self.peek()
            && next.token_type == *token_type
        {
            Err(ParserError(next.clone(), String::from("Unexpected token")))
        } else {
            Ok(())
        }
    }

    fn check_and_advance(&mut self, token_type: &TokenType) -> bool {
        if let Ok(next) = self.peek()
            && (next.token_type == *token_type)
        {
            self.advance().is_ok()
        } else {
            false
        }
    }

    fn expect(&mut self, token_type: &TokenType) -> Result<Token, RloxError> {
        let next_token = self.peek()?;
        if next_token.token_type != *token_type {
            Err(ParserError(
                next_token.clone(),
                format!("Expected {:?}, got {:?}", token_type, next_token),
            ))
        } else {
            self.advance()
        }
    }

    fn match_and_return(&mut self, token_type: &TokenType) -> Result<Token, RloxError> {
        let token = self.peek().cloned()?;
        if token.token_type == *token_type {
            return self.advance();
        }
        let msg = format!("Expected token: {token_type:?}, got: {token:?}");
        Err(ParserError(token, msg))
    }

    fn match_any(&mut self, token_types: &[TokenType]) -> bool {
        if let Ok(next) = self.peek() {
            token_types.contains(&next.token_type)
        } else {
            false
        }
    }

    fn match_any_and_return(
        &mut self,
        token_types: &[TokenType],
    ) -> Result<Option<Token>, RloxError> {
        let next = self.peek().cloned()?;
        if token_types.contains(&next.token_type) {
            return self.advance().map(|token| Some(token));
        }
        let msg = format!("Expected tokens: {token_types:?}, got: {next:?}");
        Err(ParserError(next, msg))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::rlox::var_resolver::Resolver;

    #[cfg(test)]
    pub(crate) fn parse_one_expression(input: &str) -> Expr {
        use crate::rlox::scanner::Scanner;

        let scanner = Scanner::new(input.lines());
        let mut parser = Parser::new(scanner);
        let result = parser.parse_expression();
        assert!(result.is_ok());
        result.unwrap_or_else(|_| panic!("expected expression for {input:?}"))
    }

    pub(crate) fn parse_expression(input: &str) -> Result<Expr, RloxError> {
        let scanner = Scanner::new(input.lines());
        let mut parser = Parser::new(scanner);
        let mut resolver = Resolver::new();
        let mut expr = parser.parse_expression();
        while let res = parser.parse_expression()
            && res.is_ok()
        {
            expr = res
        }
        if let Ok(exp) = &mut expr {
            resolver.resolve_expression(exp)?
        };
        expr
    }

    pub(crate) fn parse_statement(input: &str) -> Result<Statement, RloxError> {
        let scanner = Scanner::new(input.lines());
        let mut parser = Parser::new(scanner);
        let mut resolver = Resolver::new();
        let mut statements = parser.parse_statements();
        if let Ok(stmnts) = &mut statements {
            resolver.resolve_statement(stmnts)?
        };
        statements
    }

    pub(crate) fn parse_ok(input: &str) -> Expr {
        let result = parse_expression(input);
        println!("{:?}", result);
        result.unwrap_or_else(|_| panic!("expected expression for {input:?}"))
    }

    pub(crate) fn parse_statement_ok(input: &str) -> Statement {
        let result = parse_statement(input);
        result.unwrap_or_else(|_| panic!("expected expression for {input:?}"))
    }

    pub(crate) fn parse_expression_fails(input: &str) {
        let result = parse_expression(input);
        assert!(
            result.is_err(),
            "expected parse failure for {input:?}, got {result:?}"
        );
    }

    pub(crate) fn parse_statement_fails(input: &str) {
        let result = parse_statement(input);
        assert!(
            result.is_err(),
            "expected parse failure for {input:?}, got {result:?}"
        );
    }

    macro_rules! parsed {
        ($input:expr, |$e:ident| $body:expr) => {{
            let $e = parse_ok($input);
            $body
        }};
    }

    macro_rules! parsed_stmt {
        ($input:expr, |$s:ident| $body:expr) => {{
            let $s = parse_statement_ok($input);
            $body
        }};
    }

    pub(crate) fn as_binary(expr: &Expr) -> (&Expr, TokenType, &Expr) {
        match expr {
            Binary { left, op, right } => (left, op.token_type.clone(), right),
            other => panic!("expected binary expression, got {other:?}"),
        }
    }

    pub(crate) fn as_unary(expr: &Expr) -> (TokenType, &Expr) {
        match expr {
            Unary { op, right } => (op.token_type.clone(), right),
            other => panic!("expected unary expression, got {other:?}"),
        }
    }

    pub(crate) fn as_grouping(expr: &Expr) -> &Expr {
        match expr {
            Grouping { expr } => expr,
            other => panic!("expected grouping expression, got {other:?}"),
        }
    }

    pub(crate) fn as_number(expr: &Expr) -> u64 {
        match expr {
            Expr::SyntaxLiteral(Literal::Number(n)) => *n,
            other => panic!("expected number literal, got {other:?}"),
        }
    }

    pub(crate) fn as_float(expr: &Expr) -> f64 {
        match expr {
            Expr::SyntaxLiteral(Literal::FloatNumber(n)) => *n,
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    pub(crate) fn as_string(expr: &Expr) -> &str {
        match expr {
            Expr::SyntaxLiteral(Literal::StringL(s)) => s.as_str(),
            other => panic!("expected string literal, got {other:?}"),
        }
    }

    pub(crate) fn as_ternary(expr: &Expr) -> (&Expr, &Expr, &Expr) {
        match expr {
            Expr::Ternary {
                predicate: condition,
                left,
                right,
            } => (condition, left, right),
            other => panic!("expected ternary expression, got {other:?}"),
        }
    }

    pub(crate) fn parse_with_error(input: &str) -> Result<Expr, RloxError> {
        parse_expression(input)
    }

    pub(crate) fn parse_error_recovery(input: &str) -> Expr {
        let result = parse_with_error(input);
        result.unwrap_or_else(|_| panic!("expected recovered expression for {input:?}"))
    }

    pub(crate) fn as_operator(expr: &Expr) -> TokenType {
        match expr {
            Expr::MissingLeftOperand { token } => token.token_type.clone(),
            other => panic!("expected operator placeholder, got {other:?}"),
        }
    }

    pub(crate) fn assert_missing_left_binary(
        expr: &Expr,
        expected_op: TokenType,
    ) -> (&Expr, &Expr) {
        let (left, op, right) = as_binary(expr);
        assert_eq!(op, expected_op);
        assert_eq!(as_operator(left), expected_op);
        (left, right)
    }

    mod literals {
        use super::*;

        #[test]
        fn parses_integer() {
            parsed!("42", |expr| {
                assert_eq!(as_number(&expr), 42);
            });
        }

        #[test]
        fn parses_float() {
            parsed!("3.15", |expr| {
                assert!((as_float(&expr) - 3.15).abs() < f64::EPSILON);
            });
        }

        #[test]
        fn parses_string() {
            parsed!("\"hello\"", |expr| {
                assert_eq!(as_string(&expr), "hello");
            });
        }

        #[test]
        fn parses_true() {
            assert!(matches!(
                parse_ok("true"),
                Expr::SyntaxLiteral(Literal::True)
            ));
        }

        #[test]
        fn parses_false() {
            assert!(matches!(
                parse_ok("false"),
                Expr::SyntaxLiteral(Literal::False)
            ));
        }

        #[test]
        fn parses_nil() {
            assert!(matches!(parse_ok("nil"), Expr::SyntaxLiteral(Literal::Nil)));
        }
    }

    mod binary_operators {
        use super::*;

        #[test]
        fn parses_addition() {
            parsed!("1 + 2", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            });
        }

        #[test]
        fn parses_subtraction() {
            parsed!("5 - 3", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Minus);
            });
        }

        #[test]
        fn parses_multiplication() {
            parsed!("3 * 4", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Star);
            });
        }

        #[test]
        fn parses_division() {
            parsed!("8 / 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Slash);
            });
        }

        #[test]
        fn parses_equality() {
            parsed!("1 == 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
            });
        }

        #[test]
        fn parses_inequality() {
            parsed!("1 != 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, BangEqual);
            });
        }

        #[test]
        fn parses_less_than() {
            parsed!("1 < 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Less);
            });
        }

        #[test]
        fn parses_less_equal() {
            parsed!("1 <= 2", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, LessEqual);
            });
        }

        #[test]
        fn parses_greater_than() {
            parsed!("2 > 1", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, Greater);
            });
        }

        #[test]
        fn parses_greater_equal() {
            parsed!("2 >= 1", |expr| {
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, GreaterEqual);
            });
        }

        #[test]
        fn parses_chained_addition_left_associative() {
            parsed!("1 + 2 + 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(right), 3);
                let (inner_left, inner_op, inner_right) = as_binary(left);
                assert_eq!(inner_op, Plus);
                assert_eq!(as_number(inner_left), 1);
                assert_eq!(as_number(inner_right), 2);
            });
        }

        #[test]
        fn parses_chained_equality_left_associative() {
            parsed!("1 == 2 == 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert_eq!(as_number(right), 3);
                let (_, inner_op, _) = as_binary(left);
                assert_eq!(inner_op, EqualEqual);
            });
        }
    }

    mod logical {
        use super::*;

        #[test]
        fn parses_logical_and() {
            parsed!("true and false", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, And);
                assert!(matches!(left, Expr::SyntaxLiteral(Literal::True)));
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::False)));
            });
        }

        #[test]
        fn parses_logical_or() {
            parsed!("false or true", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Or);
                assert!(matches!(left, Expr::SyntaxLiteral(Literal::False)));
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::True)));
            });
        }

        #[test]
        fn and_binds_tighter_than_or() {
            parsed!("true or false and nil", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Or);
                assert!(matches!(left, Expr::SyntaxLiteral(Literal::True)));

                let (and_left, and_op, and_right) = as_binary(right);
                assert_eq!(and_op, And);
                assert!(matches!(and_left, Expr::SyntaxLiteral(Literal::False)));
                assert!(matches!(and_right, Expr::SyntaxLiteral(Literal::Nil)));
            });
        }

        #[test]
        fn equality_binds_tighter_than_and() {
            parsed!("1 == 1 and 2 != 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, And);
                assert_eq!(as_binary(left).1, EqualEqual);
                assert_eq!(as_binary(right).1, BangEqual);
            });
        }

        #[test]
        fn logical_operators_are_left_associative() {
            parsed!("true or false or nil", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Or);
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::Nil)));

                let (inner_left, inner_op, inner_right) = as_binary(left);
                assert_eq!(inner_op, Or);
                assert!(matches!(inner_left, Expr::SyntaxLiteral(Literal::True)));
                assert!(matches!(inner_right, Expr::SyntaxLiteral(Literal::False)));
            });
        }

        #[test]
        fn logical_or_binds_tighter_than_ternary() {
            parsed!("false or true ? 1 : 2", |expr| {
                let (predicate, then_branch, else_branch) = as_ternary(&expr);
                assert_eq!(as_binary(predicate).1, Or);
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn grouping_overrides_logical_precedence() {
            parsed!("(true or false) and nil", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, And);
                assert_eq!(as_binary(as_grouping(left)).1, Or);
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::Nil)));
            });
        }

        #[test]
        fn rejects_logical_operator_without_right_operand() {
            parse_expression_fails("true and");
            parse_expression_fails("false or");
        }
    }

    mod precedence {
        use super::*;

        #[test]
        fn multiplication_binds_tighter_than_addition() {
            parsed!("1 + 2 * 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(left), 1);
                let (_, mul_op, _) = as_binary(right);
                assert_eq!(mul_op, Star);
            });
        }

        #[test]
        fn addition_binds_looser_than_multiplication() {
            parsed!("1 * 2 + 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(right), 3);
                let (_, mul_op, _) = as_binary(left);
                assert_eq!(mul_op, Star);
            });
        }

        #[test]
        fn addition_binds_tighter_than_equality() {
            parsed!("1 + 2 == 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert_eq!(as_number(right), 3);
                let (_, add_op, _) = as_binary(left);
                assert_eq!(add_op, Plus);
            });
        }

        #[test]
        fn comparison_binds_tighter_than_equality() {
            parsed!("1 < 2 == 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert_eq!(as_number(right), 3);
                let (_, cmp_op, _) = as_binary(left);
                assert_eq!(cmp_op, Less);
            });
        }

        #[test]
        fn term_binds_tighter_than_comparison() {
            parsed!("1 + 2 < 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Less);
                assert_eq!(as_number(right), 3);
                let (_, add_op, _) = as_binary(left);
                assert_eq!(add_op, Plus);
            });
        }
    }

    mod unary {
        use super::*;

        #[test]
        fn parses_negation() {
            parsed!("-1", |expr| {
                let (op, right) = as_unary(&expr);
                assert_eq!(op, Minus);
                assert_eq!(as_number(right), 1);
            });
        }

        #[test]
        fn parses_logical_not() {
            parsed!("!true", |expr| {
                let (op, right) = as_unary(&expr);
                assert_eq!(op, Bang);
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::True)));
            });
        }

        #[test]
        fn parses_nested_unary_right_associative() {
            parsed!("--1", |expr| {
                let (outer_op, inner) = as_unary(&expr);
                assert_eq!(outer_op, Minus);
                let (inner_op, right) = as_unary(inner);
                assert_eq!(inner_op, Minus);
                assert_eq!(as_number(right), 1);
            });
        }

        #[test]
        fn unary_binds_tighter_than_multiplication() {
            parsed!("-1 * 2", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Star);
                let (unary_op, _) = as_unary(left);
                assert_eq!(unary_op, Minus);
                assert_eq!(as_number(right), 2);
            });
        }

        #[test]
        fn unary_binds_tighter_than_equality() {
            parsed!("!true == false", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                let (bang_op, _) = as_unary(left);
                assert_eq!(bang_op, Bang);
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::False)));
            });
        }
    }

    mod grouping {
        use super::*;

        #[test]
        fn parses_parenthesized_expression() {
            parsed!("(1 + 2)", |expr| {
                let inner = as_grouping(&expr);
                let (_, op, _) = as_binary(inner);
                assert_eq!(op, Plus);
            });
        }

        #[test]
        fn parentheses_override_precedence() {
            parsed!("(1 + 2) * 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Star);
                assert_eq!(as_number(right), 3);
                let inner = as_grouping(left);
                let (_, add_op, _) = as_binary(inner);
                assert_eq!(add_op, Plus);
            });
        }

        #[test]
        fn parses_nested_parentheses() {
            parsed!("((42))", |expr| {
                let inner = as_grouping(&expr);
                let deeper = as_grouping(inner);
                assert_eq!(as_number(deeper), 42);
            });
        }

        #[test]
        fn grouping_can_wrap_comma_expression() {
            parsed!("(1, 2)", |expr| {
                let inner = as_grouping(&expr);
                let (_, op, _) = as_binary(inner);
                assert_eq!(op, Comma);
            });
        }
    }

    mod comma {
        use super::*;

        #[test]
        fn parses_comma_with_both_operands() {
            parsed!("1 + 2 , 2 + 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);

                let (left_left, left_op, left_right) = as_binary(left);
                assert_eq!(left_op, Plus);
                assert_eq!(as_number(left_left), 1);
                assert_eq!(as_number(left_right), 2);

                let (right_left, right_op, right_right) = as_binary(right);
                assert_eq!(right_op, Plus);
                assert_eq!(as_number(right_left), 2);
                assert_eq!(as_number(right_right), 3);
            });
        }

        #[test]
        fn comma_is_left_associative() {
            parsed!("1, 2, 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(right), 3);

                let (inner_left, inner_op, inner_right) = as_binary(left);
                assert_eq!(inner_op, Comma);
                assert_eq!(as_number(inner_left), 1);
                assert_eq!(as_number(inner_right), 2);
            });
        }

        #[test]
        fn comma_has_lower_precedence_than_equality() {
            parsed!("1 == 2, 3", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(right), 3);
                let (_, eq_op, _) = as_binary(left);
                assert_eq!(eq_op, EqualEqual);
            });
        }

        #[test]
        fn comma_has_lower_precedence_than_addition() {
            parsed!("1 + 2, 3 + 4", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                let (_, left_op, _) = as_binary(left);
                assert_eq!(left_op, Plus);
                let (_, right_op, _) = as_binary(right);
                assert_eq!(right_op, Plus);
            });
        }
    }

    mod ternary {
        use super::*;

        #[test]
        fn parses_simple_ternary() {
            parsed!("true ? 1 : 2", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn parses_ternary_with_false_condition() {
            parsed!("false ? 1 : 2", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::False)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn parses_ternary_with_nil_and_string_branches() {
            parsed!("nil ? \"yes\" : \"no\"", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::Nil)));
                assert_eq!(as_string(then_branch), "yes");
                assert_eq!(as_string(else_branch), "no");
            });
        }

        #[test]
        fn chained_ternary_is_right_associative() {
            parsed!("1 ? 2 : 3 ? 4 : 5", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert_eq!(as_number(cond), 1);
                assert_eq!(as_number(then_branch), 2);

                let (inner_cond, inner_then, inner_else) = as_ternary(else_branch);
                assert_eq!(as_number(inner_cond), 3);
                assert_eq!(as_number(inner_then), 4);
                assert_eq!(as_number(inner_else), 5);
            });
        }

        #[test]
        fn nested_ternary_in_then_branch() {
            parsed!("1 ? 2 ? 3 : 4 : 5", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert_eq!(as_number(cond), 1);
                assert_eq!(as_number(else_branch), 5);

                let (inner_cond, inner_then, inner_else) = as_ternary(then_branch);
                assert_eq!(as_number(inner_cond), 2);
                assert_eq!(as_number(inner_then), 3);
                assert_eq!(as_number(inner_else), 4);
            });
        }

        #[test]
        fn nested_ternary_in_else_branch_via_equality_precedence() {
            parsed!("true ? 1 : 2 == 3 ? 4 : 5", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);

                let (inner_cond, inner_then, inner_else) = as_ternary(else_branch);
                let (_, eq_op, _) = as_binary(inner_cond);
                assert_eq!(eq_op, EqualEqual);
                assert_eq!(as_number(inner_then), 4);
                assert_eq!(as_number(inner_else), 5);
            });
        }

        #[test]
        fn arithmetic_in_condition_is_fully_parsed() {
            parsed!("1 + 2 ? 3 : 4", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                let (_, add_op, _) = as_binary(cond);
                assert_eq!(add_op, Plus);
                assert_eq!(as_number(then_branch), 3);
                assert_eq!(as_number(else_branch), 4);
            });
        }

        #[test]
        fn equality_in_condition_is_fully_parsed() {
            parsed!("1 == 2 ? 3 : 4", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                let (_, eq_op, _) = as_binary(cond);
                assert_eq!(eq_op, EqualEqual);
                assert_eq!(as_number(then_branch), 3);
                assert_eq!(as_number(else_branch), 4);
            });
        }

        #[test]
        fn comparison_in_condition_is_fully_parsed() {
            parsed!("1 < 2 ? 3 : 4", |expr| {
                let (cond, _, _) = as_ternary(&expr);
                let (_, cmp_op, _) = as_binary(cond);
                assert_eq!(cmp_op, Less);
            });
        }

        #[test]
        fn unary_in_condition_binds_tighter_than_ternary() {
            parsed!("!true ? 1 : 2", |expr| {
                let (cond, _, _) = as_ternary(&expr);
                let (bang_op, _) = as_unary(cond);
                assert_eq!(bang_op, Bang);
            });
        }

        #[test]
        fn negation_in_condition_binds_tighter_than_ternary() {
            parsed!("-1 ? 2 : 3", |expr| {
                let (cond, _, _) = as_ternary(&expr);
                let (minus_op, inner) = as_unary(cond);
                assert_eq!(minus_op, Minus);
                assert_eq!(as_number(inner), 1);
            });
        }

        #[test]
        fn else_branch_parses_equality_before_next_ternary() {
            parsed!("true ? 1 : 2 == 3", |expr| {
                let (_, _, else_branch) = as_ternary(&expr);
                let (_, eq_op, _) = as_binary(else_branch);
                assert_eq!(eq_op, EqualEqual);
            });
        }

        #[test]
        fn arithmetic_in_branches_is_parsed() {
            parsed!("true ? 1 + 2 : 3 * 4", |expr| {
                let (_, then_branch, else_branch) = as_ternary(&expr);
                let (_, then_op, _) = as_binary(then_branch);
                assert_eq!(then_op, Plus);
                let (_, else_op, _) = as_binary(else_branch);
                assert_eq!(else_op, Star);
            });
        }

        #[test]
        fn parentheses_override_ternary_precedence() {
            parsed!("(1 ? 2 : 3) + 4", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert_eq!(as_number(right), 4);
                let inner = as_grouping(left);
                let (cond, then_branch, else_branch) = as_ternary(inner);
                assert_eq!(as_number(cond), 1);
                assert_eq!(as_number(then_branch), 2);
                assert_eq!(as_number(else_branch), 3);
            });
        }

        #[test]
        fn parentheses_force_left_associative_chaining() {
            parsed!("(1 ? 2 : 3) ? 4 : 5", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                let inner = as_grouping(cond);
                let (inner_cond, inner_then, inner_else) = as_ternary(inner);
                assert_eq!(as_number(inner_cond), 1);
                assert_eq!(as_number(inner_then), 2);
                assert_eq!(as_number(inner_else), 3);
                assert_eq!(as_number(then_branch), 4);
                assert_eq!(as_number(else_branch), 5);
            });
        }

        #[test]
        fn grouping_can_wrap_entire_ternary() {
            parsed!("(true ? 1 : 2)", |expr| {
                let inner = as_grouping(&expr);
                let (cond, then_branch, else_branch) = as_ternary(inner);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn ternary_binds_tighter_than_comma_on_the_right() {
            parsed!("1 ? 2 : 3, 4", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(right), 4);
                let (_, then_branch, else_branch) = as_ternary(left);
                assert_eq!(as_number(then_branch), 2);
                assert_eq!(as_number(else_branch), 3);
            });
        }

        #[test]
        fn ternary_binds_tighter_than_comma_on_the_left() {
            parsed!("1, 2 ? 3 : 4", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(left), 1);
                let (_, then_branch, else_branch) = as_ternary(right);
                assert_eq!(as_number(then_branch), 3);
                assert_eq!(as_number(else_branch), 4);
            });
        }

        #[test]
        fn then_branch_allows_comma_expression() {
            parsed!("true ? 1, 2 : 3", |expr| {
                let (_, then_branch, else_branch) = as_ternary(&expr);
                let (_, comma_op, _) = as_binary(then_branch);
                assert_eq!(comma_op, Comma);
                assert_eq!(as_number(else_branch), 3);
            });
        }

        #[test]
        fn rejects_missing_then_branch() {
            parse_expression_fails("1 ?");
        }

        #[test]
        fn rejects_missing_colon() {
            parse_expression_fails("1 ? 2");
        }

        #[test]
        fn rejects_missing_else_branch() {
            parse_expression_fails("1 ? 2 :");
        }

        #[test]
        fn rejects_leading_question_mark() {
            parse_expression_fails("? 1 : 2");
        }

        #[test]
        fn rejects_bare_question_mark() {
            parse_expression_fails("?");
        }

        #[test]
        fn rejects_double_question_without_colon() {
            parse_expression_fails("1 ? ? 2 : 3");
        }

        #[test]
        fn rejects_ternary_without_condition() {
            parse_expression_fails("? true : false");
        }
    }

    mod complex {
        use super::*;

        #[test]
        fn parses_mixed_expression() {
            parsed!("!(1 + 2) * -3 < 10 == true", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::True)));
                let (_, cmp_op, _) = as_binary(left);
                assert_eq!(cmp_op, Less);
            });
        }

        #[test]
        fn parses_expression_with_string_and_nil() {
            parsed!("\"a\" == nil", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, EqualEqual);
                assert_eq!(as_string(left), "a");
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::Nil)));
            });
        }

        #[test]
        fn parses_float_in_arithmetic() {
            parsed!("1.5 + 2", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Plus);
                assert!((as_float(left) - 1.5).abs() < f64::EPSILON);
                assert_eq!(as_number(right), 2);
            });
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn rejects_unclosed_parenthesis() {
            parse_expression_fails("((1 + 2)");
        }

        #[test]
        fn rejects_unclosed_parenthesis_with_semicolon() {
            parse_statement_fails("((1 + 2);");
        }

        #[test]
        fn rejects_unclosed_parenthesis_with_semicolon_and_recovers() {
            let input = "((1 + 2); var x = 2+3;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let first_parse = parser.parse_statements();
            assert!(first_parse.is_err());
            let second_parse = parser.parse_statements();
            assert!(second_parse.is_ok());
        }

        #[test]
        fn rejects_extra_closing_parenthesis() {
            let scanner = Scanner::new("(1 + 2))".lines());
            let mut parser = Parser::new(scanner);
            let _ = parser.parse_expression();
            let result = parser.parse_expression();
            assert!(result.is_err());
        }

        #[test]
        fn rejects_empty_parentheses() {
            parse_expression_fails("()");
        }

        #[test]
        fn rejects_bare_operator_at_start() {
            parse_error_recovery("+ 1");
        }

        #[test]
        fn rejects_missing_right_operand_after_plus() {
            parse_expression_fails("1 +");
        }

        #[test]
        fn rejects_missing_right_operand_after_equality() {
            parse_expression_fails("1 ==");
        }

        #[test]
        fn rejects_missing_right_operand_after_comma() {
            parse_expression_fails("1,");
        }

        #[test]
        fn rejects_unary_without_operand() {
            parse_expression_fails("!");
        }

        #[test]
        fn parses_identifier_expression() {
            parsed!("foo", |expr| {
                assert!(matches!(expr, Expr::IdentifierExpression(_)));
            });
        }

        #[test]
        fn rejects_unexpected_token() {
            parse_expression_fails(")");
        }

        #[test]
        fn rejects_empty_input() {
            parse_expression_fails("");
        }
    }

    mod missing_left_operand {
        use super::*;

        mod term {
            use super::*;

            #[test]
            fn recovers_bare_plus_with_number() {
                let expr = parse_error_recovery("+ 1");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                assert_eq!(as_number(right), 1);
            }

            #[test]
            fn recovers_bare_plus_with_float() {
                let expr = parse_error_recovery("+ 1.5");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                assert!((as_float(right) - 1.5).abs() < f64::EPSILON);
            }

            #[test]
            fn recovers_bare_plus_with_string() {
                let expr = parse_error_recovery("+ \"hi\"");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                assert_eq!(as_string(right), "hi");
            }

            #[test]
            fn rhs_parses_full_product_for_plus() {
                let expr = parse_error_recovery("+ 1 * 2");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                let (left, op, right) = as_binary(right);
                assert_eq!(op, Star);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn rhs_parses_chained_addition() {
                let expr = parse_error_recovery("+ 1 + 2");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                let (inner_left, inner_op, inner_right) = as_binary(right);
                assert_eq!(inner_op, Plus);
                assert_eq!(as_number(inner_left), 1);
                assert_eq!(as_number(inner_right), 2);
            }

            #[test]
            fn chained_missing_plus_operators() {
                let expr = parse_error_recovery("+ + 1");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                let (_, inner_op, inner_right) = as_binary(right);
                assert_eq!(inner_op, Plus);
                assert_eq!(as_number(inner_right), 1);
            }

            #[test]
            fn recovers_plus_with_unary_negation_rhs() {
                let expr = parse_error_recovery("+ -1");
                let (_, right) = assert_missing_left_binary(&expr, Plus);
                let (unary_op, inner) = as_unary(right);
                assert_eq!(unary_op, Minus);
                assert_eq!(as_number(inner), 1);
            }

            #[test]
            fn recovers_inside_parentheses() {
                let expr = parse_error_recovery("(+ 1)");
                let inner = as_grouping(&expr);
                let (_, right) = assert_missing_left_binary(inner, Plus);
                assert_eq!(as_number(right), 1);
            }

            #[test]
            fn unary_minus_is_not_missing_left_operand() {
                parsed!("-1", |expr| {
                    let (op, right) = as_unary(&expr);
                    assert_eq!(op, Minus);
                    assert_eq!(as_number(right), 1);
                });
            }

            #[test]
            fn spaced_unary_minus_is_not_missing_left_operand() {
                parsed!("- 1", |expr| {
                    let (op, right) = as_unary(&expr);
                    assert_eq!(op, Minus);
                    assert_eq!(as_number(right), 1);
                });
            }
        }

        mod factor {
            use super::*;

            #[test]
            fn recovers_bare_star_with_number() {
                let expr = parse_error_recovery("* 2");
                let (_, right) = assert_missing_left_binary(&expr, Star);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn recovers_bare_slash_with_number() {
                let expr = parse_error_recovery("/ 4");
                let (_, right) = assert_missing_left_binary(&expr, Slash);
                assert_eq!(as_number(right), 4);
            }

            #[test]
            fn rhs_parses_full_quotient() {
                let expr = parse_error_recovery("* 6 / 2");
                let (_, right) = assert_missing_left_binary(&expr, Star);
                let (left, op, right) = as_binary(right);
                assert_eq!(op, Slash);
                assert_eq!(as_number(left), 6);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn rhs_parses_unary_operand() {
                let expr = parse_error_recovery("* -3");
                let (_, right) = assert_missing_left_binary(&expr, Star);
                let (unary_op, inner) = as_unary(right);
                assert_eq!(unary_op, Minus);
                assert_eq!(as_number(inner), 3);
            }

            #[test]
            fn chained_missing_star_operators() {
                let expr = parse_error_recovery("* * 2");
                let (_, right) = assert_missing_left_binary(&expr, Star);
                let (_, inner_op, inner_right) = as_binary(right);
                assert_eq!(inner_op, Star);
                assert_eq!(as_number(inner_right), 2);
            }
        }

        mod comparison {
            use super::*;

            #[test]
            fn recovers_bare_less_with_number() {
                let expr = parse_error_recovery("< 3");
                let (_, right) = assert_missing_left_binary(&expr, Less);
                assert_eq!(as_number(right), 3);
            }

            #[test]
            fn recovers_bare_less_equal_with_number() {
                let expr = parse_error_recovery("<= 3");
                let (_, right) = assert_missing_left_binary(&expr, LessEqual);
                assert_eq!(as_number(right), 3);
            }

            #[test]
            fn recovers_bare_greater_with_number() {
                let expr = parse_error_recovery("> 5");
                let (_, right) = assert_missing_left_binary(&expr, Greater);
                assert_eq!(as_number(right), 5);
            }

            #[test]
            fn recovers_bare_greater_equal_with_number() {
                let expr = parse_error_recovery(">= 5");
                let (_, right) = assert_missing_left_binary(&expr, GreaterEqual);
                assert_eq!(as_number(right), 5);
            }

            #[test]
            fn rhs_parses_addition_within_comparison() {
                let expr = parse_error_recovery("< 1 + 2");
                let (_, right) = assert_missing_left_binary(&expr, Less);
                let (left, op, right) = as_binary(right);
                assert_eq!(op, Plus);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            }
        }

        mod equality {
            use super::*;

            #[test]
            fn recovers_bare_equal_equal_with_number() {
                let expr = parse_error_recovery("== 1");
                let (_, right) = assert_missing_left_binary(&expr, EqualEqual);
                assert_eq!(as_number(right), 1);
            }

            #[test]
            fn recovers_bare_bang_equal_with_number() {
                let expr = parse_error_recovery("!= 2");
                let (_, right) = assert_missing_left_binary(&expr, BangEqual);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn rhs_parses_comparison_within_equality() {
                let expr = parse_error_recovery("== 1 < 2");
                let (_, right) = assert_missing_left_binary(&expr, EqualEqual);
                let (_, cmp_op, _) = as_binary(right);
                assert_eq!(cmp_op, Less);
            }

            #[test]
            fn recovers_inside_parentheses() {
                let expr = parse_error_recovery("(== 1)");
                let inner = as_grouping(&expr);
                let (_, right) = assert_missing_left_binary(inner, EqualEqual);
                assert_eq!(as_number(right), 1);
            }
        }

        mod comma {
            use super::*;

            #[test]
            fn recovers_bare_comma_with_number() {
                let expr = parse_error_recovery(", 1");
                let (_, right) = assert_missing_left_binary(&expr, Comma);
                assert_eq!(as_number(right), 1);
            }

            #[test]
            fn rhs_parses_comma_expression() {
                let expr = parse_error_recovery(", 1, 2");
                let (_, right) = assert_missing_left_binary(&expr, Comma);
                let (left, op, right) = as_binary(right);
                assert_eq!(op, Comma);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn rhs_parses_ternary_within_comma_expression() {
                let expr = parse_error_recovery(", true ? 1 : 2");
                let (_, right) = assert_missing_left_binary(&expr, Comma);
                let (condition, then_branch, else_branch) = as_ternary(right);
                assert!(matches!(condition, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            }
        }

        mod recovery {
            use super::*;

            #[test]
            fn returns_ok_for_recovered_expression() {
                let result = parse_with_error("+ 1");
                assert!(result.is_ok());
            }

            #[test]
            fn still_returns_syntax_tree() {
                let result = parse_with_error("+ 1");
                assert!(result.is_ok());
            }

            #[test]
            fn valid_expressions_remain_unaffected() {
                parse_ok("1 + 2 * 3");
            }

            #[test]
            fn bare_operator_without_rhs_still_fails() {
                parse_expression_fails("+");
            }

            #[test]
            fn middle_operand_still_requires_expression() {
                parse_expression_fails("1 * + 2");
            }
        }
    }

    mod statements {
        use super::*;

        mod break_statement {
            use super::*;

            #[test]
            fn parses_break_as_while_body() {
                parsed_stmt!("while (true) break;", |stmt| {
                    let Statement::WhileStatement { body, .. } = stmt else {
                        panic!("expected while statement");
                    };
                    let Statement::BreakStatement(token) = *body else {
                        panic!("expected break statement");
                    };

                    assert_eq!(token.token_type, TokenType::Break);
                    assert_eq!(token.lexeme, "break");
                });
            }

            #[test]
            fn parses_break_nested_in_loop_block_and_if() {
                parsed_stmt!("while (true) { if (ready) break; }", |stmt| {
                    let Statement::WhileStatement { body, .. } = stmt else {
                        panic!("expected while statement");
                    };
                    let Statement::BlockStatement(block) = *body else {
                        panic!("expected loop body block");
                    };
                    let statements = block.statements;
                    let Statement::IfStatement { then, .. } = &statements[0] else {
                        panic!("expected if statement");
                    };
                    let Statement::BreakStatement(token) = then.as_ref() else {
                        panic!("expected break statement");
                    };

                    assert_eq!(token.token_type, TokenType::Break);
                    assert_eq!(token.lexeme, "break");
                });
            }

            #[test]
            fn preserves_break_before_desugared_for_increment() {
                parsed_stmt!("for (var i = 0; i < 3; i = i + 1) break;", |stmt| {
                    let BlockStatement(block) = stmt else {
                        panic!("expected initializer block");
                    };
                    let statements = block.statements;
                    let WhileStatement { body, .. } = &statements[1] else {
                        panic!("expected desugared while statement");
                    };
                    let BlockStatement(body_block) = body.as_ref() else {
                        panic!("expected desugared while body block");
                    };
                    let body = &body_block.statements;
                    let Statement::BreakStatement(token) = &body[0] else {
                        panic!("expected break statement");
                    };

                    assert_eq!(token.token_type, TokenType::Break);
                    assert_eq!(token.lexeme, "break");
                    assert!(matches!(body[1], Statement::ExprStatement(_)));
                });
            }

            #[test]
            fn rejects_break_without_semicolon() {
                parse_statement_fails("while (true) break");
            }

            #[test]
            fn rejects_break_with_a_value() {
                parse_statement_fails("while (true) break 1;");
            }

            #[test]
            fn rejects_break_outside_loop() {
                parse_statement_fails("break;");
            }

            #[test]
            fn rejects_break_in_non_loop_block() {
                parse_statement_fails("{ break; }");
            }

            #[test]
            fn rejects_break_in_if_outside_loop() {
                parse_statement_fails("if (true) break;");
            }

            #[test]
            fn loop_context_does_not_leak_to_following_statement() {
                let scanner = Scanner::new("while (false) 1; break;".lines());
                let mut parser = Parser::new(scanner);

                assert!(parser.parse_statements().is_ok());
                assert!(parser.parse_statements().is_err());
            }
        }

        mod if_statement {
            use super::*;

            #[test]
            fn parses_if_with_expression_statement() {
                parsed_stmt!("if (true) 1;", |stmt| {
                    let Statement::IfStatement {
                        expr,
                        then,
                        else_stmnt,
                    } = stmt
                    else {
                        panic!("expected if statement");
                    };

                    assert!(matches!(expr, Expr::SyntaxLiteral(Literal::True)));
                    assert!(matches!(*then, Statement::ExprStatement(_)));
                    assert!(else_stmnt.is_none());
                });
            }

            #[test]
            fn parses_if_else() {
                parsed_stmt!("if (false) 1; else 2;", |stmt| {
                    let Statement::IfStatement {
                        then, else_stmnt, ..
                    } = stmt
                    else {
                        panic!("expected if statement");
                    };

                    assert!(matches!(*then, Statement::ExprStatement(_)));
                    assert!(matches!(
                        else_stmnt.as_deref(),
                        Some(Statement::ExprStatement(_))
                    ));
                });
            }

            #[test]
            fn parses_block_branches() {
                parsed_stmt!("if (true) { print 1; } else { print 2; }", |stmt| {
                    let Statement::IfStatement {
                        then, else_stmnt, ..
                    } = stmt
                    else {
                        panic!("expected if statement");
                    };

                    assert!(matches!(*then, Statement::BlockStatement(_)));
                    assert!(matches!(
                        else_stmnt.as_deref(),
                        Some(Statement::BlockStatement(_))
                    ));
                });
            }

            #[test]
            fn dangling_else_belongs_to_nearest_if() {
                parsed_stmt!("if (true) if (false) 1; else 2;", |stmt| {
                    let Statement::IfStatement {
                        then, else_stmnt, ..
                    } = stmt
                    else {
                        panic!("expected outer if statement");
                    };
                    assert!(else_stmnt.is_none());

                    let Statement::IfStatement {
                        else_stmnt: inner_else,
                        ..
                    } = *then
                    else {
                        panic!("expected nested if statement");
                    };
                    assert!(inner_else.is_some());
                });
            }

            #[test]
            fn rejects_missing_condition_parentheses() {
                parse_statement_fails("if true 1;");
                parse_statement_fails("if (true 1;");
            }
        }

        mod while_statement {
            use super::*;

            #[test]
            fn parses_expression_body() {
                parsed_stmt!("while (true) 1;", |stmt| {
                    let Statement::WhileStatement { expr, body } = stmt else {
                        panic!("expected while statement");
                    };

                    assert!(matches!(expr, Expr::SyntaxLiteral(Literal::True)));
                    assert!(matches!(*body, Statement::ExprStatement(_)));
                });
            }

            #[test]
            fn parses_block_body_with_multiple_statements() {
                parsed_stmt!("while (i < 3) { print i; i = i + 1; }", |stmt| {
                    let Statement::WhileStatement { expr, body } = stmt else {
                        panic!("expected while statement");
                    };

                    let (_, op, _) = as_binary(&expr);
                    assert_eq!(op, Less);

                    let Statement::BlockStatement(block) = *body else {
                        panic!("expected block body");
                    };
                    let statements = block.statements;
                    assert_eq!(statements.len(), 2);
                    assert!(matches!(statements[0], Statement::Print(_)));
                    assert!(matches!(statements[1], Statement::ExprStatement(_)));
                });
            }

            #[test]
            fn parses_empty_block_body() {
                parsed_stmt!("while (false) {}", |stmt| {
                    let Statement::WhileStatement { body, .. } = stmt else {
                        panic!("expected while statement");
                    };

                    let Statement::BlockStatement(block) = *body else {
                        panic!("expected block body");
                    };
                    let statements = block.statements;
                    assert!(statements.is_empty());
                });
            }

            #[test]
            fn parses_nested_while_statements() {
                parsed_stmt!("while (true) while (false) 1;", |stmt| {
                    let Statement::WhileStatement { body, .. } = stmt else {
                        panic!("expected outer while statement");
                    };
                    assert!(matches!(*body, Statement::WhileStatement { .. }));
                });
            }

            #[test]
            fn parses_assignment_condition() {
                parsed_stmt!("while (running = false) 1;", |stmt| {
                    let Statement::WhileStatement { expr, .. } = stmt else {
                        panic!("expected while statement");
                    };

                    let AssignmentExpression { left, right } = expr else {
                        panic!("expected assignment condition");
                    };
                    assert_eq!(left.token.lexeme, "running");
                    assert!(matches!(*right, Expr::SyntaxLiteral(Literal::False)));
                });
            }

            #[test]
            fn while_body_can_be_if_else_statement() {
                parsed_stmt!("while (true) if (false) 1; else 2;", |stmt| {
                    let Statement::WhileStatement { body, .. } = stmt else {
                        panic!("expected while statement");
                    };

                    let Statement::IfStatement { else_stmnt, .. } = *body else {
                        panic!("expected if body");
                    };
                    assert!(else_stmnt.is_some());
                });
            }

            #[test]
            fn rejects_missing_condition_parentheses() {
                parse_statement_fails("while true 1;");
                parse_statement_fails("while (true 1;");
            }

            #[test]
            fn rejects_missing_condition() {
                parse_statement_fails("while () 1;");
            }

            #[test]
            fn rejects_missing_body() {
                parse_statement_fails("while (true)");
            }
        }

        mod for_statement {
            use super::*;

            #[test]
            fn desugars_all_clauses() {
                parsed_stmt!("for (var i = 0; i < 3; i = i + 1) print i;", |stmt| {
                    let Statement::BlockStatement(block) = stmt else {
                        panic!("expected initializer block");
                    };
                    let statements = block.statements;
                    assert_eq!(statements.len(), 2);

                    let Statement::DeclarationStatement(name, Some(initializer)) = &statements[0]
                    else {
                        panic!("expected variable initializer");
                    };
                    assert_eq!(name.lexeme, "i");
                    assert_eq!(as_number(initializer), 0);

                    let Statement::WhileStatement { expr, body } = &statements[1] else {
                        panic!("expected desugared while statement");
                    };
                    let (left, op, right) = as_binary(expr);
                    assert_eq!(op, Less);
                    assert!(matches!(
                        left,
                        Expr::IdentifierExpression(ident) if ident.token.lexeme == "i"
                    ));
                    assert_eq!(as_number(right), 3);

                    let Statement::BlockStatement(body_block) = body.as_ref() else {
                        panic!("expected desugared while body block");
                    };
                    let body = &body_block.statements;
                    assert_eq!(body.len(), 2);
                    assert!(matches!(body[0], Statement::Print(_)));

                    let Statement::ExprStatement(AssignmentExpression { left, right }) = &body[1]
                    else {
                        panic!("expected increment assignment");
                    };
                    assert_eq!(left.token.lexeme, "i");
                    let (left, op, right) = as_binary(right);
                    assert_eq!(op, Plus);
                    assert!(matches!(
                        left,
                        Expr::IdentifierExpression(ident) if ident.token.lexeme == "i"
                    ));
                    assert_eq!(as_number(right), 1);
                });
            }

            #[test]
            fn desugars_expression_initializer() {
                parsed_stmt!("for (i = 0; i < 2; i = i + 1) 1;", |stmt| {
                    let Statement::BlockStatement(block) = stmt else {
                        panic!("expected initializer block");
                    };
                    let statements = block.statements;
                    assert_eq!(statements.len(), 2);
                    assert!(matches!(
                        statements[0],
                        Statement::ExprStatement(Expr::AssignmentExpression { .. })
                    ));
                    assert!(matches!(statements[1], Statement::WhileStatement { .. }));
                });
            }

            #[test]
            fn omitted_initializer_does_not_add_outer_block() {
                parsed_stmt!("for (; i < 2; i = i + 1) print i;", |stmt| {
                    assert!(matches!(stmt, WhileStatement { .. }));
                });
            }

            #[test]
            fn omitted_condition_defaults_to_true() {
                parsed_stmt!("for (var i = 0;; i = i + 1) print i;", |stmt| {
                    let Statement::BlockStatement(block) = stmt else {
                        panic!("expected initializer block");
                    };
                    let statements = block.statements;
                    let Statement::WhileStatement { expr, .. } = &statements[1] else {
                        panic!("expected desugared while statement");
                    };
                    assert!(matches!(expr, Expr::SyntaxLiteral(Literal::True)));
                });
            }

            #[test]
            fn omitted_increment_does_not_add_increment_statement() {
                parsed_stmt!("for (var ignored; keep_going;) print 1;", |stmt| {
                    let Statement::BlockStatement(block) = stmt else {
                        panic!("expected initializer block");
                    };
                    let statements = block.statements;
                    let Statement::WhileStatement { body, .. } = &statements[1] else {
                        panic!("expected desugared while statement");
                    };
                    let Statement::BlockStatement(body_block) = body.as_ref() else {
                        panic!("expected desugared while body block");
                    };
                    let body = &body_block.statements;
                    assert_eq!(body.len(), 1);
                    assert!(matches!(body[0], Statement::Print(_)));
                });
            }

            #[test]
            fn supports_all_three_omitted_clauses() {
                parsed_stmt!("for (;;) print 1;", |stmt| {
                    let WhileStatement { expr, body } = stmt else {
                        panic!("expected desugared while statement");
                    };
                    assert!(matches!(expr, Expr::SyntaxLiteral(Literal::True)));
                    let BlockStatement(body_block) = *body else {
                        panic!("expected desugared while body block");
                    };
                    let body = body_block.statements;
                    assert_eq!(body.len(), 1);
                    assert!(matches!(body[0], Statement::Print(_)));
                });
            }

            #[test]
            fn preserves_block_body_before_increment() {
                parsed_stmt!(
                    "for (var ignored; true; i = i + 1) { print i; print i + 1; }",
                    |stmt| {
                        let BlockStatement(block) = stmt else {
                            panic!("expected initializer block");
                        };
                        let statements = block.statements;
                        let WhileStatement { body, .. } = &statements[1] else {
                            panic!("expected desugared while statement");
                        };
                        let BlockStatement(body_block) = body.as_ref() else {
                            panic!("expected desugared while body block");
                        };
                        let body = &body_block.statements;
                        assert_eq!(body.len(), 2);
                        let BlockStatement(original_block) = &body[0] else {
                            panic!("expected original block body");
                        };
                        let original_body = &original_block.statements;
                        assert_eq!(original_body.len(), 2);
                        assert!(matches!(body[1], Statement::ExprStatement(_)));
                    }
                );
            }

            #[test]
            fn supports_nested_for_statements() {
                parsed_stmt!("for (var outer;;) for (var inner;;) print 1;", |stmt| {
                    let Statement::BlockStatement(block) = stmt else {
                        panic!("expected outer initializer block");
                    };
                    let statements = block.statements;
                    let Statement::WhileStatement { body, .. } = &statements[1] else {
                        panic!("expected outer desugared while statement");
                    };
                    let Statement::BlockStatement(body_block) = body.as_ref() else {
                        panic!("expected outer while body block");
                    };
                    let body = &body_block.statements;
                    assert!(matches!(body[0], Statement::BlockStatement(_)));
                });
            }

            #[test]
            fn body_can_be_if_else_statement() {
                parsed_stmt!(
                    "for (var ignored;;) if (true) print 1; else print 2;",
                    |stmt| {
                        let Statement::BlockStatement(block) = stmt else {
                            panic!("expected initializer block");
                        };
                        let statements = block.statements;
                        let Statement::WhileStatement { body, .. } = &statements[1] else {
                            panic!("expected desugared while statement");
                        };
                        let Statement::BlockStatement(body_block) = body.as_ref() else {
                            panic!("expected desugared while body block");
                        };
                        let body = &body_block.statements;
                        assert!(matches!(
                            body[0],
                            Statement::IfStatement {
                                else_stmnt: Some(_),
                                ..
                            }
                        ));
                    }
                );
            }

            #[test]
            fn rejects_missing_opening_parenthesis() {
                parse_statement_fails("for ;;) print 1;");
            }

            #[test]
            fn rejects_missing_initializer_semicolon() {
                parse_statement_fails("for (var i = 0 i < 3; i = i + 1) print i;");
            }

            #[test]
            fn rejects_missing_condition_semicolon() {
                parse_statement_fails("for (var i = 0; i < 3 i = i + 1) print i;");
            }

            #[test]
            fn rejects_missing_closing_parenthesis() {
                parse_statement_fails("for (var i = 0; i < 3; i = i + 1 print i;");
            }

            #[test]
            fn rejects_missing_initializer_expression() {
                parse_statement_fails("for (var i =; i < 3; i = i + 1) print i;");
            }

            #[test]
            fn rejects_missing_body() {
                parse_statement_fails("for (;;)");
            }
        }

        mod print_statement {
            use super::*;

            #[test]
            fn parses_simple_print_statement() {
                parsed_stmt!("print 1 + 2;", |stmt| {
                    assert!(matches!(stmt, Statement::Print(_)));
                });
            }

            #[test]
            fn parses_print_with_complex_expression() {
                parsed_stmt!("print (1 + 2) * 3;", |stmt| {
                    assert!(matches!(stmt, Statement::Print(_)));
                });
            }

            #[test]
            fn parses_print_with_string() {
                parsed_stmt!(r#"print "hello world";"#, |stmt| {
                    assert!(matches!(stmt, Statement::Print(_)));
                });
            }

            #[test]
            fn parses_print_with_ternary() {
                parsed_stmt!("print true ? 1 : 2;", |stmt| {
                    assert!(matches!(stmt, Statement::Print(_)));
                });
            }

            #[test]
            fn parses_multiple_print_statements() {
                let input = "print 1; print 2; print 3;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(stmt1.is_ok());
                assert!(matches!(stmt1.unwrap(), Statement::Print(_)));

                let stmt2 = parser.parse_statements();
                assert!(stmt2.is_ok());
                assert!(matches!(stmt2.unwrap(), Statement::Print(_)));

                let stmt3 = parser.parse_statements();
                assert!(stmt3.is_ok());
                assert!(matches!(stmt3.unwrap(), Statement::Print(_)));

                let stmt4 = parser.parse_statements();
                assert!(stmt4.is_err()); // EOF
            }

            #[test]
            fn rejects_missing_semicolon_after_print() {
                parse_statement_fails("print 1");
            }

            #[test]
            fn rejects_missing_expression_after_print() {
                parse_statement_fails("print;");
            }

            #[test]
            fn rejects_missing_semicolon_and_recovers() {
                let input = "print 1; print 2";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let first = parser.parse_statements();
                assert!(first.is_ok());

                let second = parser.parse_statements();
                assert!(second.is_err());
            }
        }

        mod declaration_statement {
            use super::*;

            #[test]
            fn parses_simple_var_declaration() {
                parsed_stmt!("var x = 1;", |stmt| {
                    assert!(matches!(stmt, Statement::DeclarationStatement(_, _)));
                });
            }

            #[test]
            fn parses_var_declaration_with_expression() {
                parsed_stmt!("var x = 1 + 2 * 3;", |stmt| {
                    assert!(matches!(stmt, Statement::DeclarationStatement(_, _)));
                });
            }

            #[test]
            fn parses_var_declaration_with_string() {
                parsed_stmt!(r#"var name = "hello";"#, |stmt| {
                    assert!(matches!(stmt, Statement::DeclarationStatement(_, _)));
                });
            }

            #[test]
            fn parses_var_declaration_with_ternary() {
                parsed_stmt!("var x = true ? 1 : 2;", |stmt| {
                    assert!(matches!(stmt, Statement::DeclarationStatement(_, _)));
                });
            }

            #[test]
            fn parses_multiple_var_declarations() {
                let input = "var a = 1; var b = 2; var c = 3;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(stmt1.is_ok());
                assert!(matches!(
                    stmt1.unwrap(),
                    Statement::DeclarationStatement(_, _)
                ));

                let stmt2 = parser.parse_statements();
                assert!(stmt2.is_ok());
                assert!(matches!(
                    stmt2.unwrap(),
                    Statement::DeclarationStatement(_, _)
                ));

                let stmt3 = parser.parse_statements();
                assert!(stmt3.is_ok());
                assert!(matches!(
                    stmt3.unwrap(),
                    Statement::DeclarationStatement(_, _)
                ));
            }

            #[test]
            fn rejects_missing_semicolon_after_declaration() {
                parse_statement_fails("var x = 1");
            }

            #[test]
            fn parses_var_declaration_without_initializer() {
                let input = "var x;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);
                let result = parser.parse_statements();
                assert!(result.is_ok(), "Failed to parse: {:?}", result);
                if let Ok(DeclarationStatement(name, init)) = result {
                    assert_eq!(name.lexeme, "x");
                    assert!(init.is_none());
                } else {
                    panic!("expected DeclarationStatement");
                }
            }

            #[test]
            fn parses_var_declaration_without_initializer_multiple() {
                let input = "var a; var b = 2; var c;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(stmt1.is_ok());
                if let DeclarationStatement(name, init) = stmt1.unwrap() {
                    assert_eq!(name.lexeme, "a");
                    assert!(init.is_none());
                }

                let stmt2 = parser.parse_statements();
                assert!(stmt2.is_ok());
                if let DeclarationStatement(name, init) = stmt2.unwrap() {
                    assert_eq!(name.lexeme, "b");
                    assert!(init.is_some());
                }

                let stmt3 = parser.parse_statements();
                assert!(stmt3.is_ok());
                if let DeclarationStatement(name, init) = stmt3.unwrap() {
                    assert_eq!(name.lexeme, "c");
                    assert!(init.is_none());
                }
            }

            #[test]
            fn parses_mixed_initialized_and_uninitialized() {
                let input = "var a = 1; var b; var c = 3; var d;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let expected = vec![
                    ("a", true),  // initialized
                    ("b", false), // uninitialized
                    ("c", true),  // initialized
                    ("d", false), // uninitialized
                ];

                for (name, should_have_init) in expected {
                    let stmt = parser.parse_statements();
                    assert!(stmt.is_ok());
                    if let DeclarationStatement(n, init) = stmt.unwrap() {
                        assert_eq!(n.lexeme, name);
                        assert_eq!(init.is_some(), should_have_init, "{name} init mismatch");
                    } else {
                        panic!("expected DeclarationStatement");
                    }
                }

                // Next should be EOF
                assert!(parser.parse_statements().is_err());
            }

            #[test]
            fn parses_declaration_without_initializer_then_print() {
                let input = "var x; print 42;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(stmt1.is_ok());
                if let DeclarationStatement(name, init) = stmt1.unwrap() {
                    assert_eq!(name.lexeme, "x");
                    assert!(init.is_none());
                }

                let stmt2 = parser.parse_statements();
                assert!(stmt2.is_ok());
                assert!(matches!(stmt2.unwrap(), Statement::Print(_)));
            }

            #[test]
            fn parses_var_declaration_with_complex_initializer() {
                let input = "var x = (1 + 2) * 3 - 4 / 5;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt = parser.parse_statements();
                assert!(stmt.is_ok());
                if let DeclarationStatement(name, init) = stmt.unwrap() {
                    assert_eq!(name.lexeme, "x");
                    assert!(init.is_some());
                }
            }

            #[test]
            fn parses_multiple_uninitialized_then_initialized() {
                // All uninitialized first, then initialized
                let input = "var a; var b; var c = 1 + 2;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(
                    matches!(stmt1.unwrap(), Statement::DeclarationStatement(n, i) if n.lexeme == "a" && i.is_none())
                );

                let stmt2 = parser.parse_statements();
                assert!(
                    matches!(stmt2.unwrap(), Statement::DeclarationStatement(n, i) if n.lexeme == "b" && i.is_none())
                );

                let stmt3 = parser.parse_statements();
                assert!(
                    matches!(stmt3.unwrap(), Statement::DeclarationStatement(n, i) if n.lexeme == "c" && i.is_some())
                );
            }

            #[test]
            fn rejects_equals_without_expression() {
                // 'var x = ;' should fail - equals present but no expression
                parse_statement_fails("var x = ;");
            }

            #[test]
            fn parses_uninitialized_with_ternary_in_next_declaration() {
                let input = "var a; var b = true ? 1 : 2;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(
                    matches!(stmt1.unwrap(), Statement::DeclarationStatement(n, i) if n.lexeme == "a" && i.is_none())
                );

                let stmt2 = parser.parse_statements();
                assert!(
                    matches!(stmt2.unwrap(), Statement::DeclarationStatement(n, i) if n.lexeme == "b" && i.is_some())
                );
            }

            #[test]
            fn parses_uninitialized_then_expression_statement() {
                let input = "var x; 1 + 2;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(
                    matches!(stmt1.unwrap(), Statement::DeclarationStatement(n, i) if n.lexeme == "x" && i.is_none())
                );

                let stmt2 = parser.parse_statements();
                assert!(matches!(stmt2.unwrap(), Statement::ExprStatement(_)));
            }

            #[test]
            fn initializer_none_is_distinct_from_nil_literal() {
                // var x; should have init = None, not init = Some(nil)
                let input = "var x; var y = nil;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                if let Statement::DeclarationStatement(name, init) = stmt1.unwrap() {
                    assert_eq!(name.lexeme, "x");
                    assert!(
                        init.is_none(),
                        "uninitialized should be None, not Some(nil)"
                    );
                }

                let stmt2 = parser.parse_statements();
                if let DeclarationStatement(name, init) = stmt2.unwrap() {
                    assert_eq!(name.lexeme, "y");
                    assert!(init.is_some(), "initialized with nil should be Some");
                    // The inner expression should be a nil literal
                    if let Some(expr) = init {
                        assert!(matches!(expr, Expr::SyntaxLiteral(Literal::Nil)));
                    }
                }
            }

            #[test]
            fn rejects_missing_equals() {
                parse_statement_fails("var x 1;");
            }

            #[test]
            fn rejects_missing_variable_name() {
                parse_statement_fails("var = 1;");
            }

            #[test]
            fn do_not_rejects_missing_var_keyword() {
                // its treated as assignment
                parse_statement_ok("x = 1;");
            }

            #[test]
            fn rejects_invalid_identifier() {
                parse_statement_fails("var 123 = 1;");
            }
        }

        mod expression_statement {
            use super::*;

            #[test]
            fn parses_expression_statement() {
                parsed_stmt!("1 + 2;", |stmt| {
                    assert!(matches!(stmt, Statement::ExprStatement(_)));
                });
            }

            #[test]
            fn parses_complex_expression_statement() {
                parsed_stmt!("(1 + 2) * 3 == 4;", |stmt| {
                    assert!(matches!(stmt, Statement::ExprStatement(_)));
                });
            }

            #[test]
            fn parses_expression_statement_with_comma() {
                parsed_stmt!("1, 2, 3;", |stmt| {
                    assert!(matches!(stmt, Statement::ExprStatement(_)));
                });
            }

            #[test]
            fn rejects_missing_semicolon() {
                parse_statement_fails("1 + 2");
            }

            #[test]
            fn parses_multiple_expression_statements() {
                let input = "1 + 2; 3 * 4; 5 - 6;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(stmt1.is_ok());
                assert!(matches!(stmt1.unwrap(), Statement::ExprStatement(_)));

                let stmt2 = parser.parse_statements();
                assert!(stmt2.is_ok());
                assert!(matches!(stmt2.unwrap(), Statement::ExprStatement(_)));

                let stmt3 = parser.parse_statements();
                assert!(stmt3.is_ok());
                assert!(matches!(stmt3.unwrap(), Statement::ExprStatement(_)));
            }
        }

        mod mixed_statements {
            use super::*;

            #[test]
            fn parses_mixed_declaration_and_print() {
                // Parser doesn't support variable references, use literals
                let input = "var x = 1; print 42;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(stmt1.is_ok());
                assert!(matches!(
                    stmt1.unwrap(),
                    Statement::DeclarationStatement(_, _)
                ));

                let stmt2 = parser.parse_statements();
                assert!(stmt2.is_ok());
                assert!(matches!(stmt2.unwrap(), Statement::Print(_)));
            }

            #[test]
            fn parses_mixed_expression_and_declaration() {
                let input = "1 + 2; var x = 3;";
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let stmt1 = parser.parse_statements();
                assert!(stmt1.is_ok());
                assert!(matches!(stmt1.unwrap(), Statement::ExprStatement(_)));

                let stmt2 = parser.parse_statements();
                assert!(stmt2.is_ok());
                assert!(matches!(
                    stmt2.unwrap(),
                    Statement::DeclarationStatement(_, _)
                ));
            }

            #[test]
            fn parses_complex_program() {
                // Parser doesn't support variable references, use literals
                let input = r#"
                    var a = 1;
                    var b = 2;
                    print 1 + 2;
                    var c = 3;
                    print 3;
                    42;
                "#;
                let scanner = Scanner::new(input.lines());
                let mut parser = Parser::new(scanner);

                let mut count = 0;
                while let Ok(_) = parser.parse_statements() {
                    count += 1;
                }
                assert_eq!(count, 6);
            }
        }
    }

    mod synchronization {
        use super::*;

        #[test]
        fn recovers_after_invalid_expression_statement() {
            let input = "1 + ; var x = 2;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(
                second.unwrap(),
                Statement::DeclarationStatement(_, _)
            ));
        }

        #[test]
        fn recovers_after_missing_semicolon() {
            let input = "1 + 2 var x = 3;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(
                second.unwrap(),
                Statement::DeclarationStatement(_, _)
            ));
        }

        #[test]
        fn recovers_after_unclosed_parenthesis() {
            let input = "((1 + 2); var x = 3;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(
                second.unwrap(),
                Statement::DeclarationStatement(_, _)
            ));
        }

        #[test]
        fn recovers_after_invalid_declaration() {
            let input = "var = 1; var x = 2;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(
                second.unwrap(),
                Statement::DeclarationStatement(_, _)
            ));
        }

        #[test]
        fn recovers_after_invalid_print() {
            let input = "print; print 2;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(second.unwrap(), Statement::Print(_)));
        }

        #[test]
        fn recovers_after_invalid_print_with_literal() {
            let input = r#"print; print "hello";"#;
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(second.unwrap(), Statement::Print(_)));
        }

        #[test]
        fn recovers_multiple_errors_in_row() {
            // First error at '1 + ;' -> sync consumes '2 + ;' up to semicolon
            // Second parse sees 'var x = 3;' and succeeds
            let input = "1 + ; 2 + ; var x = 3;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            // Second parse succeeds because sync stopped at semicolon before 'var'
            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(
                second.unwrap(),
                Statement::DeclarationStatement(_, _)
            ));

            // Third parse hits EOF
            let third = parser.parse_statements();
            assert!(third.is_err());
        }

        #[test]
        fn synchronizes_at_semicolon() {
            let input = "1 + 2 ; var x = 3 ; print 42 ;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let mut count = 0;
            while let Ok(_) = parser.parse_statements() {
                count += 1;
            }
            assert_eq!(count, 3);
        }

        #[test]
        fn synchronizes_at_statement_keywords() {
            // After error, should sync at next var/print keyword
            let input = "1 + ; var x = 1; print 42; var y = 2;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(
                second.unwrap(),
                Statement::DeclarationStatement(_, _)
            ));

            let third = parser.parse_statements();
            assert!(third.is_ok());
            assert!(matches!(third.unwrap(), Statement::Print(_)));

            let fourth = parser.parse_statements();
            assert!(fourth.is_ok());
            assert!(matches!(
                fourth.unwrap(),
                Statement::DeclarationStatement(_, _)
            ));
        }

        #[test]
        fn does_not_synchronize_inside_expression() {
            // Errors in the middle of an expression trigger sync at next statement boundary
            // If next statement starts with expression (not keyword), sync may consume it
            // Use a keyword-started statement to ensure it survives sync
            let input = "1 + 2 * ; var x = 3;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let first = parser.parse_statements();
            assert!(first.is_err());

            // The next statement (starting with keyword) should survive sync
            let second = parser.parse_statements();
            assert!(second.is_ok());
            assert!(matches!(
                second.unwrap(),
                Statement::DeclarationStatement(_, _)
            ));
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn parses_deeply_nested_grouping() {
            let input = "((((42))))";
            parsed!(input, |expr| {
                let mut current = &expr;
                for _ in 0..4 {
                    current = as_grouping(current);
                }
                assert_eq!(as_number(current), 42);
            });
        }

        #[test]
        fn parses_deeply_nested_unary() {
            parsed!("- - - - 1", |expr| {
                let mut current = &expr;
                for _ in 0..4 {
                    let (op, inner) = as_unary(current);
                    assert_eq!(op, Minus);
                    current = inner;
                }
                assert_eq!(as_number(current), 1);
            });
        }

        #[test]
        fn parses_complex_precedence_chain() {
            parsed!("1 + 2 * 3 - 4 / 5 < 6 == 7 != 8 >= 9", |expr| {
                // Just verify it parses without error
                let (_, op, _) = as_binary(&expr);
                assert_eq!(op, BangEqual);
            });
        }

        #[test]
        fn parses_ternary_with_all_precedence_levels() {
            parsed!("1 + 2 * 3 < 4 ? 5 - 6 / 7 : 8 == 9", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                // Condition: 1 + 2 * 3 < 4
                let (_, cmp_op, _) = as_binary(cond);
                assert_eq!(cmp_op, Less);
                // Then branch: 5 - 6 / 7
                let (_, sub_op, _) = as_binary(then_branch);
                assert_eq!(sub_op, Minus);
                // Else branch: 8 == 9
                let (_, eq_op, _) = as_binary(else_branch);
                assert_eq!(eq_op, EqualEqual);
            });
        }

        #[test]
        fn parses_expression_with_only_unary_operators() {
            parsed!("!-!-!-1", |expr| {
                let mut current = &expr;
                let expected_ops = [Bang, Minus, Bang, Minus, Bang, Minus];
                for expected_op in expected_ops {
                    let (op, inner) = as_unary(current);
                    assert_eq!(op, expected_op);
                    current = inner;
                }
                assert_eq!(as_number(current), 1);
            });
        }

        #[test]
        fn parses_expression_statement_with_only_unary() {
            parsed_stmt!("!-1;", |stmt| {
                assert!(matches!(stmt, Statement::ExprStatement(_)));
            });
        }

        #[test]
        fn parses_var_declaration_with_complex_initializer() {
            parsed_stmt!("var x = (1 + 2) * (3 - 4) / 5;", |stmt| {
                assert!(matches!(stmt, Statement::DeclarationStatement(_, _)));
            });
        }

        #[test]
        fn parses_print_with_complex_expression() {
            parsed_stmt!("print (1 + 2) * (3 - 4) / 5;", |stmt| {
                assert!(matches!(stmt, Statement::Print(_)));
            });
        }

        #[test]
        fn parses_empty_string_in_declaration() {
            parsed_stmt!("var x = \"\";", |stmt| {
                assert!(matches!(stmt, Statement::DeclarationStatement(_, _)));
            });
        }

        #[test]
        fn parses_zero_as_number() {
            parsed!("0", |expr| {
                assert_eq!(as_number(&expr), 0);
            });
        }

        #[test]
        fn parses_large_integer() {
            parsed!("9999999999999999999", |expr| {
                assert_eq!(as_number(&expr), 9999999999999999999u64);
            });
        }

        #[test]
        fn parses_scientific_notation_like() {
            // Parser doesn't support scientific notation; parses as number followed by identifier
            // which then causes an error in expression parsing, but the number is parsed successfully
            parsed!("1e10", |expr| {
                assert_eq!(as_number(&expr), 1);
            });
        }

        #[test]
        fn rejects_keyword_as_identifier() {
            parse_expression_fails("var");
            parse_expression_fails("if");
            parse_expression_fails("while");
            parse_expression_fails("class");
        }

        #[test]
        fn rejects_keyword_in_declaration_name() {
            parse_statement_fails("var var = 1;");
            parse_statement_fails("var if = 1;");
            parse_statement_fails("var while = 1;");
        }

        #[test]
        fn parses_chained_comparison() {
            parsed!("1 < 2 < 3 < 4", |expr| {
                // Left associative: (((1 < 2) < 3) < 4)
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Less);
                assert_eq!(as_number(right), 4);
                let (_, op, right) = as_binary(left);
                assert_eq!(op, Less);
                assert_eq!(as_number(right), 3);
            });
        }

        #[test]
        fn parses_comma_in_grouping() {
            parsed!("(1, 2, 3)", |expr| {
                let inner = as_grouping(&expr);
                let (left, op, right) = as_binary(inner);
                assert_eq!(op, Comma);
                assert_eq!(as_number(right), 3);
                let (left, op, right) = as_binary(left);
                assert_eq!(op, Comma);
                assert_eq!(as_number(left), 1);
                assert_eq!(as_number(right), 2);
            });
        }

        #[test]
        fn parses_grouping_in_ternary_branches() {
            parsed!("true ? (1 + 2) : (3 * 4)", |expr| {
                let (_, then_branch, else_branch) = as_ternary(&expr);
                let then_inner = as_grouping(then_branch);
                let (_, add_op, _) = as_binary(then_inner);
                assert_eq!(add_op, Plus);

                let else_inner = as_grouping(else_branch);
                let (_, mul_op, _) = as_binary(else_inner);
                assert_eq!(mul_op, Star);
            });
        }

        #[test]
        fn handles_eof_gracefully() {
            let input = "";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_expression();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), RloxError::EOF));
        }

        #[test]
        fn parses_multiple_errors_with_recovery() {
            let input = "+ 1; * 2; / 3; 4;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            // Each should produce a recovered expression statement
            let mut count = 0;
            while let Ok(_) = parser.parse_statements() {
                count += 1;
            }
            assert_eq!(count, 4);
        }
    }

    mod error_recovery_quality {
        use super::*;

        #[test]
        fn recovered_binary_has_correct_structure() {
            let expr = parse_error_recovery("+ 1 * 2");
            let (left, op, right) = as_binary(&expr);
            assert_eq!(op, Plus);
            assert_eq!(as_operator(left), Plus);
            // Right should be fully parsed: 1 * 2
            let (r_left, r_op, r_right) = as_binary(right);
            assert_eq!(r_op, Star);
            assert_eq!(as_number(r_left), 1);
            assert_eq!(as_number(r_right), 2);
        }

        #[test]
        fn recovered_ternary_has_correct_structure() {
            // Parser doesn't recover leading '?' as ternary - it fails to parse condition
            // This test documents the current behavior
            parse_expression_fails("? 1 : 2");
        }

        #[test]
        fn recovered_comma_has_correct_structure() {
            let expr = parse_error_recovery(", 1, 2");
            let (left, op, right) = as_binary(&expr);
            assert_eq!(op, Comma);
            assert_eq!(as_operator(left), Comma);
            let (r_left, r_op, r_right) = as_binary(right);
            assert_eq!(r_op, Comma);
            assert_eq!(as_number(r_left), 1);
            assert_eq!(as_number(r_right), 2);
        }

        #[test]
        fn recovered_unary_is_not_missing_left() {
            let expr = parse_error_recovery("- 1");
            let (op, right) = as_unary(&expr);
            assert_eq!(op, Minus);
            assert_eq!(as_number(right), 1);
        }

        #[test]
        fn recovered_expression_in_statement_context() {
            let input = "+ 1;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_statements();
            assert!(result.is_ok());
            assert!(matches!(result.unwrap(), Statement::ExprStatement(_)));
        }
    }

    mod statement_error_messages {
        use super::*;

        #[test]
        fn error_on_missing_semicolon_includes_location() {
            let input = "var x = 1";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_statements();
            assert!(result.is_err());
            if let Err(ParserError(_, msg)) = result {
                assert!(msg.contains("semicolon"));
            } else {
                panic!("Expected ParserError");
            }
        }

        #[test]
        fn error_on_missing_initializer_after_equals() {
            // 'var x = ;' should fail - equals present but no expression
            let input = "var x = ;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_statements();
            assert!(result.is_err());
        }

        #[test]
        fn error_on_missing_equals_in_declaration() {
            let input = "var x 1;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_statements();
            assert!(result.is_err());
        }

        #[test]
        fn error_on_missing_condition_in_ternary() {
            let input = "? 1 : 2";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_expression();
            assert!(result.is_err());
        }

        #[test]
        fn error_on_missing_colon_in_ternary() {
            let input = "1 ? 2";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_expression();
            assert!(result.is_err());
        }

        #[test]
        fn error_on_missing_else_in_ternary() {
            let input = "1 ? 2 :";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_expression();
            assert!(result.is_err());
        }
    }

    mod assignment {
        use super::*;

        pub(crate) fn as_assignment(expr: &Expr) -> (&Token, &Expr) {
            match expr {
                AssignmentExpression { left, right } => (&left.token, right),
                other => panic!("expected assignment expression, got {other:?}"),
            }
        }

        #[test]
        fn parses_simple_assignment() {
            parsed!("x = 1", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                assert_eq!(as_number(right), 1);
            });
        }

        #[test]
        fn parses_assignment_with_expression() {
            parsed!("x = 1 + 2", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                let (_, op, _) = as_binary(right);
                assert_eq!(op, Plus);
            });
        }

        #[test]
        fn parses_assignment_with_ternary() {
            parsed!("x = true ? 1 : 2", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                let (cond, then_branch, else_branch) = as_ternary(right);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn parses_chained_assignment() {
            parsed!("x = y = 1", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                let (inner_left, inner_right) = as_assignment(right);
                assert_eq!(inner_left.lexeme, "y");
                assert_eq!(as_number(inner_right), 1);
            });
        }

        #[test]
        fn parses_chained_assignment_right_associative() {
            // x = y = 1 should parse as x = (y = 1)
            parsed!("x = y = 1", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                let (inner_left, inner_right) = as_assignment(right);
                assert_eq!(inner_left.lexeme, "y");
                assert_eq!(as_number(inner_right), 1);
            });
        }

        #[test]
        fn parses_assignment_in_grouping() {
            parsed!("(x = 1)", |expr| {
                let inner = as_grouping(&expr);
                let (left, right) = as_assignment(inner);
                assert_eq!(left.lexeme, "x");
                assert_eq!(as_number(right), 1);
            });
        }

        #[test]
        fn parses_assignment_as_expression_statement() {
            let input = "x = 1;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);
            let result = parser.parse_statements();
            assert!(result.is_ok());
            if let Statement::ExprStatement(expr) = result.unwrap() {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                assert_eq!(as_number(right), 1);
            } else {
                panic!("expected ExprStatement");
            }
        }

        #[test]
        fn parses_multiple_assignments_in_statement() {
            let input = "x = 1; y = 2;";
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new(scanner);

            let stmt1 = parser.parse_statements().unwrap();
            if let Statement::ExprStatement(expr) = stmt1 {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                assert_eq!(as_number(right), 1);
            }

            let stmt2 = parser.parse_statements().unwrap();
            if let Statement::ExprStatement(expr) = stmt2 {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "y");
                assert_eq!(as_number(right), 2);
            }
        }

        #[test]
        fn assignment_has_lower_precedence_than_comma() {
            // a = 1, 2 should parse as (a = 1), 2
            parsed!("a = 1, 2", |expr| {
                let (left, op, right) = as_binary(&expr);
                assert_eq!(op, Comma);
                assert_eq!(as_number(right), 2);
                let (assign_left, assign_right) = as_assignment(left);
                assert_eq!(assign_left.lexeme, "a");
                assert_eq!(as_number(assign_right), 1);
            });
        }

        #[test]
        fn assignment_has_lower_precedence_than_ternary() {
            // a = true ? 1 : 2 should parse as a = (true ? 1 : 2)
            parsed!("a = true ? 1 : 2", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "a");
                let (cond, then_branch, else_branch) = as_ternary(right);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                assert_eq!(as_number(then_branch), 1);
                assert_eq!(as_number(else_branch), 2);
            });
        }

        #[test]
        fn rejects_assignment_to_non_identifier() {
            parse_expression_fails("1 = 2");
            parse_expression_fails("true = false");
            parse_expression_fails("nil = 1");
            parse_expression_fails("\"str\" = 1");
        }

        #[test]
        fn rejects_assignment_to_grouping() {
            parse_expression_fails("(x) = 1");
        }

        #[test]
        fn rejects_assignment_to_binary() {
            parse_expression_fails("x + y = 1");
        }

        #[test]
        fn rejects_assignment_to_unary() {
            parse_expression_fails("-x = 1");
        }

        #[test]
        fn parses_complex_assignment_chain() {
            // a = b = c = 1
            parsed!("a = b = c = 1", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "a");
                let (left, right) = as_assignment(right);
                assert_eq!(left.lexeme, "b");
                let (left, right) = as_assignment(right);
                assert_eq!(left.lexeme, "c");
                assert_eq!(as_number(right), 1);
            });
        }

        #[test]
        fn parses_assignment_with_complex_rhs() {
            // x = (1 + 2) * 3
            parsed!("x = (1 + 2) * 3", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                let (inner_left, op, _) = as_binary(right);
                assert_eq!(op, Star);
                let grouped = as_grouping(inner_left);
                let (_, add_op, _) = as_binary(grouped);
                assert_eq!(add_op, Plus);
            });
        }

        #[test]
        fn parses_assignment_in_ternary_then_branch() {
            parsed!("true ? x = 1 : y = 2", |expr| {
                let (cond, then_branch, else_branch) = as_ternary(&expr);
                assert!(matches!(cond, Expr::SyntaxLiteral(Literal::True)));
                let (left, right) = as_assignment(then_branch);
                assert_eq!(left.lexeme, "x");
                assert_eq!(as_number(right), 1);
                let (left, right) = as_assignment(else_branch);
                assert_eq!(left.lexeme, "y");
                assert_eq!(as_number(right), 2);
            });
        }

        #[test]
        fn parses_assignment_in_ternary_else_branch() {
            parsed!("false ? 1 : x = 2", |expr| {
                let (_, _, else_branch) = as_ternary(&expr);
                let (left, right) = as_assignment(else_branch);
                assert_eq!(left.lexeme, "x");
                assert_eq!(as_number(right), 2);
            });
        }

        #[test]
        fn parses_assignment_with_unary_rhs() {
            parsed!("x = -1", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                let (unary_op, inner) = as_unary(right);
                assert_eq!(unary_op, Minus);
                assert_eq!(as_number(inner), 1);
            });
        }

        #[test]
        fn parses_assignment_with_string() {
            parsed!(r#"x = "hello""#, |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                assert_eq!(as_string(right), "hello");
            });
        }

        #[test]
        fn parses_assignment_with_nil() {
            parsed!("x = nil", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "x");
                assert!(matches!(right, Expr::SyntaxLiteral(Literal::Nil)));
            });
        }

        #[test]
        fn assignment_expression_returns_rhs_value() {
            // Assignment is an expression that evaluates to the RHS value
            parsed!("x = 1", |expr| {
                let (_, right) = as_assignment(&expr);
                assert_eq!(as_number(right), 1);
            });
        }

        #[test]
        fn assignment_is_right_associative_in_complex_expr() {
            // a = b = 1 + 2 should be a = (b = (1 + 2))
            parsed!("a = b = 1 + 2", |expr| {
                let (left, right) = as_assignment(&expr);
                assert_eq!(left.lexeme, "a");
                let (left, right) = as_assignment(right);
                assert_eq!(left.lexeme, "b");
                let (_, op, _) = as_binary(right);
                assert_eq!(op, Plus);
            });
        }
    }

    mod parser_modes {
        use super::*;

        fn parse_statement_with_mode(
            input: &str,
            mode: ParserMode,
        ) -> Result<Statement, RloxError> {
            let scanner = Scanner::new(input.lines());
            let mut parser = Parser::new_with_mode(scanner, mode);
            parser.parse_statements()
        }

        #[test]
        fn repl_accepts_expression_terminated_by_eof() {
            let result = parse_statement_with_mode("1 + 2", ParserMode::Repl);

            assert!(matches!(result, Ok(Statement::ExprStatement(_))));
        }

        #[test]
        fn script_rejects_expression_without_semicolon() {
            let result = parse_statement_with_mode("1 + 2", ParserMode::Script);

            assert!(result.is_err());
        }

        #[test]
        fn script_accepts_expression_with_semicolon() {
            let result = parse_statement_with_mode("1 + 2;", ParserMode::Script);

            assert!(matches!(result, Ok(Statement::ExprStatement(_))));
        }

        #[test]
        fn repl_accepts_declaration_terminated_by_eof() {
            let result = parse_statement_with_mode("var x = 1", ParserMode::Repl);

            assert!(matches!(
                result,
                Ok(Statement::DeclarationStatement(name, Some(_))) if name.lexeme == "x"
            ));
        }

        #[test]
        fn script_rejects_declaration_without_semicolon() {
            let result = parse_statement_with_mode("var x = 1", ParserMode::Script);

            assert!(result.is_err());
        }

        #[test]
        fn repl_rejects_incomplete_expression_at_eof() {
            let result = parse_statement_with_mode("1 +", ParserMode::Repl);

            assert!(result.is_err());
        }
    }

    mod functions {
        use super::*;

        fn callable(expr: &Expr) -> (&Expr, &Token, &[Expr]) {
            let Expr::CallableExpression {
                callee,
                paren,
                args,
            } = expr
            else {
                panic!("expected call expression, got {expr:?}");
            };
            (callee, paren, args)
        }

        fn identifier(expr: &Expr) -> &str {
            let Expr::IdentifierExpression(ident) = expr else {
                panic!("expected identifier expression, got {expr:?}");
            };
            &ident.token.lexeme
        }

        fn function_expression(expr: &Expr) -> (&[Token], &Block) {
            let Expr::FunctionExpression { args, body } = expr else {
                panic!("expected function expression, got {expr:?}");
            };
            (args, body)
        }

        fn param_names(args: &[Token]) -> Vec<&str> {
            args.iter().map(|arg| arg.lexeme.as_str()).collect()
        }

        mod declarations {
            use super::*;

            #[test]
            fn parses_name_parameters_and_body() {
                let statement = parse_statement_ok("fun add(left, right) { return left + right; }");
                let Statement::FunctionStatement { name, args, body } = statement else {
                    panic!("expected function declaration");
                };

                assert_eq!(name.token.lexeme, "add");
                assert_eq!(
                    args.iter()
                        .map(|arg| arg.lexeme.as_str())
                        .collect::<Vec<_>>(),
                    ["left", "right"]
                );
                assert!(matches!(
                    body.statements.as_slice(),
                    [ReturnStatement(_, Some(Expr::Binary { .. }))]
                ));
            }

            #[test]
            fn parses_empty_body() {
                let statement = parse_statement_ok("fun noop(unused) {}");
                let Statement::FunctionStatement { body, .. } = statement else {
                    panic!("expected function declaration");
                };

                assert!(body.statements.is_empty());
            }

            #[test]
            fn rejects_duplicate_parameter_names() {
                parse_statement_fails("fun pick(value, value) { return value; }");
            }

            #[test]
            fn accepts_255_parameters() {
                let params = (0..255)
                    .map(|index| format!("p{index}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let statement = parse_statement_ok(&format!("fun many({params}) {{}}"));
                let Statement::FunctionStatement { args, .. } = statement else {
                    panic!("expected function declaration");
                };

                assert_eq!(args.len(), 255);
            }

            #[test]
            fn rejects_256_parameters() {
                let params = (0..256)
                    .map(|index| format!("p{index}"))
                    .collect::<Vec<_>>()
                    .join(",");

                parse_statement_fails(&format!("fun many({params}) {{}}"));
            }

            #[test]
            fn accepts_zero_parameter_declaration() {
                let statement = parse_statement_ok("fun clockwork() {}");
                let Statement::FunctionStatement { args, .. } = statement else {
                    panic!("expected function declaration");
                };

                assert!(args.is_empty());
            }

            #[test]
            fn rejects_missing_left_parenthesis() {
                parse_statement_fails("fun identity value) { return value; }");
            }

            #[test]
            fn rejects_missing_parameter_separator() {
                parse_statement_fails("fun add(left right) { return left + right; }");
            }

            #[test]
            fn rejects_non_identifier_parameter() {
                parse_statement_fails("fun bad(123) {}");
            }

            #[test]
            fn rejects_trailing_parameter_comma() {
                parse_statement_fails("fun identity(value,) { return value; }");
            }

            #[test]
            fn rejects_missing_right_parenthesis() {
                parse_statement_fails("fun identity(value { return value; }");
            }

            #[test]
            fn rejects_missing_body() {
                parse_statement_fails("fun identity(value) return value;");
            }

            #[test]
            fn rejects_unterminated_body() {
                parse_statement_fails("fun identity(value) { return value;");
            }
        }

        mod expressions {
            use super::*;

            fn parse_script(input: &str) -> Result<Vec<Statement>, RloxError> {
                let mut parser = Parser::new(Scanner::new(input.lines()));
                let mut statements = Vec::new();
                loop {
                    match parser.parse_statements() {
                        Ok(statement) => statements.push(statement),
                        Err(RloxError::EOF) => return Ok(statements),
                        Err(error) => return Err(error),
                    }
                }
            }

            #[test]
            fn parses_parameters_and_body() {
                let expr = parse_ok("fun (left, right) { return left + right; }");
                let (args, body) = function_expression(&expr);

                assert_eq!(param_names(args), ["left", "right"]);
                assert!(matches!(
                    body.statements.as_slice(),
                    [ReturnStatement(_, Some(Expr::Binary { .. }))]
                ));
            }

            #[test]
            fn parses_zero_parameter_expression() {
                let expr = parse_ok("fun () { return 1; }");
                let (args, body) = function_expression(&expr);

                assert!(args.is_empty());
                assert!(matches!(
                    body.statements.as_slice(),
                    [ReturnStatement(_, Some(_))]
                ));
            }

            #[test]
            fn anonymous_function_statement_is_an_expression_statement() {
                let statement = parse_statement_ok("fun (value) { return value; };");
                let Statement::ExprStatement(expr) = statement else {
                    panic!("expected expression statement, got {statement:?}");
                };
                let (args, _) = function_expression(&expr);

                assert_eq!(param_names(args), ["value"]);
            }

            #[test]
            fn discarded_anonymous_function_is_the_book_statement_case() {
                let statement = parse_statement_ok("fun () {};");

                assert!(matches!(
                    statement,
                    Statement::ExprStatement(Expr::FunctionExpression { ref args, ref body })
                        if args.is_empty() && body.statements.is_empty()
                ));
            }

            #[test]
            fn named_function_is_still_a_declaration() {
                let statement = parse_statement_ok("fun named() {}");

                assert!(matches!(statement, Statement::FunctionStatement { .. }));
            }

            #[test]
            fn script_requires_semicolon_after_anonymous_function_statement() {
                parse_statement_fails("fun () {}");
            }

            #[test]
            fn parses_function_expression_in_variable_initializer() {
                let statement = parse_statement_ok("var f = fun (n) { return n; };");
                let Statement::DeclarationStatement(name, Some(expr)) = statement else {
                    panic!("expected declaration with initializer, got {statement:?}");
                };

                assert_eq!(name.lexeme, "f");
                assert_eq!(param_names(function_expression(&expr).0), ["n"]);
            }

            #[test]
            fn parses_function_expression_as_call_argument() {
                let expr = parse_ok("thrice(fun (a) { print a; })");
                let (callee, _, args) = callable(&expr);

                assert_eq!(identifier(callee), "thrice");
                assert_eq!(args.len(), 1);
                assert_eq!(param_names(function_expression(&args[0]).0), ["a"]);
            }

            #[test]
            fn parses_grouped_iife() {
                let expr = parse_ok("(fun () { print \"fun!\"; })()");
                let (callee, _, args) = callable(&expr);
                let grouped = as_grouping(callee);
                let (params, body) = function_expression(grouped);

                assert!(params.is_empty());
                assert!(args.is_empty());
                assert!(matches!(body.statements.as_slice(), [Statement::Print(_)]));
            }

            #[test]
            fn parses_ungrouped_iife_expression() {
                let expr = parse_ok("fun () { return 1; }()");
                let (callee, _, args) = callable(&expr);
                let (params, _) = function_expression(callee);

                assert!(params.is_empty());
                assert!(args.is_empty());
            }

            #[test]
            fn parses_iife_with_arguments() {
                let expr = parse_ok("(fun (a, b) { return a + b; })(20, 22)");
                let (callee, _, args) = callable(&expr);

                assert_eq!(
                    param_names(function_expression(as_grouping(callee)).0),
                    ["a", "b"]
                );
                assert_eq!(args.len(), 2);
                assert_eq!(as_number(&args[0]), 20);
                assert_eq!(as_number(&args[1]), 22);
            }

            #[test]
            fn call_binds_tighter_than_arithmetic() {
                let expr = parse_ok("fun () { return 1; }() + 2");
                let (left, operator, right) = as_binary(&expr);
                let (callee, _, _) = callable(left);

                assert_eq!(operator, Plus);
                assert!(matches!(callee, Expr::FunctionExpression { .. }));
                assert_eq!(as_number(right), 2);
            }

            #[test]
            fn parses_nested_function_expression() {
                let expr = parse_ok("fun () { return fun (x) { return x; }; }");
                let (_, body) = function_expression(&expr);
                let [ReturnStatement(_, Some(inner))] = body.statements.as_slice() else {
                    panic!("expected return of inner function, got {body:?}");
                };

                assert_eq!(param_names(function_expression(inner).0), ["x"]);
            }

            #[test]
            fn return_is_allowed_inside_anonymous_body() {
                let statement = parse_statement_ok("fun () { return 1; };");
                let Statement::ExprStatement(expr) = statement else {
                    panic!("expected expression statement, got {statement:?}");
                };
                let (_, body) = function_expression(&expr);

                assert!(matches!(
                    body.statements.as_slice(),
                    [ReturnStatement(_, Some(_))]
                ));
            }

            #[test]
            fn top_level_return_is_still_rejected_after_anonymous_function() {
                let result = parse_script("fun () { return 1; };\nreturn 1;");

                assert!(
                    result.is_err(),
                    "top-level return after a fun expression should still fail, got {result:?}"
                );
            }

            #[test]
            fn rejects_named_function_expression() {
                parse_expression_fails("fun identity() {}");
            }

            #[test]
            fn rejects_missing_parentheses() {
                parse_statement_fails("fun {};");
            }

            #[test]
            fn rejects_assignment_to_function_expression() {
                parse_expression_fails("fun () {} = 1");
            }

            #[test]
            fn rejects_trailing_parameter_comma() {
                parse_expression_fails("fun (value,) {}");
            }

            #[test]
            fn rejects_missing_body() {
                parse_expression_fails("fun (value) return value;");
            }

            #[test]
            fn accepts_255_parameters() {
                let params = (0..255)
                    .map(|index| format!("p{index}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let expr = parse_ok(&format!("fun ({params}) {{}}"));

                assert_eq!(function_expression(&expr).0.len(), 255);
            }

            #[test]
            fn rejects_256_parameters() {
                let params = (0..256)
                    .map(|index| format!("p{index}"))
                    .collect::<Vec<_>>()
                    .join(",");

                parse_expression_fails(&format!("fun ({params}) {{}}"));
            }
        }

        mod calls {
            use super::*;

            #[test]
            fn parses_zero_argument_call() {
                let expr = parse_ok("clock()");
                let (callee, paren, args) = callable(&expr);

                assert_eq!(identifier(callee), "clock");
                assert_eq!(paren.token_type, RightParen);
                assert!(args.is_empty());
            }

            #[test]
            fn parses_multiple_arguments_in_source_order() {
                let expr = parse_ok("combine(1, 2 + 3, \"last\")");
                let (callee, _, args) = callable(&expr);

                assert_eq!(identifier(callee), "combine");
                assert_eq!(args.len(), 3);
                assert_eq!(as_number(&args[0]), 1);
                assert!(matches!(args[1], Expr::Binary { .. }));
                assert_eq!(as_string(&args[2]), "last");
            }

            #[test]
            fn call_binds_tighter_than_arithmetic() {
                let expr = parse_ok("identity(2) * 3");
                let (left, operator, right) = as_binary(&expr);

                assert_eq!(operator, Star);
                assert_eq!(identifier(callable(left).0), "identity");
                assert_eq!(as_number(right), 3);
            }

            #[test]
            fn grouped_expression_can_be_called() {
                let expr = parse_ok("(identity)(42)");
                let (callee, _, args) = callable(&expr);

                assert!(matches!(callee, Expr::Grouping { .. }));
                assert_eq!(as_number(&args[0]), 42);
            }

            #[test]
            fn call_can_be_the_final_argument_of_another_call() {
                let expr = parse_ok("outer(1, inner(2))");
                let (_, _, outer_args) = callable(&expr);
                let (inner, _, inner_args) = callable(&outer_args[1]);

                assert_eq!(identifier(inner), "inner");
                assert_eq!(as_number(&inner_args[0]), 2);
            }

            #[test]
            fn rejects_trailing_argument_comma() {
                parse_statement_fails("identity(42,);");
            }

            #[test]
            fn accepts_255_arguments() {
                let arguments = std::iter::repeat_n("1", 255).collect::<Vec<_>>().join(",");
                let expr = parse_ok(&format!("many({arguments})"));

                assert_eq!(callable(&expr).2.len(), 255);
            }

            #[test]
            fn rejects_256_arguments() {
                let arguments = std::iter::repeat_n("1", 256).collect::<Vec<_>>().join(",");

                parse_statement_fails(&format!("many({arguments});"));
            }

            #[test]
            fn rejects_missing_right_parenthesis() {
                parse_statement_fails("identity(42;");
            }

            #[test]
            fn rejects_missing_argument_before_comma() {
                parse_statement_fails("identity(,42);");
            }

            #[test]
            fn parses_chained_invocation() {
                let expr = parse_ok("factory(1)(2)");
                let (first_call, _, second_args) = callable(&expr);
                let (factory, _, first_args) = callable(first_call);

                assert_eq!(identifier(factory), "factory");
                assert_eq!(as_number(&first_args[0]), 1);
                assert_eq!(as_number(&second_args[0]), 2);
            }

            #[test]
            fn nested_call_can_precede_another_argument() {
                let expr = parse_ok("outer(inner(1), 2)");
                let (outer, _, outer_args) = callable(&expr);
                let (inner, _, inner_args) = callable(&outer_args[0]);

                assert_eq!(identifier(outer), "outer");
                assert_eq!(outer_args.len(), 2);
                assert_eq!(identifier(inner), "inner");
                assert_eq!(as_number(&inner_args[0]), 1);
                assert_eq!(as_number(&outer_args[1]), 2);
            }
        }

        mod returns {
            use super::*;

            #[test]
            fn parses_return_value_and_keyword_token() {
                let statement = parse_statement_ok("fun sum(unused) { return 1 + 2; }");
                let Statement::FunctionStatement { body, .. } = statement else {
                    panic!("expected function declaration");
                };
                let [ReturnStatement(token, Some(expr))] = body.statements.as_slice() else {
                    panic!("expected return statement with a value");
                };

                assert_eq!(token.token_type, Return);
                assert_eq!(token.lexeme, "return");
                assert!(matches!(expr, Expr::Binary { .. }));
            }

            #[test]
            fn parses_return_nested_in_function_control_flow() {
                let statement =
                    parse_statement_ok("fun choose(value) { if (value) return 1; else return 2; }");
                let Statement::FunctionStatement { body, .. } = statement else {
                    panic!("expected function declaration");
                };

                assert!(matches!(
                    body.statements.as_slice(),
                    [Statement::IfStatement {
                        then,
                        else_stmnt: Some(_),
                        ..
                    }] if matches!(then.as_ref(), Statement::ReturnStatement(_, Some(_)))
                ));
            }

            #[test]
            fn rejects_top_level_return() {
                parse_statement_fails("return 42;");
            }

            #[test]
            fn parses_bare_return_inside_function() {
                let statement = parse_statement_ok("fun noop(unused) { return; }");
                let Statement::FunctionStatement { body, .. } = statement else {
                    panic!("expected function declaration");
                };

                assert!(matches!(
                    body.statements.as_slice(),
                    [Statement::ReturnStatement(_, None)]
                ));
            }

            #[test]
            fn rejects_return_without_semicolon() {
                parse_statement_fails("return 42");
            }

            #[test]
            fn rejects_return_with_missing_expression() {
                parse_statement_fails("return +;");
            }
        }
    }
}
