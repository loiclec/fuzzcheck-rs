# Grammar-based string mutators

This module contains tools to write grammar-based string mutators. Here, I
describe the general approach for building such mutators.

First, we assume that all strings (that follow a grammar) can be represented as
an Abstract Syntax Tree (AST) of the form:
```rust
// defined in ast.rs
pub enum AST {
    Token(char),
    Sequence(Vec<AST>),
    Box(Box<AST>),
}
```
Second, given an `AST`, we can generate the string that corresponds to it
relatively easily. If we generate a string from an `AST`, we can also link each
AST node to a string range via an `ASTMapping`. For example, the AST:
```rust
Sequence {
    Token 'a'
    Box {
        Sequence {
            'b'
        }
    }
}
```
corresponds to the string `ab` and the `ASTMapping` is:
```rust
ASTMapping {
    index_range: "ab" // actually = (0 .. 2) but written here as "ab" for clarity
    content: Sequence {
        ASTMapping {
            index_range: "a",
            content: Token
        },
        ASTMapping {
            index_range: "b",
            content: Sequence {
                ASTMapping {
                    index_range: "b",
                    content: Token
                }
            }
        }
    }
}
```
The mapping is important to establish because it allows us to propagate 
mutations done on the AST to the string.

We can create an `ASTMutator` using normal fuzzcheck mutators, for example by
using the `#[make_mutator]` procedural macro (the choice of field mutators 
will be explained later):
```rust
#[make_mutator(name: ASTMutator, recursive: true, default: false)]
pub enum AST {
    Token(#[field_mutator(CharWithinRangeMutator)] char),
    Sequence(
        #[field_mutator(Either<
                    FixedLenVecMutator<AST, ASTMutator>,
                    VecMutator<AST, ASTMutator>,
                >)]
        Vec<AST>,
    ),
    Box(#[field_mutator(BoxMutator<AST, ASTMutator>)] Box<AST>),
}
```
This gives us an `ASTMutator` type. Like all mutators, `ASTMutator` operates on
`AST`s in-place. It mutates them, encodes the mutations into `UnmutateToken`s
that can be used to undo the mutations later on.

To build a grammar-based string mutator, we wrap the `ASTMutator` and make it 
mutate the *cache* of our mutator (of type AST), and then we propagate these 
cache mutations to the value (of type String) using `ASTMapping`.
```rust
// the code below is simplified to focus on the essential bits
struct GrammarBasedStringMutator {
    ast_mutator: ASTMutator
}
struct Cache {
    ast: AST,
    mapping: ASTMapping,
    cache: ASTMutator::Cache,
}
impl Mutator<String> for GrammarBasedStringMutator {
    type Cache = Cache;
    fn mutate(&self, value: &mut String, cache: &mut Self::Cache) -> Self::UnmutateToken {
        let unmutate_token = self.ast_mutator.mutate(&mut cache.ast, &mut cache.cache);
        cache.mapping.propagate_mutation(value, &cache.ast, &unmutate_token);
        unmutate_token
    }
}
```
We do essentially the same for `unmutate`. Using this technique, we get 
in-place grammar-based mutations of strings based on in-place mutations of
their corresponding ASTs.

There are a few more issues to solve. First, to generate arbitrary values, we do:
```rust
impl Mutator<String> for GrammarBasedStringMutator {
    fn arbitrary(&self, max_cplx: f64) -> (String, f64) {
        let (ast, complexity) = self.ast_mutator.arbitrary(max_cplx);
        let value = ast.generate_string();
        (value, complexity)
    }
}
```
So in particular, note that the complexity of a string is based not on its 
length but on the complexity of its corresponding AST.

Then, we want to make sure that the `ASTMutator` only ever produces values
that conform to a grammar. To explain how we do this, we first define what
a grammar is. It is, essentially:
```rust
enum Grammar {
    Literal(Range<char>),
    Alternation(Vec<Grammar>),
    Concatenation(Vec<Grammar>),
    Repetition(Grammar, Range<usize>),
    Recurse(Weak<Grammar>),
    End,
}
```
For example, the regular expression `a([0-9]|[a-z])+` corresponds to:
```rust
Concatenation {
    Literal 'a' ..= 'a'
    Repetition {
        Alternation {
            Literal '0' ..= '9'
            Literal 'a' ..= 'z'
        },
        1..MAX
    }
}
```
Note also the possibility of recursive grammars from the variant 
`Grammar::Recurse`.

To build an `ASTMutator` that can only ever produce `AST`s adhering to a given
grammar, we initialize it carefully using the right submutators.

For example, for a mutator that can only produce literals between 'a' and 'z',
we use:
```rust
ASTMutator::token(CharWithinRangeMutator::new('a' ..= 'z'))
```
For alternations, we use an `AlternationMutator`:
```rust
ASTMutator::box(AlternationMutator::new(vec![ mutator_1, mutator_2, ... ]))
```
For concatenation, we use a `FixedLenVecMutator`:
```rust
ASTMutator::sequence(FixedLenVecMutator::new(vec![ mutator_1, mutator_2, ... ]))
```
For repetition, we use a simple `VecMutator` and constrain its length:
```rust
ASTMutator::sequence(VecMutator::new(vec![ mutator_1, mutator_2, ... ], length_range))
```
For nested grammars, we use a `BoxMutator`:
```rust
ASTMutator::box(BoxMutator::new(mutator))
```
And for recursive grammars, we would use `RecursiveMutator`/`RecurToMutator` 
(not implemented yet):
```rust
RecursiveMutator::new(|self_| {
    ASTMutator::box(BoxMutator::new(RecurToMutator(self_.clone())))
})
```
Because this is error-prone, we provide a function that takes a `Grammar` and
returns the correct `ASTMutator`.
```rust
let mutator = ASTMutator::from_grammar(grammar);
```

Finally, we want our `GrammarBasedStringMutator` to be able to read values of 
type `String` directly, verify that they conform to the grammar, and produce 
the corresponding cache. This allows the mutator to read an input corpus of 
strings, without requiring the user to annotate each string with its 
corresponding `AST` and `ASTMapping`. This, in turns, makes it easier to use a
`GrammarBasedStringMutator` anywhere that a `Mutator<String>` could be used. 
For example, as the mutator of a struct field:
```rust
#[make_mutator(name: PersonMutator)]
struct Person {
    // hypothetical syntax
    #[field_mutator(GrammarBasedStringMutator = from_regex!("[0-9]{1,2}-[0-9]{1,2}") )]
    birthday: String,
    #[field_mutator(StringMutator)]
    name: String
}
```

This is what the `validate_value` method (in the `Mutator<T>`) trait is for. 
The problem is that to implement it, we need a correct parser that takes a 
`String` and returns a `Option<AST>`. In general, I'd like to leave the choice
of parser up to the user. But for now, a basic recursive descent parser that 
can also match regular expressions is provided. To validate the value, we do:
```rust
fn validate_value(&self, value: &String) -> Option<(Self::Cache, Self::MutationStep)> {
    let ast = parser::parse_from_grammar(value, self.grammar)?;       
    let (ast_mutator_cache, mutation_step) = self.ast_mutator.validate_value(&ast).unwrap();
    let (_, mapping) = ast.generate_string();
    let cache = Cache {
        ast,
        ast_mutator_cache,
        mapping,
    };
    Some((cache, mutation_step))
}
```
Note that the provided parser is slow, cannot handle left-recursive grammars 
and is vulnerable to catastrophic backtracking and stack overflows. But it
will work for well-formed grammars and medium-sized strings. I will improve it
over time.

I imagine there are cases where one cannot provide a parser. For example, if 
this string-based mutator is used to test a parser for the same grammar! 
It is possible to modify the solution proposed here such that no parser is 
necessary. The mutator would then read and produce values of type 
`(AST, String)`. I will explore that later.