mod tuples;

#[macro_use]
extern crate decent_synquote_alternative;

// make_basic_tuple_mutator!(2) {
//     (A, B, C, D, E, F, G, H, I, J)
// }

#[proc_macro]
pub fn make_basic_tuple_mutator(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    tuples::make_basic_tuple_mutator_impl(item.into()).into()
}
