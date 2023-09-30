// This file is derived from the shtring crate, which is licensed under MIT
// and available at https://github.com/Spanfile/shtring
/*
MIT License

Copyright (c) 2020 Spanfile

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
 */

use std::iter::Peekable;
use std::str::CharIndices;

pub(crate) struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

pub(crate) enum Token<'a> {
    Word(&'a str),
    Whitespace(&'a str),
    SingleQuote,
    DoubleQuote,
    Escape(&'a str),
}

impl<'a> Iterator for Lexer<'a> {
    type Item = (usize, Token<'a>);
    #[coverage(off)]
    fn next(&mut self) -> Option<Self::Item> {
        match self.chars.next() {
            Some((idx, chr)) => match chr {
                '\'' => Some((idx, Token::SingleQuote)),
                '"' => Some((idx, Token::DoubleQuote)),
                '\\' => match self.chars.next() {
                    Some((cont, _)) => Some((idx, Token::Escape(&self.input[idx..cont + 1]))),
                    None => panic!(),
                },
                c if c.is_whitespace() => {
                    let mut end = idx;
                    loop {
                        match self.chars.peek() {
                            Some((cont, c)) if c.is_whitespace() => end = *cont,
                            _ => break,
                        }
                        self.chars.next();
                    }
                    Some((idx, Token::Whitespace(&self.input[idx..end + 1])))
                }
                _ => {
                    let mut end = idx;
                    loop {
                        match self.chars.peek() {
                            Some((cont, c)) if is_word_character(*c) => end = *cont,
                            _ => break,
                        }
                        self.chars.next();
                    }
                    Some((idx, Token::Word(&self.input[idx..end + 1])))
                }
            },
            None => None,
        }
    }
}
#[coverage(off)]
fn is_word_character(c: char) -> bool {
    c != '\'' && c != '"' && c != '\\' && !c.is_whitespace()
}
/**
Split an input string into arguments by whitespace such that text between matching quotes is combined into a single argument. Additionally, single character escapes are supported and ignored where applicable. Will panic on invalid inputs.
*/
#[coverage(off)]
pub fn split_string_by_whitespace(input: &str) -> Vec<&str> {
    let mut lexer = Lexer {
        input,
        chars: input.char_indices().peekable(),
    };
    let mut result = vec![];
    while let Some((idx, token)) = lexer.next() {
        match token {
            Token::Whitespace(_) => continue,
            Token::Word(_) | Token::Escape(_) => loop {
                match lexer.next() {
                    Some((cont, Token::Whitespace(_))) => {
                        result.push(&input[idx..cont]);
                        break;
                    }
                    Some((_, Token::Word(_) | Token::Escape(_))) => continue,
                    Some((_, Token::SingleQuote | Token::DoubleQuote)) => {
                        panic!()
                    }
                    None => {
                        result.push(&input[idx..]);
                        break;
                    }
                }
            },
            Token::SingleQuote | Token::DoubleQuote => loop {
                match lexer.next() {
                    Some((cont, quote))
                        if matches!(
                            (&quote, &token),
                            (Token::SingleQuote, Token::SingleQuote) | (Token::DoubleQuote, Token::DoubleQuote)
                        ) =>
                    {
                        result.push(&input[idx + 1..cont]);
                        break;
                    }
                    Some((_, _)) => continue,
                    None => panic!(),
                }
            },
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::split_string_by_whitespace;
    #[test]
    #[coverage(off)]
    fn test1() {
        let s = "hello world";
        println!("{:?}", split_string_by_whitespace(s));

        let s = "hello 'world bye'";
        println!("{:?}", split_string_by_whitespace(s));

        let s = "hello \\'world bye";
        println!("{:?}", split_string_by_whitespace(s));

        let s = "hello \"world \\\"bye\"";
        println!("{:?}", split_string_by_whitespace(s));

        let s = "\"hello \" world \\\"bye \"\"";
        println!("{:?}", split_string_by_whitespace(s));
    }
}
