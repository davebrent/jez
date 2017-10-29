use std::collections::HashMap;
use std::collections::hash_map::{DefaultHasher, Entry};
use std::hash::Hasher;

use super::parse::{Directive, Token, Value};
use err::AssemErr;
use vm::Instr;

pub fn hash_str(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

struct Assembler<'a> {
    globals: HashMap<&'a str, Value<'a>>,
    funcs: HashMap<u64, (usize, usize)>,
    instrs: Vec<Instr>,
    string_map: HashMap<&'a str, usize>,
    strings: Vec<&'a str>,
}

impl<'a> Assembler<'a> {
    pub fn new() -> Assembler<'a> {
        Assembler {
            globals: HashMap::new(),
            funcs: HashMap::new(),
            instrs: Vec::new(),
            string_map: HashMap::new(),
            strings: Vec::new(),
        }
    }

    pub fn assemble(&mut self,
                    dirs: &'a [Directive])
                    -> Result<Vec<Instr>, AssemErr> {
        for directive in dirs {
            match *directive {
                Directive::Comment(_) => (),
                Directive::Func(name, args, ref words) => {
                    try!(self.emit_func(name, args, words))
                }
                Directive::Globals(ref globals) => {
                    try!(self.emit_globals(globals))
                }
                Directive::Version(ver) => {
                    if ver != 1 {
                        return Err(AssemErr::UnsupportedVersion(1));
                    }
                }
            }
        }
        self.emit_footer();
        Ok(self.instrs.clone())
    }

    fn emit_footer(&mut self) {
        self.instrs.push(Instr::Begin(0));

        // Pack global variables deterministicly
        let mut global_keys: Vec<&&str> = self.globals.keys().collect();
        global_keys.sort();
        for key in &global_keys {
            let instr = match self.globals[*key] {
                Value::Str(sym) => Instr::LoadSymbol(hash_str(sym)),
                Value::Num(num) => Instr::LoadNumber(num),
            };
            self.instrs.push(instr);
            self.instrs.push(Instr::StoreGlob(hash_str(key)));
        }

        // Pack string literals
        for (i, literal) in self.strings.iter().enumerate() {
            let bytes = literal.as_bytes();
            self.instrs.push(
                Instr::StoreString(i as u64, bytes.len() as u64),
            );
            for b in bytes {
                self.instrs.push(Instr::RawData(*b));
            }
        }

        self.instrs.push(Instr::Return);
        self.instrs.push(Instr::End(0));
    }

    fn emit_globals(&mut self,
                    globals: &'a HashMap<&str, Value>)
                    -> Result<(), AssemErr> {
        for (key, value) in globals.iter() {
            if self.globals.contains_key(key) {
                return Err(AssemErr::DuplicateVariable);
            }
            self.globals.insert(key, *value);
        }
        Ok(())
    }

    fn emit_func(&mut self,
                 name: &str,
                 args: u64,
                 words: &'a [Token])
                 -> Result<(), AssemErr> {
        let name = hash_str(name);
        if self.funcs.contains_key(&name) {
            return Err(AssemErr::DuplicateFunction);
        }

        self.funcs.insert(name, (args as usize, self.instrs.len()));
        self.instrs.push(Instr::Begin(name));

        for word in words {
            match *word {
                Token::Comment(_) => (),
                Token::ListBegin => {
                    self.instrs.push(Instr::ListBegin);
                }
                Token::ListEnd => {
                    self.instrs.push(Instr::ListEnd);
                }
                Token::Null => {
                    self.instrs.push(Instr::Null);
                }
                Token::Symbol(var) => {
                    self.instrs.push(Instr::LoadSymbol(hash_str(var)));
                }
                Token::Assignment(var) => {
                    self.instrs.push(Instr::StoreVar(hash_str(var)));
                }
                Token::Variable(var) => {
                    self.instrs.push(Instr::LoadVar(hash_str(var)));
                }
                Token::StringLiteral(literal) => self.emit_str_lit(literal),
                Token::Value(prim) => self.emit_value(prim),
            }
        }

        self.instrs.push(Instr::Return);
        self.instrs.push(Instr::End(name));
        Ok(())
    }

    fn emit_str_lit(&mut self, literal: &'a str) {
        let idx = match self.string_map.entry(literal) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let idx = self.strings.len();
                v.insert(idx);
                self.strings.push(literal);
                idx
            }
        };
        self.instrs.push(Instr::LoadString(idx as u64));
    }

    fn emit_value(&mut self, value: Value) {
        let instr = match value {
            Value::Num(num) => Instr::LoadNumber(num),
            Value::Str(word) => {
                let sym = hash_str(word);
                if self.funcs.contains_key(&sym) {
                    let (args, pc) = self.funcs[&sym];
                    Instr::Call(args, pc)
                } else {
                    Instr::Keyword(sym)
                }
            }
        };
        self.instrs.push(instr);
    }
}

pub fn assemble(dirs: &[Directive]) -> Result<Vec<Instr>, AssemErr> {
    Assembler::new().assemble(dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strings() {
        let dirs = vec![
            Directive::Version(1),
            Directive::Func(
                "main",
                0,
                vec![
                    Token::StringLiteral("abc"),
                    Token::StringLiteral("def"),
                    Token::StringLiteral("abc"),
                ]
            ),
        ];

        let result = assemble(&dirs).unwrap();
        let instrs = vec![
            Instr::Begin(17450787904383802648),
            Instr::LoadString(0),
            Instr::LoadString(1),
            Instr::LoadString(0),
            Instr::Return,
            Instr::End(17450787904383802648),
            Instr::Begin(0),
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
            Instr::Return,
            Instr::End(0),
        ];
        assert_eq!(result, instrs);
        let abc = String::from_utf8(vec![97, 32, 98, 32, 99]).unwrap();
        let def = String::from_utf8(vec![100, 101, 102]).unwrap();
        assert_eq!(abc, String::from("a b c"));
        assert_eq!(def, String::from("def"));
    }

    #[test]
    fn test_simple() {
        let mut globs = HashMap::new();
        globs.insert("b", Value::Num(2.0));
        globs.insert("a", Value::Num(3.9));

        let dirs = vec![
            Directive::Version(1),
            Directive::Globals(globs),
            Directive::Func(
                "bar",
                1,
                vec![
                    Token::Value(Value::Num(2.7)),
                    Token::Value(Value::Str("add")),
                ]
            ),
            Directive::Func(
                "foo",
                1,
                vec![
                    Token::Value(Value::Num(3.6)),
                    Token::Value(Value::Str("bar")),
                ]
            ),
        ];

        let result = assemble(&dirs).unwrap();
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
            Instr::Return,
            Instr::End(0),
        ];
        assert_eq!(result, instrs);
    }
}
