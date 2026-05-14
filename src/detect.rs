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

const ATOMIC_TAGS: &[&str] = &[
    "div", "span", "p", "a", "button", "input", "img", "br", "hr",
    "svg", "ul", "ol", "li", "table", "tr", "td", "th", "form", "label",
    "header", "footer", "nav", "section", "article", "aside", "main",
    "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li",
];

const ATOMIC_UI_COMPONENTS: &[&str] = &[
    "Button", "Input", "Icon", "Image", "Link", "Text", "Span", "Badge", "Avatar"
];

#[derive(Debug)]
pub struct ContainerCandidate {
    pub component_name: String,
    pub return_line: u32,
    pub element_type: String,
    pub children: Vec<String>,
}

pub fn tsx_get_container_candidates(source: &str) -> Vec<ContainerCandidate> {
    let mut parser = Parser::new();
    let lang: Language = tree_sitter_typescript::LANGUAGE_TSX.into();
    parser.set_language(&lang).expect("valid TSX grammar");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return vec![],
    };
    
    let mut candidates = Vec::new();
    
    let func_query = Query::new(
        &lang,
        "(function_declaration) @func"
    ).expect("valid query");
    
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&func_query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let func_node = capture.node;
            let name_node = func_node.child_by_field_name("name");
            let name = match name_node.and_then(|n| n.utf8_text(source.as_bytes()).ok()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let return_node = find_direct_return(&func_node, source);
            if let Some(rn) = return_node {
                let container = extract_container_from_return(&rn, source, &name);
                if let Some(c) = container {
                    candidates.push(c);
                }
            }
        }
    }
    
    let arrow_query = Query::new(
        &lang,
        "(variable_declarator) @decl"
    ).expect("valid query");
    
    let mut matches = cursor.matches(&arrow_query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let decl = capture.node;
            let name_node = decl.child_by_field_name("name");
            let name = match name_node.and_then(|n| n.utf8_text(source.as_bytes()).ok()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if let Some(init) = decl.child_by_field_name("init") {
                if init.kind() == "arrow_function" {
                    if let Some(return_node) = find_arrow_return(&init, source) {
                        if let Some(container) = extract_container_from_return(&return_node, source, &name) {
                            candidates.push(container);
                        }
                    }
                }
            }
        }
    }
    
    candidates
}

fn find_direct_return<'tree>(func_node: &'tree tree_sitter::Node, _source: &str) -> Option<tree_sitter::Node<'tree>> {
    let body = func_node.child_by_field_name("body")?;
    
    if body.kind() == "block" || body.kind() == "statement_block" {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "return_statement" {
                let mut ret_cursor = child.walk();
                for ret_child in child.children(&mut ret_cursor) {
                    if ret_child.kind() == "jsx_element" || ret_child.kind() == "jsx_fragment" {
                        return Some(ret_child);
                    }
                    if ret_child.kind() == "parenthesized_expression" {
                        let mut paren_cursor = ret_child.walk();
                        for inner in ret_child.children(&mut paren_cursor) {
                            if inner.kind() == "jsx_element" || inner.kind() == "jsx_fragment" {
                                return Some(inner);
                            }
                        }
                    }
                }
            }
        }
    }
    
    if body.kind() == "jsx_element" || body.kind() == "jsx_fragment" {
        return Some(body);
    }
    
    None
}

fn find_arrow_return<'tree>(arrow_node: &'tree tree_sitter::Node, _source: &str) -> Option<tree_sitter::Node<'tree>> {
    let body = arrow_node.child_by_field_name("body")?;
    
    if body.kind() == "jsx_element" || body.kind() == "jsx_fragment" {
        return Some(body);
    }
    
    if body.kind() == "block" {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "return_statement" {
                return child.child_by_field_name("argument");
            }
        }
    }
    
    None
}

fn extract_container_from_return(return_node: &tree_sitter::Node, source: &str, func_name: &str) -> Option<ContainerCandidate> {
    let tag_name = get_jsx_element_name(return_node, source)?;
    
    if ATOMIC_TAGS.contains(&tag_name.as_str()) {
        return None;
    }
    
    if ATOMIC_UI_COMPONENTS.contains(&tag_name.as_str()) {
        return None;
    }
    
    let children = get_jsx_children(return_node, source);
    if children.len() < 2 {
        return None;
    }
    
    Some(ContainerCandidate {
        component_name: func_name.to_string(),
        return_line: return_node.start_position().row as u32 + 1,
        element_type: tag_name,
        children,
    })
}

fn get_jsx_element_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == "jsx_opening_element" {
            for inner in child.children(&mut child.walk()) {
                if inner.kind() == "identifier" {
                    return inner.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

fn get_jsx_children(node: &tree_sitter::Node, source: &str) -> Vec<String> {
    let mut children = Vec::new();
    
    for child in node.children(&mut node.walk()) {
        if child.kind() == "jsx_self_closing_element" {
            let mut name = None;
            for inner in child.children(&mut child.walk()) {
                if inner.kind() == "identifier" {
                    name = inner.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                    break;
                }
            }
            if let Some(n) = name {
                children.push(n);
            }
        }
    }
    
    children
}

fn get_jsx_self_closing_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "jsx_opening_element" {
                for j in 0..child.child_count() {
                    if let Some(name_child) = child.child(j) {
                        if name_child.kind() == "identifier" || name_child.kind() == "member_expression" {
                            return name_child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                        }
                    }
                }
            }
        }
    }
    None
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

    #[test]
    fn test_container_candidates_direct_return() {
        let source = r#"
function DashboardPage() {
  return (
    <Layout>
      <Sidebar />
      <Content />
      <Actions />
    </Layout>
  );
}
"#;
        let candidates = tsx_get_container_candidates(source);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].component_name, "DashboardPage");
        assert_eq!(candidates[0].element_type, "Layout");
        assert_eq!(candidates[0].children, vec!["Sidebar", "Content", "Actions"]);
    }

    #[test]
    fn test_container_atomic_component_skipped() {
        let source = r#"
function Page() {
  return (
    <div>
      <Button />
      <Input />
    </div>
  );
}
"#;
        let candidates = tsx_get_container_candidates(source);
        assert_eq!(candidates.len(), 0, "Should skip atomic components");
    }

    #[test]
    fn test_container_single_child_skipped() {
        let source = r#"
function Page() {
  return (
    <Layout>
      <Content />
    </Layout>
  );
}
"#;
        let candidates = tsx_get_container_candidates(source);
        assert_eq!(candidates.len(), 0, "Should skip single child containers");
    }
}
