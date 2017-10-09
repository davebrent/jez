use std::collections::HashMap;
use std::collections::hash_map::{DefaultHasher, Entry};
use std::hash::Hasher;

use super::parse::{Directive, Token, Value};
use err::AssemErr;
use interp::Instr;

pub fn hash_str(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

pub fn assemble(dirs: &[Directive]) -> Result<Vec<Instr>, AssemErr> {
    let mut globals: HashMap<&str, Value> = HashMap::new();
    let mut funcs: HashMap<u64, (usize, usize)> = HashMap::new();
    let mut instrs = Vec::new();

    let mut string_map: HashMap<&str, usize> = HashMap::new();
    let mut strings = Vec::new();

    if !dirs.is_empty() && *dirs.first().unwrap() != Directive::Version(1) {
        return Err(AssemErr::UnsupportedVersion(1));
    }

    for dir in dirs {
        match *dir {
            Directive::Comment(_) |
            Directive::Version(_) => (),
            Directive::Globals(ref globs) => {
                for (key, value) in globs.iter() {
                    if globals.contains_key(key) {
                        return Err(AssemErr::DuplicateVariable);
                    }
                    globals.insert(key, *value);
                }
            }
            Directive::Func(name, args, ref words) => {
                let name_sym = hash_str(name);
                if funcs.contains_key(&name_sym) {
                    return Err(AssemErr::DuplicateFunction);
                }

                let ilen = instrs.len();
                funcs.insert(name_sym, (args as usize, ilen));
                instrs.push(Instr::Begin(name_sym));

                for word in words {
                    match *word {
                        Token::Comment(_) => (),
                        Token::ListBegin => {
                            instrs.push(Instr::ListBegin);
                        }
                        Token::ListEnd => {
                            instrs.push(Instr::ListEnd);
                        }
                        Token::Null => {
                            instrs.push(Instr::Null);
                        }
                        Token::Symbol(var) => {
                            instrs.push(Instr::LoadSymbol(hash_str(var)));
                        }
                        Token::Assignment(var) => {
                            instrs.push(Instr::StoreVar(hash_str(var)));
                        }
                        Token::Variable(var) => {
                            instrs.push(Instr::LoadVar(hash_str(var)));
                        }
                        Token::StringLiteral(literal) => {
                            let idx = match string_map.entry(literal) {
                                Entry::Occupied(o) => *o.get(),
                                Entry::Vacant(v) => {
                                    let idx = strings.len();
                                    v.insert(idx);
                                    strings.push(literal);
                                    idx
                                }
                            };
                            instrs.push(Instr::LoadString(idx as u64));
                        }
                        Token::Value(prim) => {
                            match prim {
                                Value::Num(num) => {
                                    instrs.push(Instr::LoadNumber(num));
                                }
                                Value::Str(word) => {
                                    let sym = hash_str(word);

                                    if funcs.contains_key(&sym) {
                                        let (_args, pc) = funcs[&sym];
                                        instrs.push(Instr::Call(_args, pc));
                                    } else {
                                        instrs.push(Instr::Keyword(sym));
                                    }
                                }
                            }
                        }
                    }
                }

                instrs.push(Instr::Return);
                instrs.push(Instr::End(name_sym));
            }
        }
    }

    instrs.push(Instr::Begin(0));

    // Ensure variables are listed deterministicly
    let mut global_keys: Vec<&&str> = globals.keys().collect();
    global_keys.sort();
    for key in &global_keys {
        match globals[*key] {
            Value::Str(sym) => instrs.push(Instr::LoadSymbol(hash_str(sym))),
            Value::Num(num) => instrs.push(Instr::LoadNumber(num)),
        };
        instrs.push(Instr::StoreGlob(hash_str(key)));
    }

    // Pack string literals into the program
    for (i, literal) in strings.iter().enumerate() {
        let bytes = literal.as_bytes();
        instrs.push(Instr::StoreString(i as u64, bytes.len() as u64));
        for b in bytes {
            instrs.push(Instr::RawData(*b));
        }
    }

    instrs.push(Instr::Return);
    instrs.push(Instr::End(0));
    Ok(instrs)
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