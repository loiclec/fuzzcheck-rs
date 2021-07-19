pub mod ast;
mod grammar;
mod list;
mod mapping;
pub mod parser;

#[macro_export]
macro_rules! literal {
    ($c:expr) => {
        Grammar::literal($c)
    };
    ($($a:expr),*) => {
        Grammar::alternation([$(Grammar::literal($a)),*])
    };
}
#[macro_export]
macro_rules! concatenation {
    ($($gsm:expr),*) => {
        Grammar::concatenation([
            $($gsm),*
        ])
    }
    ;
}
#[macro_export]
macro_rules! alternation {
    ($($gsm:expr),*) => {
        Grammar::alternation([
            $($gsm),*
        ])
    }
    ;
}
