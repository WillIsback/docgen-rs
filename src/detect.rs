use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

pub fn python_has_missing_docstrings(source: &str, force: bool) -> bool {
    let mut parser = Parser::new();
    let lang: Language = tree_sitter_python::LANGUAGE.into();
    parser.set_language(&lang).expect("valid Python grammar");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return false,
    };
    let query = Query::new(&lang, "[(function_definition) (class_definition)] @def")
        .expect("valid query");
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let has_docstring = capture
                .node
                .child_by_field_name("body")
                .and_then(|b| b.named_child(0))
                .map(|stmt| {
                    stmt.kind() == "expression_statement"
                        && stmt
                            .named_child(0)
                            .map(|e| e.kind() == "string")
                            .unwrap_or(false)
                })
                .unwrap_or(false);
            if !has_docstring || force {
                return true;
            }
        }
    }
    false
}

pub fn ts_has_missing_docstrings(source: &str, force: bool) -> bool {
    let mut parser = Parser::new();
    let lang: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    parser.set_language(&lang).expect("valid TS grammar");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return false,
    };
    let query = Query::new(
        &lang,
        "[(function_declaration) (method_definition) (class_declaration)] @def",
    )
    .expect("valid query");
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let start = capture.node.start_byte();
            let has_jsdoc = source[..start].trim_end().ends_with("*/");
            if !has_jsdoc || force {
                return true;
            }
        }
    }
    false
}

pub fn needs_docstrings(file: &Path, source: &str, force: bool) -> bool {
    match file.extension().and_then(|e| e.to_str()) {
        Some("py") => python_has_missing_docstrings(source, force),
        Some("ts") | Some("tsx") => ts_has_missing_docstrings(source, force),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn py_fn_without_docstring() {
        assert!(python_has_missing_docstrings("def foo():\n    pass\n", false));
    }

    #[test]
    fn py_fn_with_docstring_not_flagged() {
        assert!(!python_has_missing_docstrings(
            "def foo():\n    \"\"\"Doc.\"\"\"\n    pass\n",
            false
        ));
    }

    #[test]
    fn py_class_without_docstring() {
        assert!(python_has_missing_docstrings("class Foo:\n    pass\n", false));
    }

    #[test]
    fn py_force_flags_documented() {
        assert!(python_has_missing_docstrings(
            "def foo():\n    \"\"\"Exists.\"\"\"\n    pass\n",
            true
        ));
    }

    #[test]
    fn ts_fn_without_jsdoc() {
        assert!(ts_has_missing_docstrings("function foo(): void {}\n", false));
    }

    #[test]
    fn ts_fn_with_jsdoc_not_flagged() {
        assert!(!ts_has_missing_docstrings(
            "/** Does something. */\nfunction foo(): void {}\n",
            false
        ));
    }

    #[test]
    fn ts_class_without_jsdoc() {
        assert!(ts_has_missing_docstrings("class Bar {}\n", false));
    }

    #[test]
    fn ts_force_flags_documented() {
        assert!(ts_has_missing_docstrings(
            "/** Exists. */\nfunction foo(): void {}\n",
            true
        ));
    }

    #[test]
    fn dispatches_by_extension() {
        assert!(needs_docstrings(
            Path::new("a.py"),
            "def foo():\n    pass\n",
            false
        ));
        assert!(needs_docstrings(
            Path::new("a.ts"),
            "function foo(): void {}\n",
            false
        ));
        assert!(!needs_docstrings(Path::new("a.md"), "# Hello", false));
    }
}
