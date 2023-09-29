use crate::mutators::map::MapMutator;
use crate::{DefaultMutator, Mutator};

/// The default mutator for strings. It is not very good and will be replaced by a different
/// one in the future.
///
/// Construct it with:
/// ```rust
/// use fuzzcheck::DefaultMutator;
///
/// let m = String::default_mutator();
/// // or:
/// use fuzzcheck::mutators::string::string_mutator;
///
/// let m = string_mutator();
/// ```
pub type StringMutator = impl Mutator<String>;

#[coverage(off)]
pub fn string_mutator() -> StringMutator {
    MapMutator::new(
        // the base mutator produces values of type Vector<u8>
        <Vec<u8>>::default_mutator(),
        // the parse function: given a string, how can I get a vector?
        #[coverage(off)]
        |string: &String| Some(string.as_bytes().to_vec()),
        // the map function: how can I get a string from a vector?
        #[coverage(off)]
        |xs| String::from_utf8_lossy(xs).to_string(),
        // the complexity function
        #[coverage(off)]
        |value, _cplx| (value.as_bytes().len() * 8) as f64,
    )
}
impl DefaultMutator for String {
    type Mutator = StringMutator;

    #[coverage(off)]
    fn default_mutator() -> Self::Mutator {
        string_mutator()
    }
}
