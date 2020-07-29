
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

pub trait TokenBuilderExtend {
    fn add_to(&self, tb: &mut TokenBuilder);
}
impl TokenBuilderExtend for Ident {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(self.clone());
    }
}
impl TokenBuilderExtend for Literal {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(self.clone());
    }
}
impl TokenBuilderExtend for Group {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(self.clone());
    }
}
impl TokenBuilderExtend for Punct {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(self.clone());
    }
}
impl TokenBuilderExtend for TokenTree {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(self.clone());
    }
}
impl TokenBuilderExtend for TokenStream {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.stream(self.clone());
    }
}
impl TokenBuilderExtend for usize {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(Literal::usize_unsuffixed(*self));
    }
}
impl TokenBuilderExtend for f64 {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(Literal::f64_suffixed(*self));
    }
}
impl<T> TokenBuilderExtend for Option<T> where T: TokenBuilderExtend {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        if let Some(x) = self {
            x.add_to(tb)
        }
    }
}
impl<T> TokenBuilderExtend for Vec<T> where T: TokenBuilderExtend {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        for x in self.iter() {
            x.add_to(tb)
        }
    }
}
impl TokenBuilderExtend for String {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.add(self)
    }
}
impl TokenBuilderExtend for &str {
    #[inline(never)]
    fn add_to(&self, tb: &mut TokenBuilder) {
        tb.add(self)
    }
}

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
    pub fn extend_tree<T: Into<TokenTree>>(&mut self, tt: T) {
        self.groups.last_mut().unwrap().1.extend(Some(tt.into()));
    }

    #[inline(never)]
    pub fn extend<T: TokenBuilderExtend>(&mut self, x: &T) {
        x.add_to(self)
    }

    #[inline(never)]
    pub fn stream(&mut self, what: TokenStream) {
        for c in what.into_iter() {
            self.extend(&c);
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
                            self.extend_tree(TokenTree::from(Punct::new(last, Spacing::Joint)));
                        }
                        last = Some(c);
                    }
                    if let Some(last) = last {
                        self.extend_tree(TokenTree::from(Punct::new(last, Spacing::Alone)));
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
                            self.extend_tree(TokenTree::from(Literal::usize_unsuffixed(part.parse().expect(INTEGER_ERROR))))
                        }
                        '\'' => {
                            if let Some('\'') = chars.last() {
                                panic!("Character literals are not supported in TokenBuilder::add");
                            } else {
                                self.extend_tree(Punct::new('\'', Spacing::Joint));
                                self.extend_tree(Ident::new(part.strip_prefix("\'").unwrap(), Span::call_site()));
                            }
                        }
                        '"' => {
                            panic!("String literals are not supported in TokenBuilder::add");
                        }
                        _ => self.extend_tree(TokenTree::from(Ident::new(part, Span::call_site()))),
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
        self.extend_tree(TokenTree::from(Group::new(delim, ts.1)));
    }
}
