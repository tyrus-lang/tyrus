use super::*;

#[test]
fn extracts_summary_and_param_tags() {
    let source = r#"
/** Greets the caller by name.
 *
 * @param name Person to greet
 * @returns void
 */
const greet = (name: string): void => {};
"#;
    let docs = extract_doc_comments(source);
    assert_eq!(docs.len(), 1);
    let doc = &docs[0];
    assert_eq!(doc.summary, "Greets the caller by name.");
    assert_eq!(
        doc.params.get("name").map(String::as_str),
        Some("Person to greet")
    );
    assert_eq!(doc.returns.as_deref(), Some("void"));
}

#[test]
fn ignores_doc_like_sequences_inside_strings() {
    let source = r#"const marker: string = "/** not a doc */";"#;
    assert!(extract_doc_comments(source).is_empty());
}

#[test]
fn attaches_doc_to_following_declaration() {
    let source = r#"/**
 * Adds two numbers.
 */
const add = (left: i32, right: i32): i32 => left + right;"#;
    let docs = extract_doc_comments(source);
    let decl_start = source.find("const").unwrap();
    let attached = doc_comment_before_decl(source, decl_start, &docs);
    assert!(attached.is_some());
    assert_eq!(attached.unwrap().summary, "Adds two numbers.");
}

#[test]
fn renders_markdown_for_hover() {
    let doc = DocComment {
        range: 0..1,
        summary: "Summary".to_string(),
        description: "Summary\n\nMore details.".to_string(),
        params: [("value".to_string(), "input".to_string())]
            .into_iter()
            .collect(),
        returns: Some("i32".to_string()),
        deprecated: None,
        examples: vec!["add(1, 2);".to_string()],
    };
    let markdown = doc.to_markdown();
    assert!(markdown.contains("Summary"));
    assert!(markdown.contains("@param value, input"));
    assert!(markdown.contains("@returns i32"));
    assert!(markdown.contains("```tyrus\nadd(1, 2);\n```"));
}

#[test]
fn extracts_multiline_param_descriptions() {
    let source = r#"
/** Processes user data.
 *
 * @param user The user object containing
 *   name and email properties
 * @param validate Whether to validate the input
 *   before processing
 * @returns true if successful
 */
const process = (user: string, validate: boolean): boolean => true;
"#;
    let docs = extract_doc_comments(source);
    assert_eq!(docs.len(), 1);
    let doc = &docs[0];
    assert_eq!(doc.summary, "Processes user data.");
    assert_eq!(
        doc.params.get("user").map(String::as_str),
        Some("The user object containing name and email properties")
    );
    assert_eq!(
        doc.params.get("validate").map(String::as_str),
        Some("Whether to validate the input before processing")
    );
}

#[test]
fn extracts_deprecated_with_message() {
    let source = r#"
/** Old function that will be removed.
 *
 * @deprecated Use newFunction instead
 */
const oldFunc = (): void => {};
"#;
    let docs = extract_doc_comments(source);
    assert_eq!(docs.len(), 1);
    let doc = &docs[0];
    assert_eq!(doc.deprecated.as_deref(), Some("Use newFunction instead"));
}

#[test]
fn extracts_multiple_examples() {
    let source = r#"
/** Demonstrates basic operations.
 *
 * @example
 * add(1, 2)
 *
 * @example
 * add(3, 4)
 */
const add = (a: i32, b: i32): i32 => a + b;
"#;
    let docs = extract_doc_comments(source);
    assert_eq!(docs.len(), 1);
    let doc = &docs[0];
    assert_eq!(doc.examples.len(), 2);
    assert_eq!(doc.examples[0], "add(1, 2)");
    assert_eq!(doc.examples[1], "add(3, 4)");
}
