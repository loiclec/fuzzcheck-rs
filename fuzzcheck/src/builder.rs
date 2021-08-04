use crate::fuzzer::{self, Fuzzer};
use fuzzcheck_common::arg::Arguments;
use fuzzcheck_common::arg::{
    options_parser, COMMAND_FUZZ, COMMAND_MINIFY_CORPUS, COMMAND_MINIFY_INPUT, CORPUS_SIZE_FLAG, INPUT_FILE_FLAG,
    IN_CORPUS_FLAG,
};
use fuzzcheck_traits::{Mutator, Serializer};
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::result::Result;

pub trait FuzzTestFunction<T: ?Sized, ImplId> {
    type NormalizedFunction: for<'a> Fn(&'a T) -> bool;
    fn test_function(self) -> Self::NormalizedFunction;
}

pub enum ReturnBool {}
pub enum ReturnVoid {}
pub enum ReturnResult {}

impl<T: ?Sized, F> FuzzTestFunction<T, ReturnBool> for F
where
    F: Fn(&T) -> bool,
{
    type NormalizedFunction = Self;

    fn test_function(self) -> Self::NormalizedFunction {
        self
    }
}
impl<T: ?Sized, F> FuzzTestFunction<T, ReturnVoid> for F
where
    F: Fn(&T),
{
    type NormalizedFunction = impl Fn(&T) -> bool;

    fn test_function(self) -> Self::NormalizedFunction {
        move |x| {
            self(x);
            true
        }
    }
}

impl<T: ?Sized, F, S, E> FuzzTestFunction<T, ReturnResult> for F
where
    F: Fn(&T) -> Result<E, S>,
{
    type NormalizedFunction = impl Fn(&T) -> bool;

    fn test_function(self) -> Self::NormalizedFunction {
        move |x| {
            let r = self(x);
            match r {
                Ok(_) => true,
                Err(_) => false,
            }
        }
    }
}

pub enum FuzzerBuilder {}

pub struct FuzzerBuilder0<T, F>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
{
    test_function: F,
    _phantom: PhantomData<T>,
}

pub struct FuzzerBuilder1<T, F, M, V>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
{
    test_function: F,
    mutator: M,
    _phantom: PhantomData<(*const T, V)>,
}

pub struct FuzzerBuilder2<T, F, M, V, S>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
{
    test_function: F,
    mutator: M,
    serializer: S,
    _phantom: PhantomData<(*const T, V)>,
}

pub struct FuzzerBuilder3<T, F, M, V, S>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
{
    test_function: F,
    mutator: M,
    serializer: S,
    arguments: Arguments,
    _phantom: PhantomData<(*const T, V)>,
}

impl FuzzerBuilder {
    pub fn test<T, F, TestFunctionKind>(test_function: F) -> FuzzerBuilder0<T, F::NormalizedFunction>
    where
        T: ?Sized,
        F: FuzzTestFunction<T, TestFunctionKind>,
    {
        FuzzerBuilder0 {
            test_function: test_function.test_function(),
            _phantom: PhantomData,
        }
    }
}

impl<T, F> FuzzerBuilder0<T, F>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
{
    pub fn mutator<M, V>(self, mutator: M) -> FuzzerBuilder1<T, F, M, V>
    where
        V: Clone + Borrow<T>,
        M: Mutator<V>,
    {
        FuzzerBuilder1 {
            test_function: self.test_function,
            mutator,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, M, V> FuzzerBuilder1<T, F, M, V>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
{
    pub fn serializer<S>(self, serializer: S) -> FuzzerBuilder2<T, F, M, V, S>
    where
        S: Serializer<Value = V>,
    {
        FuzzerBuilder2 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer,
            _phantom: PhantomData,
        }
    }
}
impl<T, F, M, V, S> FuzzerBuilder2<T, F, M, V, S>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
{
    pub fn arguments_from_cargo_fuzzcheck(self) -> FuzzerBuilder3<T, F, M, V, S> {
        let parser = options_parser();
        let mut help = format!(
            r#""
fuzzcheck <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    {fuzz}    Run the fuzz test
    {tmin}    Minify a crashing test input, requires --{input_file}
    {cmin}    Minify a corpus of test inputs, requires --{in_corpus}
"#,
            fuzz = COMMAND_FUZZ,
            tmin = COMMAND_MINIFY_INPUT,
            input_file = INPUT_FILE_FLAG,
            cmin = COMMAND_MINIFY_CORPUS,
            in_corpus = IN_CORPUS_FLAG,
        );
        help += parser.usage("").as_str();
        help += format!(
            r#""
## Examples:

fuzzcheck {fuzz}
    Launch the fuzzer with default options.

fuzzcheck {tmin} --{input_file} "artifacts/crash.json"

    Minify the test input defined in the file "artifacts/crash.json".
    It will put minified inputs in the folder artifacts/crash.minified/
    and name them {{complexity}}-{{hash}}.json. 
    For example, artifacts/crash.minified/4213--8cd7777109b57b8c.json
    is a minified input of complexity 42.13.

fuzzcheck {cmin} --{in_corpus} "fuzz-corpus" --{corpus_size} 25

    Minify the corpus defined by the folder "fuzz-corpus", which should
    contain JSON-encoded test inputs.
    It will remove files from that folder until only the 25 most important
    test inputs remain.
"#,
            fuzz = COMMAND_FUZZ,
            tmin = COMMAND_MINIFY_INPUT,
            input_file = INPUT_FILE_FLAG,
            cmin = COMMAND_MINIFY_CORPUS,
            in_corpus = IN_CORPUS_FLAG,
            corpus_size = CORPUS_SIZE_FLAG
        )
        .as_str();

        let arguments = std::env::var("FUZZCHECK_ARGS").unwrap();
        let arguments = arguments.split_ascii_whitespace().collect::<Vec<_>>();
        let arguments = match Arguments::from_parser(&parser, &arguments) {
            Ok(r) => r,
            Err(e) => {
                println!("{}\n\n{}", e, help);
                std::process::exit(1);
            }
        };

        FuzzerBuilder3 {
            test_function: self.test_function,
            mutator: self.mutator,
            serializer: self.serializer,
            arguments,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, M, V, S> FuzzerBuilder3<T, F, M, V, S>
where
    T: ?Sized,
    F: Fn(&T) -> bool,
    V: Clone + Borrow<T>,
    M: Mutator<V>,
    S: Serializer<Value = V>,
    Fuzzer<V, T, F, M, S>: 'static,
{
    pub fn launch(self) -> Result<(), std::io::Error> {
        let FuzzerBuilder3 {
            test_function,
            mutator,
            serializer,
            arguments,
            _phantom,
        } = self;

        fuzzer::launch(test_function, mutator, serializer, arguments)?;
        Ok(())
    }
}
