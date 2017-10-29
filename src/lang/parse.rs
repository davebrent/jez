use std::collections::HashMap;
use std::str;

use nom::{IResult, digit, double, multispace, space};

use err::ParseErr;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Num(f64),
    Str(&'a str),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'a> {
    Assignment(&'a str),
    Comment(&'a str),
    ListBegin,
    ListEnd,
    Null,
    Symbol(&'a str),
    StringLiteral(&'a str),
    Value(Value<'a>),
    Variable(&'a str),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Directive<'a> {
    Comment(&'a str),
    Func(&'a str, u64, Vec<Token<'a>>),
    Globals(HashMap<&'a str, Value<'a>>),
    Version(u64),
}

fn string_chars(chr: u8) -> bool {
    let chr = chr as char;
    chr.is_alphanumeric() || chr == '-' || chr == '_' || chr == '.' ||
        chr == '#'
}

named!(string<&str>, do_parse!(
    opt!(multispace)
    >> not!(char!('.'))
    >> sym: map_res!(take_while_s!(string_chars), str::from_utf8)
    >> opt!(complete!(multispace))
    >> (sym)
));

named!(comment<&str>, do_parse!(
    char!(';')
    >> com: map_res!(
        terminated!(is_not!("\n"), alt!(eof!() | tag_s!("\n"))),
        str::from_utf8)
    >> (com)
));

named!(unsigned<u64>, map_res!(
    map_res!(digit, str::from_utf8),
    str::FromStr::from_str)
);

named!(integer<f64>, map_res!(
    map_res!(digit, str::from_utf8),
    str::FromStr::from_str)
);

named!(number<f64>, do_parse!(
    opt!(multispace)
    >> sym: alt!(complete!(double) | integer)
    >> opt!(complete!(multispace))
    >> (sym)
));

named!(variable<Token>, do_parse!(
    char!('$')
    >> sym: string
    >> (Token::Variable(sym))
));

named!(symbol<Token>, do_parse!(
    char!('\'')
    >> sym: string
    >> (Token::Symbol(sym))
));

named!(string_literal<Token>, do_parse!(
    char!('"')
    >> chars: map_res!(terminated!(is_not!("\""), tag_s!("\"")), str::from_utf8)
    >> (Token::StringLiteral(chars))
));

named!(assignment<Token>, do_parse!(
    char!('=')
    >> opt!(multispace)
    >> char!('$')
    >> sym: string
    >> (Token::Assignment(sym))
));

named!(value<Value>, do_parse!(
    val: alt!(
        number => { |n| Value::Num(n) }
        | string => { |s| Value::Str(s) }
    )
    >> (val)
));

named!(token<Token>, do_parse!(
    tk: ws!(alt!(
        comment => { |c| Token::Comment(c) }
        | char!('[') => { |c| Token::ListBegin }
        | char!(']') => { |c| Token::ListEnd }
        | char!('~') => { |c| Token::Null }
        | string_literal
        | symbol
        | variable
        | assignment
        | value => { |v| Token::Value(v) }
    ))
    >> (tk)
));

named!(key_value <(&str, Value)>, do_parse!(
    key: string
    >> char!('=')
    >> val: value
    >> (key, val)
));

named!(key_value_list<HashMap<&str, Value> >,
  fold_many0!(
    key_value,
    HashMap::new(),
    |mut acc: HashMap<_, _>, item| {
        let (key, val) = item;
        acc.insert(key, val);
        acc
    }
  )
);

named!(version<Directive>, do_parse!(
    tag!(".version")
    >> multispace
    >> version: unsigned
    >> (Directive::Version(version))
));

named!(globals<Directive>, do_parse!(
    tag!(".globals")
    >> res: key_value_list
    >> (Directive::Globals(res))
));

named!(func<Directive>, do_parse!(
    tag!(".def")
    >> name: string
    >> args: unsigned
    >> opt!(space)
    >> tag!(":")
    >> tokens: many0!(token)
    >> (Directive::Func(name, args, tokens))
));

named!(directive<Directive>, do_parse!(
    dir: alt!(
        comment => {|c| Directive::Comment(c) }
        | version
        | globals
        | func
    )
    >> opt!(complete!(multispace))
    >> (dir)
));

named!(pub directives<Vec<Directive> >, do_parse!(
    opt!(multispace)
    >> dirs: many0!(directive)
    >> (dirs)
));

fn cursor_pos(txt: &str, rest: &str) -> (usize, usize) {
    let mid = txt.len() - rest.len();
    let mut line = 0;
    let mut col = 0;

    for (i, ch) in txt.chars().enumerate() {
        if i == mid {
            break;
        } else if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

pub fn parser(txt: &str) -> Result<Vec<Directive>, ParseErr> {
    match directives(txt.as_bytes()) {
        IResult::Error(_) => {
            // XXX: Is this case possible?
            Err(ParseErr::InvalidInput)
        }
        IResult::Incomplete(_) => {
            let (line, col) = cursor_pos(txt, "");
            Err(ParseErr::InvalidSyntax(line, col))
        }
        IResult::Done(rest, dirs) => {
            if rest.is_empty() {
                Ok(dirs)
            } else {
                let rest = str::from_utf8(rest).unwrap();
                let (line, col) = cursor_pos(txt, rest);
                Err(ParseErr::UnknownToken(line, col))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Text

    #[test]
    fn test_string_underscore() {
        let sym = string(b"foo_foo");
        assert_eq!(sym.unwrap(), (&b""[..], "foo_foo"));
    }

    #[test]
    fn test_string_trailing_space() {
        let sym = string(b"foo_foo- ");
        assert_eq!(sym.unwrap(), (&b""[..], "foo_foo-"));
    }

    #[test]
    fn test_string_dotprefix() {
        let sym = string(b".foo_foo");
        assert_eq!(sym.is_err(), true);
    }

    // String literals

    #[test]
    fn test_string_literal() {
        let txt = token(b"\"baz foo / bar\"");
        let expected = Token::StringLiteral("baz foo / bar");
        assert_eq!(txt.unwrap(), (&b""[..], expected));
    }

    // Values

    #[test]
    fn test_value_float() {
        let sym = value(b"3.2");
        assert_eq!(sym.unwrap(), (&b""[..], Value::Num(3.2)));
    }

    #[test]
    fn test_value_int() {
        let sym = value(b"3");
        assert_eq!(sym.unwrap(), (&b""[..], Value::Num(3.0)));
    }

    #[test]
    fn test_value_str() {
        let sym = value(b"foobar");
        assert_eq!(sym.unwrap(), (&b""[..], Value::Str("foobar")));
    }

    // Comments

    #[test]
    fn test_comment_newline() {
        let com = comment(b"; a comment\n");
        assert_eq!(com.unwrap(), (&b""[..], " a comment"));
    }

    #[test]
    fn test_comment_no_newline() {
        let com = comment(b"; a comment");
        assert_eq!(com.unwrap(), (&b""[..], " a comment"));
    }

    // Tokens

    #[test]
    fn test_token_null() {
        let s = token(b"~");
        assert_eq!(s.unwrap(), (&b""[..], Token::Null));
    }

    #[test]
    fn test_token_list_begin() {
        let s = token(b"[");
        assert_eq!(s.unwrap(), (&b""[..], Token::ListBegin));
    }

    #[test]
    fn test_token_list_end() {
        let s = token(b"]");
        assert_eq!(s.unwrap(), (&b""[..], Token::ListEnd));
    }

    #[test]
    fn test_token_variable() {
        let s = token(b"$foo");
        assert_eq!(s.unwrap(), (&b""[..], Token::Variable("foo")));
    }

    #[test]
    fn test_token_symbol() {
        let s = token(b"'foo");
        assert_eq!(s.unwrap(), (&b""[..], Token::Symbol("foo")));
    }

    #[test]
    fn test_token_symbol_special_chars() {
        let s = token(b"'f#o_-o");
        assert_eq!(s.unwrap(), (&b""[..], Token::Symbol("f#o_-o")));
    }

    #[test]
    fn test_token_assignment() {
        let s = token(b"= $foo");
        assert_eq!(s.unwrap(), (&b""[..], Token::Assignment("foo")));
    }

    // Key values

    #[test]
    fn test_key_value() {
        let a = key_value(b"foo_foo=bar");
        assert_eq!(a.unwrap(), (&b""[..], ("foo_foo", Value::Str("bar"))));

        let a = key_value(b"foo_foo =bar");
        assert_eq!(a.unwrap(), (&b""[..], ("foo_foo", Value::Str("bar"))));

        let a = key_value(b"foo_foo = bar");
        assert_eq!(a.unwrap(), (&b""[..], ("foo_foo", Value::Str("bar"))));

        let a = key_value(b"foo_foo  =  \nbar ");
        assert_eq!(a.unwrap(), (&b""[..], ("foo_foo", Value::Str("bar"))));
    }

    #[test]
    fn test_key_value_lists() {
        let mut expected = HashMap::new();
        expected.insert("a", Value::Num(2.0));
        expected.insert("b", Value::Num(3.0));
        expected.insert("c", Value::Num(1.0));

        let res = key_value_list(b"a=2 b=3 c=1\n").unwrap();
        assert_eq!(res, (&b""[..], expected));

        let mut expected = HashMap::new();
        expected.insert("a", Value::Num(2.0));
        expected.insert("b", Value::Num(3.0));
        expected.insert("c", Value::Num(1.0));

        let res = key_value_list(b"a =2\n b =3\n c = 1 .");
        assert_eq!(res.unwrap(), (&b"."[..], expected));
    }

    // Directives

    #[test]
    fn test_version() {
        let res = version(b".version 2   ").unwrap();
        assert_eq!(res, (&b"   "[..], Directive::Version(2)));

        let res = version(b".version\n2").unwrap();
        assert_eq!(res, (&b""[..], Directive::Version(2)));
    }

    #[test]
    fn test_globals() {
        let mut expected = HashMap::new();
        expected.insert("b", Value::Num(3.0));
        expected.insert("a", Value::Num(2.0));

        let res = globals(b".globals b = 3.0 a = 2\n").unwrap();
        assert_eq!(res, (&b""[..], Directive::Globals(expected)));
    }

    #[test]
    fn test_func_oneline() {
        let res = func(b".def foobar 12: 12.0 3.2 add\n");
        let dir = Directive::Func(
            "foobar",
            12,
            vec![
                Token::Value(Value::Num(12.0)),
                Token::Value(Value::Num(3.2)),
                Token::Value(Value::Str("add"))
        ],
        );
        assert_eq!(res.unwrap(), (&b""[..], dir));
    }

    #[test]
    fn test_func_manylines() {
        let res = func(
            b".def foobar 12:\n

          12.0

        \t 3.2

           add

           \n
        ",
        );

        let dir = Directive::Func(
            "foobar",
            12,
            vec![
                Token::Value(Value::Num(12.0)),
                Token::Value(Value::Num(3.2)),
                Token::Value(Value::Str("add"))
            ],
        );
        assert_eq!(res.unwrap(), (&b""[..], dir));
    }

    #[test]
    fn test_directive() {
        let res = directive(b".version 1  ");
        assert_eq!(res.unwrap(), (&b""[..], Directive::Version(1)));

        let res = directive(
            b".def foo 0:
            1\n
        2\n
        add\n
        ",
        );

        let dir = Directive::Func(
            "foo",
            0,
            vec![
                Token::Value(Value::Num(1.0)),
                Token::Value(Value::Num(2.0)),
                Token::Value(Value::Str("add"))
            ],
        );
        assert_eq!(res.unwrap(), (&b""[..], dir));
    }

    #[test]
    fn test_directives() {
        let res = directives(
            b"

        .version 1

        .globals a=2
            b =3

            .def foo 3 : add
                binlist
              \t rev

        ",
        );

        let mut globals = HashMap::new();
        globals.insert("a", Value::Num(2.0));
        globals.insert("b", Value::Num(3.0));

        let dirs = vec![
            Directive::Version(1),
            Directive::Globals(globals),
            Directive::Func("foo", 3, vec![
                Token::Value(Value::Str("add")),
                Token::Value(Value::Str("binlist")),
                Token::Value(Value::Str("rev"))
            ])
        ];

        assert_eq!(res.unwrap(), (&b""[..], dirs));
    }

    #[test]
    fn test_directives_comments() {
        let res = directives(
            b"
        ;Another comment
        .def foo 3:
            add binlist
            ;More comment
            rev
        ; Even more",
        );

        let dirs = vec![
            Directive::Comment("Another comment"),
            Directive::Func("foo", 3, vec![
                 Token::Value(Value::Str("add")),
                 Token::Value(Value::Str("binlist")),
                 Token::Comment("More comment"),
                 Token::Value(Value::Str("rev")),
                 Token::Comment(" Even more")
            ])
        ];

        assert_eq!(res.unwrap(), (&b""[..], dirs));
    }

    // Errors

    #[test]
    fn test_incomplete() {
        let res = parser(
            "
.version",
        );
        assert!(res.is_err());
        match res {
            Err(err) => {
                assert_eq!(err, ParseErr::InvalidSyntax(1, 8));
            }
            _ => (),
        }
    }

    #[test]
    fn test_line_col_nos() {
        let res = parser(
            "
.version 1

.globals a=2 b=3
     @
.def foo 3:
    add binlist rev
        ",
        );

        assert!(res.is_err());
        match res {
            Err(err) => {
                assert_eq!(err, ParseErr::UnknownToken(4, 5));
            }
            _ => (),
        }
    }
}
