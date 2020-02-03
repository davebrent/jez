use std::collections::hash_map::{DefaultHasher, Entry};
use std::collections::HashMap;
use std::hash::Hasher;

use super::dirs::{Argument, Code, Directive, Location, Name, Symbol, Value};
use crate::err::Error;
use crate::vm::Instr;

pub fn hash_str(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

struct Assembler<'a> {
    globals: HashMap<&'a str, Instr>,
    funcs: HashMap<u64, (usize, usize)>,
    tracks: Vec<u64>,
    instrs: Vec<Instr>,
    string_map: HashMap<&'a str, usize>,
    strings: Vec<&'a str>,
    debug: Vec<(usize, Location)>,
}

impl<'a> Assembler<'a> {
    pub fn new() -> Assembler<'a> {
        Assembler {
            globals: HashMap::new(),
            funcs: HashMap::new(),
            tracks: Vec::new(),
            instrs: Vec::new(),
            string_map: HashMap::new(),
            strings: Vec::new(),
            debug: Vec::new(),
        }
    }

    /// Check the language version matches the expected version
    fn version_directive(&mut self, dir: &'a Directive) -> Result<(), Error> {
        if dir.args.len() != 1 {
            return Err(error!(UnsupportedVersion));
        }

        let arg = r#try!(r#try!(r#try!(dir.arg_at(0)).as_value()).as_num());
        let ver = arg as u64;
        if ver != 0 {
            return Err(error!(UnsupportedVersion));
        }

        Ok(())
    }

    /// Declare and initialize global variables
    fn globals_directive(&mut self, dir: &'a Directive) -> Result<(), Error> {
        for token in &dir.args {
            match *token {
                Argument::Kwarg(ref key, ref val) => {
                    if self.globals.contains_key(key.data) {
                        return Err(error!(DuplicateVariable));
                    }
                    let instr = self.from_value(&val.data);
                    self.globals.insert(key.data, instr);
                }
                Argument::Arg(_) => {
                    return Err(error!(DuplicateVariable));
                }
            }
        }
        Ok(())
    }

    /// Define new keywords/functions
    fn define_directive(&mut self, dir: &'a Directive) -> Result<(), Error> {
        let arg = r#try!(dir.arg_at(0));
        let name = r#try!(arg.as_value());
        self.debug.push((self.instrs.len(), r#try!(arg.loc())));

        let name = hash_str(r#try!(name.as_keyword()));
        let args = r#try!(r#try!(r#try!(dir.arg_at(1)).as_value()).as_num()) as u64;
        self.emit_func(name, args, dir)
    }

    /// Define new track functions
    fn track_directive(&mut self, dir: &'a Directive) -> Result<(), Error> {
        let arg = r#try!(dir.arg_at(0));
        let name = r#try!(arg.as_value());
        self.debug.push((self.instrs.len(), r#try!(arg.loc())));

        let name = hash_str(r#try!(name.as_keyword()));
        r#try!(self.emit_func(name, 0, dir));
        self.tracks.push(name);
        Ok(())
    }

    fn emit_func(&mut self, name: u64, args: u64, dir: &'a Directive) -> Result<(), Error> {
        if self.funcs.contains_key(&name) {
            return Err(error!(DuplicateFunction));
        }

        self.funcs.insert(name, (args as usize, self.instrs.len()));
        self.instrs.push(Instr::Begin(name));

        for token in &dir.body {
            let instr = match token.data {
                Code::Symbol(sym) => match sym {
                    Symbol::ListBegin => Instr::ListBegin,
                    Symbol::ListEnd => Instr::ListEnd,
                    Symbol::SeqBegin => Instr::SeqBegin,
                    Symbol::SeqEnd => Instr::SeqEnd,
                    Symbol::GroupBegin => Instr::GroupBegin,
                    Symbol::GroupEnd => Instr::GroupEnd,
                    Symbol::Null => Instr::Null,
                    Symbol::Assign(var) => Instr::StoreVar(hash_str(var)),
                },
                Code::Value(ref val) => self.from_value(val),
            };
            self.debug.push((self.instrs.len(), token.loc));
            self.instrs.push(instr);
        }

        self.instrs.push(Instr::Return);
        self.instrs.push(Instr::End(name));
        Ok(())
    }

    pub fn assemble(&mut self, prog: &'a str, dirs: &'a [Directive]) -> Result<Vec<Instr>, Error> {
        for dir in dirs {
            let res = match dir.name.data {
                Name::Version => self.version_directive(dir),
                Name::Globals => self.globals_directive(dir),
                Name::Def => self.define_directive(dir),
                Name::Track => self.track_directive(dir),
            };
            r#try!(res);
        }

        self.instrs.push(Instr::Begin(0));
        // Pack global variables deterministicly
        let mut global_keys: Vec<&&str> = self.globals.keys().collect();
        global_keys.sort();
        for key in &global_keys {
            self.instrs.push(self.globals[*key]);
            self.instrs.push(Instr::StoreGlob(hash_str(key)));
        }

        // Map instructions to tokens
        for &(pc, loc) in &self.debug {
            let tk = &prog[loc.begin..loc.end];
            let id = self.strings.len();
            self.strings.push(tk);
            self.instrs.push(Instr::SourceLoc(
                pc as u64,
                id as u64,
                loc.line as u64,
                loc.col as u64,
            ));
        }

        // Pack string literals
        for (i, literal) in self.strings.iter().enumerate() {
            let bytes = literal.as_bytes();
            self.instrs
                .push(Instr::StoreString(i as u64, bytes.len() as u64));
            for b in bytes {
                self.instrs.push(Instr::RawData(*b));
            }
        }
        self.instrs.push(Instr::Return);
        self.instrs.push(Instr::End(0));

        self.instrs.push(Instr::Begin(1));
        // Return a list of track functions
        self.instrs.push(Instr::ListBegin);
        for track in &self.tracks {
            self.instrs.push(Instr::LoadSymbol(*track));
        }
        self.instrs.push(Instr::ListEnd);
        self.instrs.push(Instr::Return);
        self.instrs.push(Instr::End(1));

        Ok(self.instrs.clone())
    }

    fn from_value(&mut self, value: &'a Value) -> Instr {
        match *value {
            Value::Variable(var) => Instr::LoadVar(hash_str(var)),

            Value::Number(num) => Instr::LoadNumber(num),

            Value::StringLiteral(literal) => {
                let idx = match self.string_map.entry(literal) {
                    Entry::Occupied(o) => *o.get(),
                    Entry::Vacant(v) => {
                        let idx = self.strings.len();
                        v.insert(idx);
                        self.strings.push(literal);
                        idx
                    }
                };
                Instr::LoadString(idx as u64)
            }

            Value::Symbol(var) => Instr::LoadSymbol(hash_str(var)),

            Value::Keyword(word) => {
                let sym = hash_str(word);
                if self.funcs.contains_key(&sym) {
                    let (args, pc) = self.funcs[&sym];
                    Instr::Call(args, pc)
                } else {
                    Instr::Keyword(sym)
                }
            }
        }
    }
}

pub fn assemble(prog: &str, dirs: &[Directive]) -> Result<Vec<Instr>, Error> {
    Assembler::new().assemble(prog, dirs)
}

#[cfg(test)]
mod tests {
    use super::super::dirs::Token;
    use super::*;

    #[test]
    fn test_strings() {
        let dirs = vec![
            Directive {
                name: Token::new(Name::Version, Default::default()),
                args: vec![Argument::Arg(Token::new(
                    Value::Number(0.0),
                    Default::default(),
                ))],
                body: vec![],
            },
            Directive {
                name: Token::new(Name::Def, Default::default()),
                args: vec![
                    Argument::Arg(Token::new(Value::Keyword("main"), Default::default())),
                    Argument::Arg(Token::new(Value::Number(0.0), Default::default())),
                ],
                body: vec![
                    Token::new(Code::Value(Value::StringLiteral("abc")), Default::default()),
                    Token::new(Code::Value(Value::StringLiteral("def")), Default::default()),
                    Token::new(Code::Value(Value::StringLiteral("abc")), Default::default()),
                ],
            },
        ];

        let result = assemble("", &dirs).unwrap();
        let instrs = vec![
            Instr::Begin(17450787904383802648),
            Instr::LoadString(0),
            Instr::LoadString(1),
            Instr::LoadString(0),
            Instr::Return,
            Instr::End(17450787904383802648),
            Instr::Begin(0),
            Instr::SourceLoc(0, 2, 1, 0),
            Instr::SourceLoc(1, 3, 1, 0),
            Instr::SourceLoc(2, 4, 1, 0),
            Instr::SourceLoc(3, 5, 1, 0),
            // abc
            Instr::StoreString(0, 3),
            Instr::RawData(97),
            Instr::RawData(98),
            Instr::RawData(99),
            // def
            Instr::StoreString(1, 3),
            Instr::RawData(100),
            Instr::RawData(101),
            Instr::RawData(102),
            Instr::StoreString(2, 0),
            Instr::StoreString(3, 0),
            Instr::StoreString(4, 0),
            Instr::StoreString(5, 0),
            Instr::Return,
            Instr::End(0),
            Instr::Begin(1),
            Instr::ListBegin,
            Instr::ListEnd,
            Instr::Return,
            Instr::End(1),
        ];
        assert_eq!(result, instrs);
        let abc = String::from_utf8(vec![97, 32, 98, 32, 99]).unwrap();
        let def = String::from_utf8(vec![100, 101, 102]).unwrap();
        assert_eq!(abc, String::from("a b c"));
        assert_eq!(def, String::from("def"));
    }

    #[test]
    fn test_simple() {
        let dirs = vec![
            Directive {
                name: Token::new(Name::Version, Default::default()),
                args: vec![Argument::Arg(Token::new(
                    Value::Number(0.0),
                    Default::default(),
                ))],
                body: vec![],
            },
            Directive {
                name: Token::new(Name::Globals, Default::default()),
                args: vec![
                    Argument::Kwarg(
                        Token::new("b", Default::default()),
                        Token::new(Value::Number(2.0), Default::default()),
                    ),
                    Argument::Kwarg(
                        Token::new("a", Default::default()),
                        Token::new(Value::Number(3.9), Default::default()),
                    ),
                ],
                body: vec![],
            },
            Directive {
                name: Token::new(Name::Def, Default::default()),
                args: vec![
                    Argument::Arg(Token::new(Value::Keyword("bar"), Default::default())),
                    Argument::Arg(Token::new(Value::Number(1.0), Default::default())),
                ],
                body: vec![
                    Token::new(Code::Value(Value::Number(2.7)), Default::default()),
                    Token::new(Code::Value(Value::Keyword("add")), Default::default()),
                ],
            },
            Directive {
                name: Token::new(Name::Def, Default::default()),
                args: vec![
                    Argument::Arg(Token::new(Value::Keyword("foo"), Default::default())),
                    Argument::Arg(Token::new(Value::Number(1.0), Default::default())),
                ],
                body: vec![
                    Token::new(Code::Value(Value::Number(3.6)), Default::default()),
                    Token::new(Code::Value(Value::Keyword("bar")), Default::default()),
                ],
            },
        ];

        let result = assemble("", &dirs).unwrap();
        let instrs = vec![
            Instr::Begin(15647602356402206823),
            Instr::LoadNumber(2.7),
            Instr::Keyword(16243785806421205142),
            Instr::Return,
            Instr::End(15647602356402206823),
            Instr::Begin(7664243301495174138),
            Instr::LoadNumber(3.6),
            Instr::Call(1, 0),
            Instr::Return,
            Instr::End(7664243301495174138),
            Instr::Begin(0),
            Instr::LoadNumber(3.9),
            Instr::StoreGlob(4644417185603328019),
            Instr::LoadNumber(2.0),
            Instr::StoreGlob(10025803482645881038),
            Instr::SourceLoc(0, 0, 1, 0),
            Instr::SourceLoc(1, 1, 1, 0),
            Instr::SourceLoc(2, 2, 1, 0),
            Instr::SourceLoc(5, 3, 1, 0),
            Instr::SourceLoc(6, 4, 1, 0),
            Instr::SourceLoc(7, 5, 1, 0),
            Instr::StoreString(0, 0),
            Instr::StoreString(1, 0),
            Instr::StoreString(2, 0),
            Instr::StoreString(3, 0),
            Instr::StoreString(4, 0),
            Instr::StoreString(5, 0),
            Instr::Return,
            Instr::End(0),
            Instr::Begin(1),
            Instr::ListBegin,
            Instr::ListEnd,
            Instr::Return,
            Instr::End(1),
        ];
        assert_eq!(result, instrs);
    }
}
