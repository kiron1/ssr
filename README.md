# Structured Search Replace (SSR)

Search your source tree using a [Tree-Sitter Query][ts-query] and apply
replacemnts scripted in [Rhai][rhai] and print out the change as a patch file.

## Examples

Find all `dbg!(...)` macros are in the current source tree (unlike `grep` this
will not find comment out ones from document examples):

```sh
ssr search --language rust --query '(macro_invocation macro: (identifier) @m (#eq? @m "dbg"))'
```

# Writing queries

See [ts-query][Tree-Sitter Query Syntax] for the explaination of the query
syntax. For prototyping the query a good place is the [ts-playground][playground
of Tree-Sitter].

# Writing replacement scripts

To provide a high degree of flexibility in the replacement expressions the
[rhai][Rhai] scripting language is used which also offers an online
[rhai-playground].

The Rhai runtime is extended of a `document` object whith an `edit` method to
modify the current document. Matches from the `--query` are accessible via the
`found` object. The `--replacement` is called for every match.

For example to replace all `dbg!(...)` macros call with a call to `println!`
use:

```sh
ssr replace --language rust \
  --query '((expression_statement ((macro_invocation macro: (identifier) @m (#eq? @m "dbg")))) @exp)' \
  --replacement 'for m in found.captures { if.name == "exp { document.edit(m.range, "println!"); } }'
```

[ts-query]: https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax "Tree-Sitter Query Syntax"
[ts-playground]: https://tree-sitter.github.io/tree-sitter/playground "Tree-Sitter Playground"
[rhai]: https://rhai.rs "Rhai"
[rhai-playground]: https://rhai.rs/playground/stable/ "Rhai Playground"
