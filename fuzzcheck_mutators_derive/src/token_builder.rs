
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

pub trait ToTokenStream {
    fn to_token_stream(&self) -> TokenStream;
}
pub trait ToTokenStreamPart<'a>  {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a>;
}
impl ToTokenStream for Ident {
    fn to_token_stream(&self) -> TokenStream {
        let tree: TokenTree = self.clone().into();
        tree.into()
    }
}
impl ToTokenStream for Literal {
    fn to_token_stream(&self) -> TokenStream {
        let tree: TokenTree = self.clone().into();
        tree.into()
    }
}
impl ToTokenStream for Group {
    fn to_token_stream(&self) -> TokenStream {
        let tree: TokenTree = self.clone().into();
        tree.into()
    }
}
impl ToTokenStream for TokenTree {
    fn to_token_stream(&self) -> TokenStream {
        self.clone().into()
    }
}
impl<'a> ToTokenStreamPart<'a> for Ident {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a> {
        TokenStreamPart::Tree(self.clone().into())
    }
}
impl<'a> ToTokenStreamPart<'a> for TokenTree {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a> {
        TokenStreamPart::Tree(self.clone().into())
    }
}
impl<'a> ToTokenStreamPart<'a> for TokenStream {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a> {
        TokenStreamPart::Stream(self.clone())
    }
}
impl<'a> ToTokenStreamPart<'a> for TokenStreamPart<'a> {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a> {
        self.clone()
    }
}
impl<'a, T> ToTokenStreamPart<'a> for Option<T> where T: ToTokenStreamPart<'a> {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a> {
        if let Some(x) = self {
            x.to_token_stream_part()
        } else {
            TokenStreamPart::None
        }
    }
}
impl<'a, T> ToTokenStreamPart<'a> for Vec<T> where T: ToTokenStreamPart<'a> {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a> {
        panic!()
        // TokenStreamPart::Stream(token_stream(&self.iter().map(|x| {
        //     x.to_token_stream_part();
        // }).collect::<Vec<_>>()))
    }
}
impl<'a> ToTokenStreamPart<'a> for &str {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a> {
        TokenStreamPart::FromStr(self.to_string())
    }
}
impl<'a> ToTokenStreamPart<'a> for String {
    fn to_token_stream_part(&self) -> TokenStreamPart<'a> {
        TokenStreamPart::FromStr(self.to_string())
    }
}

#[derive(Clone)]
pub enum TokenStreamPart<'a> {
    None,
    FromStr(String),
    Tree(TokenTree),
    Stream(TokenStream),
    IntoStream(&'a dyn ToTokenStream),
    // Dyn(&'a dyn ToTokenStreamPart<'a>)
}

pub fn token_stream<'d, 's: 'd, 'a: 'd>(parts: &[&'d dyn ToTokenStreamPart<'a>]) -> TokenStream {
    let mut tb = TokenBuilder::new();
    tb.add_parts(parts.into_iter().map(|p| p.to_token_stream_part()));
    tb.end()
}

pub fn joined_token_streams<'a, I, T, U>(iter: I, separator: U) -> TokenStream where I: IntoIterator<Item=T>, T: ToTokenStreamPart<'a>, U: ToTokenStreamPart<'a> {
    let mut tb = TokenBuilder::new();
    for x in iter {
        tb.add_parts(Some(x.to_token_stream_part()));
        tb.add_parts(Some(separator.to_token_stream_part()));
    }
    tb.end()
}


impl TokenBuilder {
    pub fn add_parts<'a>(&mut self, parts: impl IntoIterator<Item=TokenStreamPart<'a>>) {
        for part in parts {
            match part {
                TokenStreamPart::None => {}
                TokenStreamPart::FromStr(x) => { self.add(&x) }
                TokenStreamPart::Tree(x) => { self.extend(x) }
                TokenStreamPart::Stream(x) => { self.stream(x) }
                TokenStreamPart::IntoStream(x) => { self.stream(x.to_token_stream()) }
                // TokenStreamPart::Dyn(x) => {  self.add_parts(Some(x.to_token_stream_part())) }
            }
        }
    }
    pub fn add_part<'a>(&mut self, part: &dyn ToTokenStreamPart<'a>) {
        match part.to_token_stream_part() {
            TokenStreamPart::None => {}
            TokenStreamPart::FromStr(x) => { self.add(&x) }
            TokenStreamPart::Tree(x) => { self.extend(x) }
            TokenStreamPart::Stream(x) => { self.stream(x) }
            TokenStreamPart::IntoStream(x) => { self.stream(x.to_token_stream()) }
            // TokenStreamPart::Dyn(x) => {  self.add_parts(Some(x.to_token_stream_part())) }
        }
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
