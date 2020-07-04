mod macro_lib;
mod macro_lib_test;

//mod struct_derive;

use proc_macro::{TokenStream, Delimiter};
use crate::macro_lib::*;

#[proc_macro_derive(HasDefaultMutator)]
pub fn derive_mutator(input: TokenStream) -> TokenStream {

    let mut parser = TokenParser::new(input);
    let mut tb = TokenBuilder::new();
    
    let vis = parser.eat_visibility();
    tb.stream(vis);

    let is_struct = parser.eat_ident("struct");
    if is_struct {
        return derive_struct_mutator(parser, tb);
    }

    tb.end()
}

fn derive_struct_mutator(mut parser: TokenParser, mut tb: TokenBuilder) -> TokenStream {
    let ident = parser.eat_any_ident();
    if ident.is_none() {
        return parser.unexpected();
    }
    let mut ident = ident.unwrap();
    ident.push_str("_derived");

    let generic = parser.eat_generic();
    let types = parser.eat_all_types();
    let where_clause = parser.eat_where_clause();

    tb.add("struct").ident(&ident).stream(generic);

    // Struct with unnamed fields
    if let Some(types) = &types {
        tb.push_group(Delimiter::Parenthesis);
        for ty in types {
           tb.stream(Some(ty.clone()));
           tb.add(",");
        }
        tb.pop_group(Delimiter::Parenthesis);
        tb.stream(where_clause);
        tb.add(";");
    } else if let Some(fields) = parser.eat_all_struct_fields(){ 
        tb.stream(where_clause);
        tb.push_group(Delimiter::Brace);
        for (field,ty) in fields {
            tb.ident(&field).add(":").stream(Some(ty)).add(",");
        }
        tb.pop_group(Delimiter::Brace);
    }
    else { return parser.unexpected() }

    let tokens = tb.end().clone();

    eprintln!("{}", tokens);

    tokens
}