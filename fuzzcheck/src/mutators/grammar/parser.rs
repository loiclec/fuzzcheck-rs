use std::ops::RangeInclusive;
use std::{ops::Range, rc::Rc};

use super::ast::AST;
use super::grammar::Grammar::{Alternation, Concatenation, Literal, Recurse, Recursive, Repetition};
use super::{grammar::Grammar, list::List};

#[no_coverage]
pub fn parse_from_grammar(string: &str, grammar: Rc<Grammar>) -> Option<AST> {
    let mut parser = grammar_parser(string, 0, grammar);
    while let Some((ast, idx)) = parser() {
        if idx == string.len() {
            return Some(ast);
        }
    }
    None
}

#[no_coverage]
fn grammar_parser<'a>(
    string: &'a str,
    idx: usize,
    grammar: Rc<Grammar>,
) -> Box<dyn 'a + FnMut() -> Option<(AST, usize)>> {
    match grammar.as_ref() {
        Literal(l) => atom_parser(string, idx, l.clone()),
        Repetition(g, range) => repetition_parser(string, idx, g.clone(), range.clone()),
        Alternation(gs) => alternation_parser(string, idx, Rc::new(List::from_slice(gs))),
        Recurse(grammar) => recurse_parser(string, idx, grammar.upgrade().unwrap().clone()),
        Recursive(inner_grammar) => {
            // the grammar might be the only strong reference to the recursive grammar,
            // and we just deconstructed it, so it might be destroyed
            // so we clone it and store it in the parser (that is, in the closure)
            let grammar_long_lived = grammar.clone();
            let mut inner_parser = grammar_parser(string, idx, inner_grammar.clone());
            Box::new(
                #[no_coverage]
                move || {
                    let _x = grammar_long_lived.as_ref(); // do anything here, to make sure the closure captures grammar_long_lived
                    inner_parser()
                },
            )
        }
        Concatenation(gs) => concatenation_parser(string, idx, gs),
    }
}

#[no_coverage]
fn recurse_parser<'a>(
    string: &'a str,
    idx: usize,
    grammar: Rc<Grammar>,
) -> Box<dyn 'a + FnMut() -> Option<(AST, usize)>> {
    let mut parser = grammar_parser(string, idx, grammar);
    Box::new(
        #[no_coverage]
        move || {
            parser().map(
                #[no_coverage]
                |(ast, idx)| (AST::Box(Box::new(ast)), idx),
            )
        },
    )
}

#[no_coverage]
fn atom_parser<'a>(
    string: &'a str,
    idx: usize,
    range_chars: Vec<RangeInclusive<char>>,
) -> Box<dyn 'a + FnMut() -> Option<(AST, usize)>> {
    let mut end = false;
    Box::new(
        #[no_coverage]
        move || {
            if end || idx >= string.len() {
                None
            } else {
                let string = &string[idx..];
                let mut chars = string.chars();
                if let Some(char) = chars.next() {
                    if range_chars.iter().find(|r| r.contains(&char)).is_some() {
                        end = true;
                        Some((AST::Token(char), idx + char.len_utf8()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        },
    )
}

#[no_coverage]
fn concatenation_parser<'a>(
    string: &'a str,
    idx: usize,
    gs: &[Rc<Grammar>],
) -> Box<dyn 'a + FnMut() -> Option<(AST, usize)>> {
    let mut rec = concatenation_parser_rec(string, idx, Rc::new(List::from_slice(gs)), Rc::new(List::Empty));
    Box::new(
        #[no_coverage]
        move || {
            let (asts, idx) = rec()?;
            let mut asts = asts.to_vec();
            asts.reverse();
            Some((AST::Sequence(asts), idx))
        },
    )
}

#[no_coverage]
fn concatenation_parser_rec<'a>(
    string: &'a str,
    idx: usize,
    gs: Rc<List<Rc<Grammar>>>,
    current_asts: Rc<List<AST>>,
) -> Box<dyn 'a + FnMut() -> Option<(Rc<List<AST>>, usize)>> {
    let mut current_grammar_parser: Option<(Box<dyn FnMut() -> Option<(AST, usize)>>, Rc<List<Rc<Grammar>>>)> = None;
    let mut rest_concatenation_parser: Option<Box<dyn FnMut() -> Option<(Rc<List<AST>>, usize)>>> = None;
    let mut last = false;
    Box::new(
        #[no_coverage]
        move || {
            if last {
                return None;
            }
            'main: loop {
                if let Some(rec_parser) = &mut rest_concatenation_parser {
                    if let Some(result) = rec_parser() {
                        return Some(result);
                    } else {
                        rest_concatenation_parser = None;
                        continue 'main;
                    }
                } else if let Some((grammar_parser, rest)) = &mut current_grammar_parser {
                    if let Some((next_ast, next_idx)) = grammar_parser() {
                        rest_concatenation_parser = Some(concatenation_parser_rec(
                            string,
                            next_idx,
                            rest.clone(),
                            current_asts.prepend(next_ast),
                        ));
                        continue;
                    } else {
                        // the grammar doesn't match the string, so the whole concatenation must be invalid
                        return None;
                    }
                } else {
                    let deconstructed = match gs.as_ref() {
                        List::Empty => None,
                        List::Cons(g, rest) => Some((g.clone(), rest.clone())),
                    };
                    match deconstructed {
                        None => {
                            if current_asts.is_empty() {
                                return None;
                            } else {
                                last = true; // next one will return None
                                return Some((current_asts.clone(), idx));
                            }
                        }
                        Some((r, rest)) => {
                            current_grammar_parser = Some((grammar_parser(string, idx, r), rest));
                            continue 'main;
                        }
                    }
                }
            }
        },
    )
}

#[no_coverage]
fn repetition_parser<'a>(
    string: &'a str,
    idx: usize,
    g: Rc<Grammar>,
    range: Range<usize>,
) -> Box<dyn 'a + FnMut() -> Option<(AST, usize)>> {
    let mut rec = repetition_parser_rec(string, idx, g, 0, range, Rc::new(List::Empty));
    Box::new(
        #[no_coverage]
        move || {
            let (asts, idx) = rec()?;
            let mut asts = asts.to_vec();
            asts.reverse();
            Some((AST::Sequence(asts), idx))
        },
    )
}

#[no_coverage]
fn repetition_parser_rec<'a>(
    string: &'a str,
    idx: usize,
    g: Rc<Grammar>,
    count: usize,
    range: Range<usize>,
    current_asts: Rc<List<AST>>,
) -> Box<dyn 'a + FnMut() -> Option<(Rc<List<AST>>, usize)>> {
    let mut current_grammar_parser: Option<Box<dyn FnMut() -> Option<(AST, usize)>>> = None;
    let mut rest_concatenation_parser: Option<Box<dyn FnMut() -> Option<(Rc<List<AST>>, usize)>>> = None;
    let mut produced_first_output = false;
    Box::new(
        #[no_coverage]
        move || {
            'main: loop {
                if let Some(rec_parser) = &mut rest_concatenation_parser {
                    if let Some(result) = rec_parser() {
                        return Some(result);
                    } else {
                        rest_concatenation_parser = None;
                        continue 'main;
                    }
                } else if let Some(grammar_parser) = &mut current_grammar_parser {
                    if let Some((next_ast, next_idx)) = grammar_parser() {
                        rest_concatenation_parser = Some(repetition_parser_rec(
                            string,
                            next_idx,
                            g.clone(),
                            count + 1,
                            range.clone(),
                            current_asts.prepend(next_ast),
                        ));
                        continue;
                    } else {
                        // the grammar doesn't match the string, so the whole concatenation must be invalid
                        return None;
                    }
                } else {
                    if count >= range.end {
                        return None;
                    }
                    if !produced_first_output && range.contains(&count) {
                        produced_first_output = true;
                        return Some((current_asts.clone(), idx));
                    }
                    current_grammar_parser = Some(grammar_parser(string, idx, g.clone()));
                    continue 'main;
                }
            }
        },
    )
}

#[no_coverage]
fn alternation_parser<'a>(
    string: &'a str,
    idx: usize,
    rs: Rc<List<Rc<Grammar>>>,
) -> Box<dyn 'a + FnMut() -> Option<(AST, usize)>> {
    let mut rs_iter = rs.iter();
    let mut parser: Option<Box<dyn FnMut() -> Option<(AST, usize)>>> = None;
    Box::new(
        #[no_coverage]
        move || 'main: loop {
            if let Some(p) = &mut parser {
                if let Some((ast, idx)) = p() {
                    let ast = AST::Box(Box::new(ast));
                    return Some((ast, idx));
                } else {
                    parser = None;
                    continue 'main;
                }
            }
            if let Some(grammar) = rs_iter.next() {
                parser = Some(grammar_parser(string, idx, grammar));
                continue 'main;
            } else {
                return None;
            }
        },
    )
}

// #[no_coverage] fn parse_end<'a>(string: &'a str, idx: usize) -> Box<dyn 'a + FnMut() -> Option<(AST, usize)>> {
//     let mut end = false;
//     Box::new(move || {
//         if !end && idx == string.len() {
//             end = true;
//             return Some((AST::Sequence(vec![]), idx));
//         } else {
//             return None;
//         }
//     })
// }

#[cfg(test)]
mod tests {
    use crate::mutators::grammar::{
        alternation, concatenation, literal, literal_range, recurse, recursive, repetition,
    };

    #[test]
    #[no_coverage]
    fn test_atom() {
        let grammar = literal_range('a'..='c');
        for string in ["a", "b", "c", "d"] {
            let mut parser = super::grammar_parser(string, 0, grammar.clone());
            while let Some((ast, _)) = parser() {
                println!("{:?}", ast);
            }
        }
    }

    #[test]
    #[no_coverage]
    fn test_alternation() {
        let grammar = alternation([
            literal_range('a'..='c'),
            literal_range('d'..='g'),
            literal_range('y'..='z'),
        ]);
        for string in ["a", "b", "e", "y", "i"] {
            let mut parser = super::grammar_parser(string, 0, grammar.clone());
            while let Some((ast, _)) = parser() {
                println!("{:?}", ast);
            }
        }
    }
    #[test]
    #[no_coverage]
    fn test_concatenation() {
        let grammar = concatenation([
            literal_range('a'..='c'),
            literal_range('d'..='g'),
            literal_range('y'..='z'),
        ]);
        for string in ["a", "ad", "ady", "bfz", "adyg"] {
            println!("results for {}", string);
            let mut parser = super::grammar_parser(string, 0, grammar.clone());
            while let Some((ast, _)) = parser() {
                println!("{:?}", ast);
            }
        }
    }
    #[test]
    #[no_coverage]
    fn test_end() {
        let grammar = concatenation([literal_range('a'..='c'), literal_range('d'..='g')]);
        for string in ["a", "ad", "ady", "bfz"] {
            println!("results for {}", string);
            let mut parser = super::grammar_parser(string, 0, grammar.clone());
            while let Some((ast, _)) = parser() {
                println!("{:?}", ast);
            }
        }
    }
    #[test]
    #[no_coverage]
    fn test_repetition() {
        let grammar = concatenation([repetition(
            concatenation([literal_range('a'..='c'), literal_range('d'..='g')]),
            0..3,
        )]);
        for string in ["", "a", "ad", "adad", "adadad"] {
            println!("results for {}", string);
            let mut parser = super::grammar_parser(string, 0, grammar.clone());
            while let Some((ast, _)) = parser() {
                println!("{:?}", ast);
            }
        }
    }

    #[test]
    #[no_coverage]
    fn test_recurse() {
        let main_rule = recursive(|grammar| {
            let letter = literal_range('a'..='z');
            let space = repetition(literal_range(' '..=' '), 0..10);
            let bar = literal_range('|'..='|');
            alternation([
                letter.clone(),
                concatenation([letter, space.clone(), bar, space, recurse(grammar)]),
            ])
        });

        let grammar = concatenation([main_rule]);

        let string = "a|a";
        let mut parser = super::grammar_parser(string, 0, grammar.clone());
        while let Some((ast, _)) = parser() {
            println!("{:?}", ast);
        }

        for string in ["a", "a|a", "a|a|a", ""] {
            println!("results for {}", string);
            let mut parser = super::grammar_parser(string, 0, grammar.clone());
            while let Some((ast, _)) = parser() {
                println!("{:?}", ast);
            }
        }
    }

    #[test]
    #[no_coverage]
    fn test_recurse_2() {
        let grammar = recursive(|g| {
            concatenation([
                literal_range('('..='('),
                alternation([literal_range('a'..='b'), recurse(g)]),
                literal_range(')'..=')'),
            ])
        });
        let string = "(((b)))";
        let mut parser = super::grammar_parser(string, 0, grammar);
        while let Some((ast, _)) = parser() {
            println!("{:?}", ast);
        }
    }

    // #[test]
    // #[no_coverage] fn test_recurse_2() {
    //     // this one overflows the stack!
    //     // here, as a mitigation, I could set a recursion limit, every time a recursive grammar
    //     // is parsed, the recursion limit goes down to 1
    //     let main_rule = Rc::new_cyclic(|grammar| {
    //         let letter = literal_range('a'..='z');
    //         let space = repetition(literal_range(' '..=' '), 0..10);
    //         let bar = literal_range('|'..='|');
    //         alternation([
    //             letter.clone(),
    //             concatenation([
    //                 recurse(grammar),
    //                 space.clone(),
    //                 bar,
    //                 space,
    //                 recurse(grammar),
    //             ]),
    //         ])
    //     });

    //     let grammar = concatenation([Grammar::shared(&main_rule)]);

    //     for string in ["a", "a | a"] {
    //         println!("results for {}", string);
    //         let mut parser = super::grammar_parser(string, 0, grammar.clone());
    //         while let Some((ast, _)) = parser() {
    //             println!("{:?}", ast);
    //         }
    //     }
    // }

    #[test]
    #[no_coverage]
    fn test_complex() {
        let grammar = concatenation([recursive(|rule| {
            let tick = literal_range('\''..='\'');
            let digit = literal_range('0'..='9');
            let number = repetition(digit.clone(), 1..10); // no more than 9 digits
            let character = alternation([literal_range('a'..='z'), digit, literal_range('_'..='_')]);
            let char_literal = alternation([/* char */ concatenation([tick.clone(), character, tick])]);

            let repetition_mark = alternation([
                literal('*'),
                literal('?'),
                literal('+'),
                concatenation([
                    literal('{'),
                    number.clone(),
                    repetition(concatenation([literal(','), number]), 0..=1),
                    literal('}'),
                ]),
            ]);
            let group = concatenation([literal('('), recurse(rule), literal(')')]);

            let literal_or_group = alternation([char_literal.clone(), group]);

            let rep = concatenation([literal_or_group.clone(), repetition_mark]);

            let alt = concatenation([literal_or_group.clone(), literal('|'), literal_or_group]);
            alternation([char_literal, rep, alt])
        })]);
        let string = "((('a'|'b')|'b')*)|('a'+)";
        let mut parser = super::grammar_parser(string, 0, grammar);
        while let Some((ast, _)) = parser() {
            println!("{:?}", ast);
        }
    }
}
