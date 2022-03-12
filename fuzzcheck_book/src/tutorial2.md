# Tutorial 2: pulldown-cmark

In this tutorial, we clone the popular `pulldown-cmark` crate and find real bugs in it.

We will start by creating a **grammar-based mutator**. From the generated syntax trees, 
we generate strings and then ask `pulldown-cmark` to parse those strings. 

To get started, go to a new folder and clone the `pulldown-cmark` repository:
```
git clone https://github.com/raphlinus/pulldown-cmark.git 
```
and then checkout the repository to its state from 18 September 2021:
```
cd pulldown-cmark
git checkout -b fuzz 5088b21d09ef94b424c4d852db7648c9c94fb630
```

> Note that grammar-based mutators are only available when the `grammar_mutator` feature is enabled. Creating grammars from regular expressions is only possible when the `regex_grammar` feature is enabled. Both of these featuress are enabled by default.