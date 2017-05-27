//! # Language
//!
//! * Labels define a section of instruction, and must consist of `[A-Za-z0-9_]`
//!   charaters and end with `:`
//! * Single line comments are defined with `;`
//! * Numbers can be written "as is".
//! * Symbols (hashed strings used for comparison) start with `$`
//! * Keywords are any unmatched text containing only `[A-Za-z0-9_]` characters
//! * Referencing a variable is done by prefixing the variable name with `@`
//! * Assignging to a variable is done by prefixing the variable name with `=`
//! * Lists are tokens wrapped with `[` `]`
//! * Null is represented with `~`

use regex::Regex;

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::Hasher;
use std::ops::Range;
use std::error::Error;


/// Representation of the different token types
#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    Label,
    Comment,
    Number,
    Symbol,
    Keyword,
    LoadVar,
    StoreVar,
    StringLiteral,
    ListBegin,
    ListEnd,
    Null,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token<'a> {
    /// The type of token that `val` represents
    pub tag: Tag,
    /// The line the token appears at
    pub line: usize,
    /// The column the token appears at
    pub col: usize,
    /// The value string of the token
    pub val: &'a str,
}

/// Instructions define a series of operations that a unit should perform
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Instr {
    LoadNumber(f32),
    LoadSymbol(u64),
    LoadVar(u64),
    StoreVar(u64),
    LoadString(u64),
    Keyword(u64),
    ListBegin,
    ListEnd,
    Null,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    labels: Vec<u64>,
    sections: Vec<Range<usize>>,
    instrs: Vec<Instr>,
}

impl Program {
    /// Construct a new program from a string
    pub fn new(text: &str) -> Result<Program, ParseErr> {
        parse(text)
    }

    /// Return instructions corresponding to a label in the program
    pub fn section(&self, label: &str) -> Option<&[Instr]> {
        let label = hash_str(label);
        match self.labels.iter().position(|&l| l == label) {
            None => None,
            Some(idx) => {
                let sec = &self.sections[idx];
                Some(&self.instrs[sec.start..sec.end])
            }
        }

    }
}

#[derive(Debug)]
pub enum ParseErr<'a> {
    UnknownToken(Token<'a>),
    UnmatchedPair(Token<'a>),
    UnknownVariable(Token<'a>),
}

impl<'a> fmt::Display for ParseErr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseErr::UnknownToken(ref token) => {
                write!(f,
                       "Encounteded unknown token \"{}\" at line {} col {}",
                       token.val,
                       token.line,
                       token.col)
            }
            ParseErr::UnmatchedPair(ref token) => {
                write!(f,
                       "Missing closing token at line {} col {}",
                       token.line,
                       token.col)
            }
            ParseErr::UnknownVariable(ref token) => {
                write!(f,
                       "Unknown variable \"{}\" at line {} col {}",
                       token.val,
                       token.line,
                       token.col)
            }
        }
    }
}

impl<'a> Error for ParseErr<'a> {
    fn description(&self) -> &str {
        match *self {
            ParseErr::UnknownToken(_) => "unknown token",
            ParseErr::UnmatchedPair(_) => "unmatched pair",
            ParseErr::UnknownVariable(_) => "unknown variable",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

/// Maintain current tokenizer position in the input string
struct TokState {
    line: usize,
    col: usize,
}

impl TokState {
    pub fn scan_char(&mut self, text: char) -> bool {
        if text.is_whitespace() {
            if text == '\n' {
                self.line += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
            return true;
        }
        false

    }

    pub fn scan_str(&mut self, text: &str) {
        for ch in text.chars() {
            if !self.scan_char(ch) {
                self.col += 1;
            }
        }
    }
}

/// Maps a regular expression to a token type
struct TagExpr {
    tag: Tag,
    re: Regex,
}

/// Split a string into a series of tokens according to the language rules
fn tokenize(text: &str) -> Result<Vec<Token>, ParseErr> {
    let types = [TagExpr {
                     tag: Tag::Comment,
                     re: Regex::new("^(?:;)(.*?)(?:\n)").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::Label,
                     re: Regex::new(r"^(\w+)(?::)").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::Number,
                     re: Regex::new(r"^(-?[0-9]\d*(\.\d+)?)").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::ListBegin,
                     re: Regex::new(r"^(\[)").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::ListEnd,
                     re: Regex::new(r"^(\])").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::Null,
                     re: Regex::new(r"^(~)").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::Symbol,
                     re: Regex::new(r"^(?:\$)(\w+)").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::LoadVar,
                     re: Regex::new(r"^(?:@)(\w+)").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::StoreVar,
                     re: Regex::new(r"^(?:=)(\w+)").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::StringLiteral,
                     re: Regex::new("^(\"((.|\n)*?)\")").unwrap(),
                 },
                 TagExpr {
                     tag: Tag::Keyword,
                     re: Regex::new(r"^(\w+)").unwrap(),
                 }];

    let mut it = text.char_indices();
    let mut state = TokState { line: 0, col: 0 };
    let mut tokens = Vec::new();
    let end = text.len();

    while let Some((offset, ch)) = it.next() {
        // Advance over whitespace
        if state.scan_char(ch) {
            continue;
        }

        // Match the next word
        let word = &text[offset..end];
        let mut handled = false;
        for expr in &types {
            match expr.re.captures(word) {
                None => (),
                Some(caps) => {
                    handled = true;

                    let mat = caps.get(1).unwrap();
                    let start = mat.start();
                    let end = mat.end();
                    let val = &word[start..end];
                    tokens.push(Token {
                                    tag: expr.tag.clone(),
                                    line: state.line,
                                    col: state.col,
                                    val: val,
                                });

                    // Advance over the full match rather than the captured
                    // value. Iterator is already at the first character
                    let orig = caps.get(0).unwrap();
                    let start = orig.start();
                    let end = orig.end();
                    for _ in 0..end - start - 1 {
                        it.next();
                    }
                    state.scan_str(&word[start..end]);
                    break;
                }
            }
        }

        if !handled {
            let token = Token {
                tag: Tag::Keyword,
                line: state.line,
                col: state.col,
                val: word,
            };
            return Err(ParseErr::UnknownToken(token));
        }
    }

    Ok(tokens)
}

// Validate 'List' tokens are balenced
fn validate_lists<'a>(tokens: &[Token<'a>]) -> Result<(), ParseErr<'a>> {
    let mut stack: Vec<Token> = vec![];

    for tok in tokens {
        match tok.tag {
            Tag::ListBegin => stack.push(tok.clone()),
            Tag::ListEnd => {
                stack.pop();
            }
            _ => (),
        }
    }

    match stack.first() {
        Some(tok) => Err(ParseErr::UnmatchedPair(tok.clone())),
        None => Ok(()),
    }
}

// Validate references to variables
fn validate_vars<'a>(tokens: &[Token<'a>]) -> Result<(), ParseErr<'a>> {
    let mut vars = vec![];

    for tok in tokens {
        match tok.tag {
            Tag::StoreVar => vars.push(tok.val),
            Tag::LoadVar => {
                if !vars.contains(&tok.val) {
                    return Err(ParseErr::UnknownVariable(tok.clone()));
                }
            }
            _ => (),
        }
    }

    Ok(())
}

pub fn hash_str(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

/// Parse a program into a list of instructions
fn parse(text: &str) -> Result<Program, ParseErr> {
    let tokens = try!(tokenize(text));

    let validators = [validate_lists, validate_vars];
    for validator in &validators {
        if let Err(err) = validator(&tokens) {
            return Err(err);
        };
    }

    let mut program = Program {
        instrs: vec![],
        labels: vec![],
        sections: vec![],
    };

    for token in &tokens {
        match token.tag {
            Tag::Label => {
                match program.sections.last_mut() {
                    None => (),
                    Some(sec) => {
                        sec.end = program.instrs.len();
                    }
                }
                let len = program.instrs.len();
                let next = Range { start: len, end: 0 };
                program.labels.push(hash_str(token.val));
                program.sections.push(next);
            }
            Tag::Symbol => {
                let sym = hash_str(token.val);
                program.instrs.push(Instr::LoadSymbol(sym));
            }
            Tag::Number => {
                let num = token.val.parse::<f32>().unwrap();
                program.instrs.push(Instr::LoadNumber(num));
            }
            Tag::Keyword => {
                let sym = hash_str(token.val);
                program.instrs.push(Instr::Keyword(sym));
            }
            Tag::LoadVar => {
                let sym = hash_str(token.val);
                program.instrs.push(Instr::LoadVar(sym));
            }
            Tag::StoreVar => {
                let sym = hash_str(token.val);
                program.instrs.push(Instr::StoreVar(sym));
            }
            Tag::ListBegin => {
                program.instrs.push(Instr::ListBegin);
            }
            Tag::ListEnd => {
                program.instrs.push(Instr::ListEnd);
            }
            Tag::Null => {
                program.instrs.push(Instr::Null);
            }
            Tag::StringLiteral => {
                let sym = hash_str(token.val);
                program.instrs.push(Instr::LoadString(sym));
            }
            Tag::Comment => (),
        }
    }

    match program.sections.last_mut() {
        None => (),
        Some(sec) => {
            sec.end = program.instrs.len();
        }
    }

    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_singlechar() {
        let tokens = tokenize("1").unwrap();
        assert_eq!(tokens,
                   vec![Token {
                            tag: Tag::Number,
                            line: 0,
                            col: 0,
                            val: "1",
                        }]);
    }

    #[test]
    fn tokenize_line_and_col_nos() {
        let tokens = tokenize("  1  -2  \n  3 ").unwrap();
        assert_eq!(tokens,
                   vec![Token {
                            tag: Tag::Number,
                            line: 0,
                            col: 2,
                            val: "1",
                        },
                        Token {
                            tag: Tag::Number,
                            line: 0,
                            col: 5,
                            val: "-2",
                        },
                        Token {
                            tag: Tag::Number,
                            line: 1,
                            col: 2,
                            val: "3",
                        }]);
    }

    #[test]
    fn tokenize_comments() {
        let tokens = tokenize("-12 ; 30\n200").unwrap();
        assert_eq!(tokens,
                   vec![Token {
                            tag: Tag::Number,
                            line: 0,
                            col: 0,
                            val: "-12",
                        },
                        Token {
                            tag: Tag::Comment,
                            line: 0,
                            col: 4,
                            val: " 30",
                        },
                        Token {
                            tag: Tag::Number,
                            line: 1,
                            col: 0,
                            val: "200",
                        }]);
    }

    #[test]
    fn tokenize_numbers() {
        let instrs = tokenize("-12 2.4 1 -200.12").unwrap();
        assert_eq!(instrs,
                   vec![Token {
                            tag: Tag::Number,
                            line: 0,
                            col: 0,
                            val: "-12",
                        },
                        Token {
                            tag: Tag::Number,
                            line: 0,
                            col: 4,
                            val: "2.4",
                        },
                        Token {
                            tag: Tag::Number,
                            line: 0,
                            col: 8,
                            val: "1",
                        },
                        Token {
                            tag: Tag::Number,
                            line: 0,
                            col: 10,
                            val: "-200.12",
                        }]);
    }

    #[test]
    fn parse_symbol() {
        let instrs = tokenize("$foo").unwrap();
        assert_eq!(instrs,
                   vec![Token {
                            tag: Tag::Symbol,
                            line: 0,
                            col: 0,
                            val: "foo",
                        }]);
    }

    #[test]
    fn parse_variable() {
        let instrs = tokenize("=foo @foo").unwrap();
        assert_eq!(instrs,
                   vec![Token {
                            tag: Tag::StoreVar,
                            line: 0,
                            col: 0,
                            val: "foo",
                        },
                        Token {
                            tag: Tag::LoadVar,
                            line: 0,
                            col: 5,
                            val: "foo",
                        }]);
    }

    #[test]
    fn parse_errors() {
        let err = parse("\n ?");
        assert!(err.is_err(),
                ParseErr::UnknownToken(Token {
                                           tag: Tag::Keyword,
                                           line: 1,
                                           col: 1,
                                           val: "?",
                                       }));
    }

    #[test]
    fn parse_keyword() {
        let instrs = tokenize("buffer").unwrap();
        assert_eq!(instrs,
                   vec![Token {
                            tag: Tag::Keyword,
                            line: 0,
                            col: 0,
                            val: "buffer",
                        }]);
    }

    #[test]
    fn parse_labels() {
        let prog = parse("
        draw:
            0 2 square
        audio:
            0 channel 0.5 gain
        ")
                .unwrap();
        let draw = hash_str("draw");
        let audio = hash_str("audio");
        assert_eq!(prog.labels, vec![draw, audio]);
        assert_eq!(prog.instrs,
                   vec![Instr::LoadNumber(0f32),
                        Instr::LoadNumber(2f32),
                        Instr::Keyword(hash_str("square")),
                        Instr::LoadNumber(0f32),
                        Instr::Keyword(hash_str("channel")),
                        Instr::LoadNumber(0.5),
                        Instr::Keyword(hash_str("gain"))]);
        assert_eq!(prog.sections,
                   vec![Range { start: 0, end: 3 },
                        Range { start: 3, end: 7 }]);
    }
}
