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
    globals: HashMap<&'a str, Instr>,
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
                    if ver != 0 {
                        return Err(AssemErr::UnsupportedVersion(ver));
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
            self.instrs.push(self.globals[*key]);
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
            let instr = self.pack_value(value);
            self.globals.insert(key, instr);
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
                Token::Assignment(var) => {
                    self.instrs.push(Instr::StoreVar(hash_str(var)));
                }
                Token::Variable(var) => {
                    self.instrs.push(Instr::LoadVar(hash_str(var)));
                }
                Token::Value(ref prim) => {
                    let instr = self.pack_value(prim);
                    self.instrs.push(instr);
                }
            }
        }

        self.instrs.push(Instr::Return);
        self.instrs.push(Instr::End(name));
        Ok(())
    }

    fn pack_value(&mut self, value: &'a Value) -> Instr {
        match *value {
            Value::Null => Instr::Null,
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

pub fn assemble(dirs: &[Directive]) -> Result<Vec<Instr>, AssemErr> {
    Assembler::new().assemble(dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strings() {
        let dirs = vec![
            Directive::Version(0),
            Directive::Func(
                "main",
                0,
                vec![
                    Token::Value(Value::StringLiteral("abc")),
                    Token::Value(Value::StringLiteral("def")),
                    Token::Value(Value::StringLiteral("abc")),
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
        globs.insert("b", Value::Number(2.0));
        globs.insert("a", Value::Number(3.9));

        let dirs = vec![
            Directive::Version(0),
            Directive::Globals(globs),
            Directive::Func(
                "bar",
                1,
                vec![
                    Token::Value(Value::Number(2.7)),
                    Token::Value(Value::Keyword("add")),
                ]
            ),
            Directive::Func(
                "foo",
                1,
                vec![
                    Token::Value(Value::Number(3.6)),
                    Token::Value(Value::Keyword("bar")),
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
