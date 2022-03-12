# Test Function

The main function declared by `pulldown-cmark` is called `push_html`. It takes a markdown string as input
and generates an HTML string from it. Our test function will only verify that `push_html` never crashes.
We write it inside `src/lib.rs`.

```rust ignore
#[cfg(all(fuzzing, test))]
mod tests {
    use crate::{html, Parser};
    fn push_html_does_not_crash(md_string: &str) {
        let parser = Parser::new(md_string);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
    }
}
```

The most important step afterwards is to create a good mutator for generating
markdown strings. This is discussed in the next section.
