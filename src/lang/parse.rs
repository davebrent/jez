use std::error::Error;
use std::fmt;

use err::ParseErr;

use super::dirs::{Argument, Code, Directive, Location, Name, Symbol, Token,
                  Value};


#[derive(Clone, Copy, Debug, PartialEq)]
enum Status {
    UnexpectedToken,
    Incomplete,
}

impl Error for Status {
    fn description(&self) -> &str {
        match *self {
            Status::UnexpectedToken => "unknown token",
            Status::Incomplete => "incomplete",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Status::UnexpectedToken => write!(f, "unknown token"),
            Status::Incomplete => write!(f, "incomplete"),
        }
    }
}

fn is_alphabetic(chr: char) -> bool {
    (chr as u8 >= 0x41 && chr as u8 <= 0x5A) ||
        (chr as u8 >= 0x61 && chr as u8 <= 0x7A)
}

fn is_digit(chr: char) -> bool {
    chr as u8 >= 0x30 && chr as u8 <= 0x39
}

fn is_alphanumeric(chr: char) -> bool {
    is_alphabetic(chr) || is_digit(chr)
}

fn is_line_ending(chr: char) -> bool {
    chr == '\r' || chr == '\n'
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct TokenStream<'a> {
    pub loc: Location,
    input: &'a str,
}

impl<'a> TokenStream<'a> {
    pub fn new(input: &'a str) -> TokenStream {
        TokenStream {
            loc: Location::new(1, 0, 0, input.len()),
            input: input,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.peek().is_none()
    }

    pub fn peek(&self) -> Option<(char, Location)> {
        let mut loc = self.loc.clone();
        let mut is_comment = false;

        loop {
            let string = match self.input.get(loc.begin..) {
                Some(string) => string,
                None => return None,
            };

            let tk = match string.chars().next() {
                Some(tk) => tk,
                None => return None,
            };

            if !is_comment && tk == ';' {
                is_comment = true;
                loc.begin += tk.len_utf8();
                loc.col += 1;
                continue;
            } else if is_comment && is_line_ending(tk) {
                is_comment = false;
            } else if is_comment {
                loc.begin += tk.len_utf8();
                loc.col += 1;
                continue;
            }

            if tk.is_whitespace() {
                loc.begin += tk.len_utf8();
                loc.col += 1;

                if is_line_ending(tk) {
                    loc.line += 1;
                    loc.col = 0;
                }
            } else {
                loc.end = loc.begin + 1;
                return Some((tk, loc));
            }
        }
    }

    pub fn next(&mut self) -> Option<(char, Location)> {
        match self.peek() {
            Some((tk, loc)) => {
                self.loc.begin = loc.begin + tk.len_utf8();
                self.loc.col = loc.col + 1;
                self.loc.line = loc.line;
                Some((tk, loc))
            }
            None => None,
        }
    }

    pub fn expect(&mut self, c: char) -> Result<(), Status> {
        match self.next() {
            Some((tk, _)) => {
                if c == tk {
                    Ok(())
                } else {
                    Err(Status::UnexpectedToken)
                }
            }
            None => Err(Status::Incomplete),
        }
    }

    pub fn take_while<F>(&mut self, cond: F) -> Option<(&'a str, Location)>
    where
        F: Fn(char) -> bool,
    {
        let (_, mut loc) = match self.peek() {
            Some((tk, loc)) => (tk, loc),
            None => return None,
        };

        while let Some((token, pos)) = self.peek() {
            // Encounted white space
            if pos.end - loc.end > 1 {
                break;
            }

            if cond(token) {
                loc.end = pos.end;
                self.next().unwrap();
            } else {
                break;
            }
        }

        match self.input.get(loc.begin..loc.end) {
            Some(string) => Some((string, loc)),
            None => None,
        }
    }
}

struct Parser<'c, 's: 'c> {
    stream: &'c mut TokenStream<'s>,
}

// Recursive descent parser for the following grammar
//
// start: directive*
//
// directive : "." name [arg+] [":" code+]
// name      : "version"       -> version
//           | "globals"       -> globals
//           | "def"           -> def
//           | "track"         -> track
// arg       : (VARIABLE "=" value) | value
// ?code     : (symbol | value)
// value     : SIGNED_NUMBER   -> number
//           | ESCAPED_STRING  -> string
//           | WORD            -> word
//           | "'" WORD        -> hash
//           | VARIABLE        -> variable
//
// symbol    : "["             -> list_begin
//           | "]"             -> list_end
//           | "("             -> seq_begin
//           | ")"             -> seq_end
//           | "{"             -> group_begin
//           | "}"             -> group_end
//           | "~"             -> null
//           | "=" VARIABLE    -> assign
//
// WORD      : LETTER ("_" | "#" | LETTER | DIGIT)*
// VARIABLE  : "@" WORD
//
// %import common.LETTER
// %import common.DIGIT
// %import common.WS
// %import common.SIGNED_NUMBER
// %import common.ESCAPED_STRING
//
// %ignore WS
impl<'c, 's: 'c> Parser<'c, 's> {
    pub fn new(stream: &'c mut TokenStream<'s>) -> Parser<'c, 's> {
        Parser { stream: stream }
    }

    pub fn parse(&mut self) -> Result<Vec<Directive<'s>>, ParseErr> {
        let mut dirs = vec![];

        while !self.stream.is_empty() {
            match self.parse_directive() {
                Ok(dir) => dirs.push(dir),
                Err(status) => {
                    let line = self.stream.loc.line;
                    let col = self.stream.loc.col;
                    match status {
                        Status::Incomplete => {
                            return Err(ParseErr::Incomplete(line, col));
                        }
                        Status::UnexpectedToken => {
                            return Err(ParseErr::UnexpectedToken(line, col));
                        }
                    }
                }
            };
        }

        Ok(dirs)
    }

    fn parse_name(&mut self) -> Result<Token<Name>, Status> {
        let (tk, loc) = match self.stream.take_while(|c| c.is_alphabetic()) {
            Some(tk) => tk,
            None => return Err(Status::Incomplete),
        };

        let name = match tk {
            "version" => Name::Version,
            "globals" => Name::Globals,
            "def" => Name::Def,
            "track" => Name::Track,
            _ => return Err(Status::UnexpectedToken),
        };

        Ok(Token::new(name, loc))
    }

    fn parse_word(&mut self) -> Result<Token<&'s str>, Status> {
        match self.stream.peek() {
            Some((token, _)) => {
                if !is_alphabetic(token) {
                    return Err(Status::UnexpectedToken);
                }
            }
            None => return Err(Status::Incomplete),
        };

        let res = self.stream.take_while(|c| {
            is_alphanumeric(c) || c == '#' || c == '_' || c == '-'
        });

        match res {
            Some((string, loc)) => Ok(Token::new(string, loc)),
            None => Err(Status::Incomplete),
        }
    }

    fn parse_value(&mut self) -> Result<Token<Value<'s>>, Status> {
        let tk = match self.stream.peek() {
            Some((tk, _)) => tk,
            None => return Err(Status::Incomplete),
        };

        let val = match tk {
            '@' => {
                let var = try!(self.parse_variable());
                Token::new(Value::Variable(var.data), var.loc)
            }
            '\'' => {
                try!(self.stream.expect('\''));
                let word = try!(self.parse_word());
                Token::new(Value::Symbol(word.data), word.loc)
            }
            '"' => {
                self.stream.next().unwrap(); // "
                // FIXME: Handle escaping + white space
                let (string, loc) =
                    self.stream.take_while(|c| c != '"').unwrap();
                self.stream.next().unwrap(); // "
                Token::new(Value::StringLiteral(string), loc)
            }
            _ => {
                if is_digit(tk) || tk == '-' {
                    let (raw, loc) = self.stream
                        .take_while(|c| is_digit(c) || c == '-' || c == '.')
                        .unwrap();
                    let num = raw.parse::<f64>().unwrap();
                    Token::new(Value::Number(num), loc)
                } else {
                    let word = try!(self.parse_word());
                    Token::new(Value::Keyword(word.data), word.loc)
                }
            }
        };

        Ok(val)
    }

    fn parse_arg(&mut self) -> Result<Argument<'s>, Status> {
        let tk = match self.stream.peek() {
            Some((tk, _)) => tk,
            None => return Err(Status::Incomplete),
        };

        if tk == '@' {
            let key = try!(self.parse_variable());
            self.stream.next().unwrap(); // =
            let val = try!(self.parse_value());
            Ok(Argument::Kwarg(key, val))
        } else {
            let val = try!(self.parse_value());
            Ok(Argument::Arg(val))
        }
    }

    fn parse_variable(&mut self) -> Result<Token<&'s str>, Status> {
        self.stream.next().unwrap(); // @
        self.parse_word()
    }

    fn parse_code(&mut self) -> Result<Token<Code<'s>>, Status> {
        let (token, loc) = match self.stream.peek() {
            Some((token, loc)) => (token, loc),
            None => return Err(Status::Incomplete),
        };

        let val = match token {
            '[' => {
                self.stream.next().unwrap();
                Token::new(Code::Symbol(Symbol::ListBegin), loc)
            }
            ']' => {
                self.stream.next().unwrap();
                Token::new(Code::Symbol(Symbol::ListEnd), loc)
            }
            '(' => {
                self.stream.next().unwrap();
                Token::new(Code::Symbol(Symbol::SeqBegin), loc)
            }
            ')' => {
                self.stream.next().unwrap();
                Token::new(Code::Symbol(Symbol::SeqEnd), loc)
            }
            '{' => {
                self.stream.next().unwrap();
                Token::new(Code::Symbol(Symbol::GroupBegin), loc)
            }
            '}' => {
                self.stream.next().unwrap();
                Token::new(Code::Symbol(Symbol::GroupEnd), loc)
            }
            '~' => {
                self.stream.next().unwrap();
                Token::new(Code::Symbol(Symbol::Null), loc)
            }
            '=' => {
                self.stream.next().unwrap();
                let var = try!(self.parse_variable());
                Token::new(Code::Symbol(Symbol::Assign(var.data)), loc)
            }
            _ => {
                let val = try!(self.parse_value());
                Token::new(Code::Value(val.data), val.loc)
            }
        };

        Ok(val)
    }

    fn parse_directive(&mut self) -> Result<Directive<'s>, Status> {
        let token = match self.stream.peek() {
            Some((token, _)) => token,
            None => return Err(Status::Incomplete),
        };

        if token != '.' {
            return Err(Status::UnexpectedToken);
        }

        self.stream.next().unwrap(); // .
        let name = try!(self.parse_name());
        let mut args = vec![];
        let mut body = vec![];

        while let Some((tk, _)) = self.stream.peek() {
            if tk == '.' || tk == ':' {
                break;
            }
            let arg = try!(self.parse_arg());
            args.push(arg);
        }

        let token = match self.stream.peek() {
            Some((token, _)) => token,
            None => return Err(Status::Incomplete),
        };

        if token == ':' {
            self.stream.next().unwrap(); // :
            while let Some((tk, _)) = self.stream.peek() {
                if tk == '.' {
                    break;
                }
                let code = try!(self.parse_code());
                body.push(code);
            }
        }

        Ok(Directive {
            name: name,
            args: args,
            body: body,
        })
    }
}

pub fn parser(txt: &str) -> Result<Vec<Directive>, ParseErr> {
    let mut stream = TokenStream::new(txt);
    let mut parser = Parser::new(&mut stream);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_next() {
        let mut ts = TokenStream::new("\n\t.de fu");
        assert_eq!(ts.next().unwrap(), ('.', Location::new(2, 1, 2, 3)));
        assert_eq!(ts.next().unwrap(), ('d', Location::new(2, 2, 3, 4)));
        assert_eq!(ts.next().unwrap(), ('e', Location::new(2, 3, 4, 5)));
        assert_eq!(ts.next().unwrap(), ('f', Location::new(2, 5, 6, 7)));
        assert_eq!(ts.next().unwrap(), ('u', Location::new(2, 6, 7, 8)));
    }

    #[test]
    fn test_stream_peek() {
        let mut ts = TokenStream::new("\n\t.de fu");
        assert_eq!(ts.peek().unwrap(), ('.', Location::new(2, 1, 2, 3)));
        assert_eq!(ts.next().unwrap(), ('.', Location::new(2, 1, 2, 3)));
        assert_eq!(ts.peek().unwrap(), ('d', Location::new(2, 2, 3, 4)));
        assert_eq!(ts.next().unwrap(), ('d', Location::new(2, 2, 3, 4)));
        assert_eq!(ts.peek().unwrap(), ('e', Location::new(2, 3, 4, 5)));
        assert_eq!(ts.next().unwrap(), ('e', Location::new(2, 3, 4, 5)));
        assert_eq!(ts.peek().unwrap(), ('f', Location::new(2, 5, 6, 7)));
        assert_eq!(ts.next().unwrap(), ('f', Location::new(2, 5, 6, 7)));
        assert_eq!(ts.peek().unwrap(), ('u', Location::new(2, 6, 7, 8)));
        assert_eq!(ts.next().unwrap(), ('u', Location::new(2, 6, 7, 8)));
    }

    #[test]
    fn test_stream_next_peek_order() {
        let mut ts = TokenStream::new("\n\t.de fu");
        assert_eq!(ts.next().unwrap(), ('.', Location::new(2, 1, 2, 3)));
        assert_eq!(ts.peek().unwrap(), ('d', Location::new(2, 2, 3, 4)));
    }

    #[test]
    fn test_stream_take_while_whitespace() {
        let mut ts = TokenStream::new(" .foo  ");
        let (tk, loc) = ts.take_while(|_| true).unwrap();
        assert_eq!(tk, ".foo");
        assert_eq!(loc, Location::new(1, 1, 1, 5));
        assert_eq!(ts.next(), None);
    }

    #[test]
    fn test_stream_take_while_eof() {
        let mut ts = TokenStream::new(" .foo");
        let (tk, _) = ts.take_while(|_| true).unwrap();
        assert_eq!(tk, ".foo");
        assert_eq!(ts.next(), None);
    }

    #[test]
    fn test_stream_take_while_multiple() {
        let mut ts = TokenStream::new("foo bar");

        let (a, loc) = ts.take_while(|_| true).unwrap();
        assert_eq!(a, "foo");
        assert_eq!(loc, Location::new(1, 0, 0, 3));

        let (b, loc) = ts.take_while(|_| true).unwrap();
        assert_eq!(b, "bar");
        assert_eq!(loc, Location::new(1, 4, 4, 7));
    }

    #[test]
    fn test_stream_comments() {
        let mut ts = TokenStream::new("foo ; comment \nbar");

        let (a, _) = ts.take_while(|_| true).unwrap();
        assert_eq!(a, "foo");

        let (b, _) = ts.take_while(|_| true).unwrap();
        assert_eq!(b, "bar");
    }
}
