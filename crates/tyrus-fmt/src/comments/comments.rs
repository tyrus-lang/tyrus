use std::ops::Range;

#[derive(Debug, Clone)]
pub struct Comment {
    pub text: String,
    pub range: Range<usize>,
    pub line: usize,
    pub is_block: bool,
    pub is_doc: bool,
}

pub fn extract_comments(source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut line = 0;

    while i < source.len() {
        if i + 1 < source.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            i += 2;
            while i < source.len() && bytes[i] != b'\n' {
                i += 1;
            }
            let text = source[start..i].to_string();
            comments.push(Comment {
                text,
                range: start..i,
                line,
                is_block: false,
                is_doc: false,
            });
            continue;
        }

        if i + 1 < source.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            let comment_line = line;
            let is_doc = i + 2 < source.len() && bytes[i + 2] == b'*';
            i += 2;
            while i + 1 < source.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < source.len() {
                i += 2;
            }
            let text = source[start..i].to_string();
            line += bytes[start..i]
                .iter()
                .filter(|&&byte| byte == b'\n')
                .count();
            comments.push(Comment {
                text,
                range: start..i,
                line: comment_line,
                is_block: true,
                is_doc,
            });
            continue;
        }

        if bytes[i] == b'\n' {
            line += 1;
        }
        i += 1;
    }

    comments
}

pub fn format_comments(comments: &[Comment], indent: &str, indent_level: usize) -> String {
    let mut out = String::new();
    let prefix = indent.repeat(indent_level);

    for comment in comments {
        out.push_str(&prefix);
        out.push_str(&comment.text);
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::extract_comments;

    #[test]
    fn comment_line_numbers_match_single_pass_source_positions() {
        let source = "let first = 1;\n// one\nlet second = 2; /* block\ncontinued */\n// two\n";
        let comments = extract_comments(source);
        assert_eq!(
            comments
                .iter()
                .map(|comment| comment.line)
                .collect::<Vec<_>>(),
            vec![1, 2, 4]
        );
    }
}
