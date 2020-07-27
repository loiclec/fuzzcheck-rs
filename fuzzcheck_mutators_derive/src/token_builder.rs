
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

// little macro utility lib

// pub fn error_span(err: &str, span: Span) -> TokenStream {
//     let mut tb = TokenBuilder::new();
//     tb.ident_with_span("compile_error", span)
//         .add("! (")
//         .string(err)
//         .add(")");
//     tb.end()
// }

// pub fn error(err: &str) -> TokenStream {
//     let mut tb = TokenBuilder::new();
//     tb.add("compile_error ! (").string(err).add(")");
//     tb.end()
// }


pub struct TokenBuilder {
    pub groups: Vec<(Delimiter, TokenStream)>,
}

impl TokenBuilder {
    #[inline(never)]
    pub fn new() -> Self {
        Self {
            groups: vec![(Delimiter::None, TokenStream::new())],
        }
    }

    #[inline(never)]
    pub fn end(mut self) -> TokenStream {
        if self.groups.len() != 1 {
            panic!("Groups not empty, you missed a pop_group")
        }
        self.groups.pop().unwrap().1
    }

    #[inline(never)]
    pub fn eprint(&self) {
        eprintln!("{}", self.groups.last().unwrap().1.to_string());
    }

    #[inline(never)]
    pub fn extend<T: Into<TokenTree>>(&mut self, tt: T) {
        self.groups.last_mut().unwrap().1.extend(Some(tt.into()));
    }

    #[inline(never)]
    pub fn stream_opt(&mut self, what: Option<TokenStream>) {
        if let Some(what) = what {
            for c in what.into_iter() {
                self.extend(c);
            }
        }
    }

    #[inline(never)]
    pub fn stream(&mut self, what: TokenStream) {
        for c in what.into_iter() {
            self.extend(c);
        }
    }

    #[inline(never)]
    pub fn add(&mut self, what: &str) {
        for part in what.split(char::is_whitespace) {
            match part {
                "{" => self.push_group(Delimiter::Brace),
                "(" => self.push_group(Delimiter::Parenthesis),
                "[" => self.push_group(Delimiter::Bracket),
                "}" => self.pop_group(Delimiter::Brace),
                ")" => self.pop_group(Delimiter::Parenthesis),
                "]" => self.pop_group(Delimiter::Bracket),
                "+" | "-" | "*" | "/" | "%" | "^" | "!" | "&" | "|" | "&&" | "||" | "<<" | ">>" | "+=" | "-="
                | "*=" | "/=" | "%=" | "^=" | "&=" | "|=" | "<<=" | ">>=" | "=" | "==" | "!=" | ">" | "<" | ">="
                | "<=" | "@" | "." | ".." | "..." | "..=" | "," | ";" | ":" | "::" | "->" | "=>" | "#" | "$" | "?" => {
                    let mut last = None;
                    for c in part.chars() {
                        if let Some(last) = last {
                            self.extend(TokenTree::from(Punct::new(last, Spacing::Joint)));
                        }
                        last = Some(c);
                    }
                    if let Some(last) = last {
                        self.extend(TokenTree::from(Punct::new(last, Spacing::Alone)));
                    }
                }
                _ => {
                    if part.len() == 0 {
                        continue;
                    }
                    let mut chars = part.chars();
                    match chars.next().unwrap() {
                        '0'..='9' => {
                            static INTEGER_ERROR: &'static str = "Can't parse number in TokenBuilder::add. Only unsuffixed usize in base 10 are supported.";
                            if let Some(next) = chars.next() {
                                match next {
                                    'b' | 'o' | 'x' => panic!(INTEGER_ERROR),
                                    _ => (),
                                }
                            }
                            self.extend(TokenTree::from(Literal::usize_unsuffixed(part.parse().expect(INTEGER_ERROR))))
                        }
                        '\'' => {
                            if let Some('\'') = chars.last() {
                                panic!("Character literals are not supported in TokenBuilder::add");
                            } else {
                                self.extend(Punct::new('\'', Spacing::Joint));
                                self.extend(Ident::new(part.strip_prefix("\'").unwrap(), Span::call_site()));
                            }
                        }
                        '"' => {
                            panic!("String literals are not supported in TokenBuilder::add");
                        }
                        _ => self.extend(TokenTree::from(Ident::new(part, Span::call_site()))),
                    }
                }
            };
        }
    }

    #[inline(never)]
    pub fn push_group(&mut self, delim: Delimiter) {
        self.groups.push((delim, TokenStream::new()));
    }

    #[inline(never)]
    pub fn pop_group(&mut self, delim: Delimiter) {
        if self.groups.len() < 2 {
            // eprintln!("Stack dump for error:\n{}", self.stack_as_string());
            panic!("pop_group stack is empty {}", self.groups.len());
        }
        let ts = self.groups.pop().unwrap();
        if ts.0 != delim {
            // eprintln!("Stack dump for error:\n{}", self.stack_as_string());
            panic!("pop_group Delimiter mismatch, got {:?} expected {:?}", ts.0, delim);
        }
        self.extend(TokenTree::from(Group::new(delim, ts.1)));
    }
}
