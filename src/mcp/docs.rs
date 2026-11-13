use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const INDEX_URL: &str = "https://tyruslang.org/llms.txt";
const FULL_URL: &str = "https://tyruslang.org/llms-full.txt";
const MAX_INDEX_BYTES: usize = 2 * 1024 * 1024;
const MAX_DOCUMENTATION_BYTES: usize = 32 * 1024 * 1024;
pub(super) const MAX_PAGE_BYTES: usize = 1024 * 1024;
const MAX_SNIPPET_CHARS: usize = 240;

#[derive(Clone, Debug)]
pub struct DocSection {
    pub heading: String,
    pub path: String,
    pub anchor: String,
    pub keywords: Vec<String>,
    pub snippet: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct DocsCache {
    pub root: PathBuf,
    pub compiler_version: String,
    pub docs_version: String,
    pub updated_at: String,
    pub index_text: String,
    pub full_text: String,
    pub pages: BTreeMap<String, String>,
    pub sections: Vec<DocSection>,
}

impl DocsCache {
    pub async fn download(root: &Path, compiler_version: &str) -> Result<Self, String> {
        Self::download_from(root, compiler_version, INDEX_URL, FULL_URL).await
    }

    async fn download_from(
        root: &Path,
        compiler_version: &str,
        index_url: &str,
        full_url: &str,
    ) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .user_agent(format!("tyrus/{compiler_version} MCP"))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| format!("could not create documentation client: {error}"))?;
        let index_text = download_text(&client, index_url, MAX_INDEX_BYTES).await?;
        let full_text = download_text(&client, full_url, MAX_DOCUMENTATION_BYTES).await?;
        if !index_text.starts_with("# Tyrus Documentation ")
            || !full_text.starts_with("# Tyrus Full Documentation ")
        {
            return Err("downloaded documentation has an unsupported format".to_string());
        }
        let docs_version = version_from_header(&index_text)
            .ok_or_else(|| "documentation index does not declare a version".to_string())?;
        let full_version = full_text
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("# Tyrus Full Documentation "));
        if full_version != Some(docs_version.as_str()) {
            return Err(
                "documentation index and full bundle declare different versions".to_string(),
            );
        }
        let pages = parse_pages(&full_text)?;
        if pages.is_empty() {
            return Err("documentation bundle contains no pages".to_string());
        }
        let sections = build_sections(&pages);
        if sections.is_empty() {
            return Err("documentation bundle contains no searchable headings".to_string());
        }
        let updated_at = timestamp_now();
        let cache = Self {
            root: root.to_path_buf(),
            compiler_version: compiler_version.to_string(),
            docs_version,
            updated_at,
            index_text,
            full_text,
            pages,
            sections,
        };
        cache.persist()?;
        Ok(cache)
    }

    pub fn load(root: &Path) -> Result<Self, String> {
        let pointer_path = root.join("current.json");
        let pointer: Value =
            serde_json::from_slice(&fs::read(&pointer_path).map_err(|_| {
                "documentation cache is missing; run `tyrus mcp install`".to_string()
            })?)
            .map_err(|_| {
                "documentation cache metadata is invalid; run `tyrus mcp update`".to_string()
            })?;
        let generation = pointer
            .get("generation")
            .and_then(Value::as_str)
            .filter(|value| {
                !value.is_empty()
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            })
            .ok_or_else(|| {
                "documentation cache metadata is invalid; run `tyrus mcp update`".to_string()
            })?;
        let generation_root = root.join("cache").join(generation);
        let metadata: Value = read_json(&generation_root.join("metadata.json"))?;
        let index_text = fs::read_to_string(generation_root.join("llms.txt"))
            .map_err(|_| "documentation cache is incomplete; run `tyrus mcp update`".to_string())?;
        let full_text = fs::read_to_string(generation_root.join("llms-full.txt"))
            .map_err(|_| "documentation cache is incomplete; run `tyrus mcp update`".to_string())?;
        let expected_hash = metadata
            .get("contentSha256")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                "documentation cache metadata has no content hash; run `tyrus mcp update`"
                    .to_string()
            })?;
        if content_hash(&index_text, &full_text) != expected_hash {
            return Err(
                "documentation cache content failed its integrity check; run `tyrus mcp update`"
                    .to_string(),
            );
        }
        let pages = parse_pages(&full_text)?;
        let sections = build_sections(&pages);
        Ok(Self {
            root: root.to_path_buf(),
            compiler_version: metadata
                .get("compilerVersion")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            docs_version: metadata
                .get("docsVersion")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            updated_at: metadata
                .get("updatedAt")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            index_text,
            full_text,
            pages,
            sections,
        })
    }

    pub fn persist(&self) -> Result<(), String> {
        let cache_root = self.root.join("cache");
        fs::create_dir_all(&cache_root)
            .map_err(|error| format!("could not create documentation cache: {error}"))?;
        let generation = format!(
            "{}-{}-{}",
            safe_version(&self.docs_version),
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let staging = cache_root.join(format!(".staging-{generation}"));
        fs::create_dir(&staging)
            .map_err(|error| format!("could not stage documentation cache: {error}"))?;
        let sections = self.sections.iter().map(section_json).collect::<Vec<_>>();
        let metadata = json!({
            "compilerVersion": self.compiler_version,
            "docsVersion": self.docs_version,
            "updatedAt": self.updated_at,
            "sectionCount": self.sections.len(),
            "sourceIndex": INDEX_URL,
            "sourceFull": FULL_URL,
            "contentSha256": content_hash(&self.index_text, &self.full_text),
        });
        let result = (|| {
            fs::write(staging.join("llms.txt"), &self.index_text)
                .map_err(|error| format!("could not write documentation index: {error}"))?;
            fs::write(staging.join("llms-full.txt"), &self.full_text)
                .map_err(|error| format!("could not write full documentation cache: {error}"))?;
            fs::write(
                staging.join("metadata.json"),
                serde_json::to_vec_pretty(&metadata).unwrap(),
            )
            .map_err(|error| format!("could not write documentation metadata: {error}"))?;
            fs::write(
                staging.join("index.json"),
                serde_json::to_vec_pretty(
                    &json!({ "version": self.docs_version, "sections": sections }),
                )
                .unwrap(),
            )
            .map_err(|error| format!("could not write documentation search index: {error}"))?;
            let generation_root = cache_root.join(&generation);
            fs::rename(&staging, &generation_root)
                .map_err(|error| format!("could not activate documentation cache: {error}"))?;
            atomic_write(
                &self.root.join("current.json"),
                &serde_json::to_vec_pretty(&json!({ "generation": generation }))
                    .map_err(|error| format!("could not encode cache pointer: {error}"))?,
            )?;
            if let Ok(entries) = fs::read_dir(&cache_root) {
                for entry in entries.flatten() {
                    if entry.file_name().to_string_lossy() != generation && entry.path().is_dir() {
                        let _ = fs::remove_dir_all(entry.path());
                    }
                }
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    pub fn page(&self, path: &str) -> Option<&str> {
        self.pages.get(path).map(String::as_str)
    }

    pub fn search(&self, query: &str, max_results: usize) -> Vec<Value> {
        let query_tokens = tokens(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }
        let mut ranked = self
            .sections
            .iter()
            .filter_map(|section| {
                let heading = normalize(&section.heading);
                let snippet = normalize(&section.snippet);
                let keywords = section
                    .keywords
                    .iter()
                    .map(|keyword| normalize(keyword))
                    .collect::<Vec<_>>();
                let score = query_tokens.iter().fold(0u32, |score, token| {
                    score
                        + if heading.contains(token) {
                            4
                        } else if keywords.iter().any(|keyword| keyword == token) {
                            2
                        } else if snippet.contains(token) {
                            1
                        } else {
                            0
                        }
                });
                (score > 0).then_some((score, section))
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.heading.cmp(&right.heading))
        });
        ranked
            .into_iter()
            .take(max_results.clamp(1, 20))
            .map(|(score, section)| {
                json!({
                    "heading": section.heading,
                    "path": section.path,
                    "anchor": section.anchor,
                    "snippet": section.snippet,
                    "url": section.url,
                    "score": score,
                })
            })
            .collect()
    }

    pub fn summary(&self) -> Value {
        json!({
            "compilerVersion": self.compiler_version,
            "docsVersion": self.docs_version,
            "updatedAt": self.updated_at,
            "pageCount": self.pages.len(),
            "sectionCount": self.sections.len(),
        })
    }
}

async fn download_text(
    client: &reqwest::Client,
    url: &str,
    maximum: usize,
) -> Result<String, String> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("could not download {url}: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "could not download {url}: HTTP {}",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|size| size > maximum as u64)
    {
        return Err(format!("download from {url} exceeds the size limit"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("could not read {url}: {error}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > maximum {
            return Err(format!("download from {url} exceeds the size limit"));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| format!("download from {url} is not valid UTF-8"))
}

fn parse_pages(full_text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut pages = BTreeMap::new();
    let mut current_path = None;
    let mut current_lines = Vec::new();
    for line in full_text.lines() {
        if let Some(path) = line.strip_prefix("# File: ") {
            if let Some(previous) = current_path.take() {
                if pages
                    .insert(previous, current_lines.join("\n").trim().to_string())
                    .is_some()
                {
                    return Err("documentation bundle contains a duplicate page path".to_string());
                }
                current_lines.clear();
            }
            if !valid_page_path(path) {
                return Err("documentation bundle contains an invalid page path".to_string());
            }
            current_path = Some(path.to_string());
        } else if current_path.is_some() {
            current_lines.push(line);
        }
    }
    if let Some(path) = current_path
        && pages
            .insert(path, current_lines.join("\n").trim().to_string())
            .is_some()
    {
        return Err("documentation bundle contains a duplicate page path".to_string());
    }
    Ok(pages)
}

fn valid_page_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && path.ends_with(".md")
}

fn build_sections(pages: &BTreeMap<String, String>) -> Vec<DocSection> {
    let mut sections = Vec::new();
    for (path, content) in pages {
        let mut in_code = false;
        let mut slug_occurrences = HashMap::<String, usize>::new();
        let lines = content.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_code = !in_code;
                continue;
            }
            if in_code {
                continue;
            }
            let Some((level, heading)) = parse_heading(trimmed) else {
                continue;
            };
            let heading = heading.trim().trim_end_matches('#').trim();
            if heading.is_empty() {
                continue;
            }
            let snippet = lines[index + 1..]
                .iter()
                .map(|line| line.trim())
                .find(|line| {
                    !line.is_empty()
                        && !line.starts_with('#')
                        && !line.starts_with("```")
                        && !line.starts_with("~~~")
                        && !line.starts_with("<!--")
                })
                .unwrap_or(heading);
            let snippet = plain_markdown(snippet, MAX_SNIPPET_CHARS);
            let base_anchor = slug(heading);
            let occurrence = slug_occurrences.entry(base_anchor.clone()).or_default();
            let anchor = if *occurrence == 0 {
                base_anchor
            } else {
                format!("{base_anchor}-{occurrence}")
            };
            *occurrence += 1;
            sections.push(DocSection {
                heading: heading.to_string(),
                path: path.clone(),
                anchor: anchor.clone(),
                keywords: tokens(&format!("{heading} {snippet}")),
                snippet,
                url: page_url(path, &anchor, level == 1),
            });
        }
    }
    sections.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.heading.cmp(&right.heading))
    });
    sections
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6)
        .contains(&level)
        .then(|| {
            line[level..]
                .strip_prefix(' ')
                .map(|heading| (level, heading))
        })
        .flatten()
}

fn page_url(path: &str, anchor: &str, is_h1: bool) -> String {
    let mut route = path.trim_end_matches(".md").to_string();
    if route == "index" {
        route.clear();
    } else if route.ends_with("/index") {
        route.truncate(route.len() - "index".len());
    }
    let route = if route.is_empty() {
        "/".to_string()
    } else {
        format!("/{route}")
    };
    if is_h1 || anchor.is_empty() {
        format!("https://tyruslang.org{route}")
    } else {
        format!("https://tyruslang.org{route}#{anchor}")
    }
}

fn section_json(section: &DocSection) -> Value {
    json!({
        "heading": section.heading,
        "path": section.path,
        "anchor": section.anchor,
        "keywords": section.keywords,
        "snippet": section.snippet,
        "url": section.url,
    })
}

fn safe_version(version: &str) -> String {
    version
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn content_hash(index: &str, full: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(index.as_bytes());
    hasher.update([0]);
    hasher.update(full.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn version_from_header(text: &str) -> Option<String> {
    text.lines()
        .next()?
        .strip_prefix("# Tyrus Documentation ")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|_| {
        "documentation cache metadata is missing; run `tyrus mcp update`".to_string()
    })?)
    .map_err(|_| "documentation cache metadata is invalid; run `tyrus mcp update`".to_string())
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "cache path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create cache directory: {error}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("could not stage cache file: {error}"))?;
    std::io::Write::write_all(&mut temporary, content)
        .map_err(|error| format!("could not write temporary cache file: {error}"))?;
    temporary
        .persist(path)
        .map_err(|error| format!("could not activate cache file: {}", error.error))?;
    Ok(())
}

fn timestamp_now() -> String {
    jiff::Timestamp::now().to_string()
}

fn normalize(text: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    use unicode_normalization::char::is_combining_mark;

    text.nfd()
        .filter(|character| !is_combining_mark(*character))
        .flat_map(char::to_lowercase)
        .collect()
}

fn tokens(text: &str) -> Vec<String> {
    normalize(text)
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn slug(heading: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in normalize(heading).chars() {
        if character.is_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character);
            separator = false;
        } else if character == '-' {
            if !result.is_empty() && !separator {
                result.push('-');
            }
            separator = true;
        } else {
            separator = true;
        }
    }
    result.trim_matches('-').to_string()
}

fn plain_markdown(text: &str, maximum: usize) -> String {
    let text = text
        .replace('`', "")
        .replace("**", "")
        .replace('*', "")
        .replace("[", "")
        .replace("]", "")
        .replace('>', "")
        .trim()
        .to_string();
    let mut result = text.chars().take(maximum).collect::<String>();
    if text.chars().count() > maximum {
        result.push('…');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{DocsCache, build_sections, parse_pages, tokens};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn search_removes_diacritics_and_ranks_heading_matches_first() {
        let pages = BTreeMap::from([
            (
                "guide/patterns.md".to_string(),
                "# Guide\n\n## Naïve pattern matching\n\nUse match expressions.\n".to_string(),
            ),
            (
                "guide/other.md".to_string(),
                "# Other\n\n## Match syntax\n\nPattern cases are useful.\n".to_string(),
            ),
        ]);
        let sections = build_sections(&pages);
        let cache = DocsCache {
            root: Default::default(),
            compiler_version: String::new(),
            docs_version: String::new(),
            updated_at: String::new(),
            index_text: String::new(),
            full_text: String::new(),
            pages,
            sections,
        };
        let results = cache.search("naive pattern", 5);
        assert_eq!(results[0]["path"], "guide/patterns.md");
    }

    #[test]
    fn page_bundle_rejects_paths_that_escape_the_docs_root() {
        let result =
            parse_pages("# Tyrus Full Documentation\n\n---\n\n# File: ../secret.md\n\nsecret");
        assert!(result.is_err());
    }

    #[test]
    fn tokenization_normalizes_accents_and_case() {
        assert_eq!(tokens("NAÏVE MCP"), ["naive", "mcp"]);
    }

    #[test]
    fn persisted_cache_loads_and_checks_its_content_hash() {
        let directory = tempfile::tempdir().unwrap();
        let pages = BTreeMap::from([("guide.md".to_string(), "# Guide\n\nText.".to_string())]);
        let full_text =
            "# Tyrus Full Documentation v0.1.0\n\n# File: guide.md\n\n# Guide\n\nText.".to_string();
        let cache = DocsCache {
            root: directory.path().to_path_buf(),
            compiler_version: "0.1.0".to_string(),
            docs_version: "v0.1.0".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            index_text: "# Tyrus Documentation v0.1.0".to_string(),
            full_text,
            sections: build_sections(&pages),
            pages,
        };
        cache.persist().unwrap();
        let loaded = DocsCache::load(directory.path()).unwrap();
        assert_eq!(loaded.docs_version, "v0.1.0");
        assert_eq!(loaded.page("guide.md"), Some("# Guide\n\nText."));

        let pointer: Value =
            serde_json::from_slice(&fs::read(directory.path().join("current.json")).unwrap())
                .unwrap();
        let generation = pointer["generation"].as_str().unwrap();
        fs::write(
            directory
                .path()
                .join("cache")
                .join(generation)
                .join("llms.txt"),
            "tampered",
        )
        .unwrap();
        assert!(
            DocsCache::load(directory.path())
                .unwrap_err()
                .contains("integrity check")
        );
    }

    #[test]
    fn missing_cache_recommends_installing_the_mcp() {
        let directory = tempfile::tempdir().unwrap();
        assert!(
            DocsCache::load(directory.path())
                .unwrap_err()
                .contains("tyrus mcp install")
        );
    }

    #[test]
    fn partial_cache_is_rejected_without_replacing_the_active_generation() {
        let directory = tempfile::tempdir().unwrap();
        let cache_root = directory.path().join("cache");
        let generation_root = cache_root.join("partial");
        fs::create_dir_all(&generation_root).unwrap();
        fs::write(
            directory.path().join("current.json"),
            r#"{"generation":"partial"}"#,
        )
        .unwrap();
        fs::write(generation_root.join("metadata.json"), "{}\n").unwrap();
        fs::write(
            generation_root.join("llms.txt"),
            "# Tyrus Documentation v0.1.0\n",
        )
        .unwrap();

        let error = DocsCache::load(directory.path()).unwrap_err();

        assert!(
            error.contains("cache is incomplete"),
            "unexpected error: {error}"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("current.json")).unwrap(),
            r#"{"generation":"partial"}"#
        );
    }

    #[tokio::test]
    async fn successful_download_replaces_the_active_cache_generation() {
        let directory = tempfile::tempdir().unwrap();
        let old_pages = BTreeMap::from([("guide.md".to_string(), "# Old Guide".to_string())]);
        let old_cache = DocsCache {
            root: directory.path().to_path_buf(),
            compiler_version: "0.1.0".to_string(),
            docs_version: "v0.0.9".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            index_text: "# Tyrus Documentation v0.0.9".to_string(),
            full_text: "# Tyrus Full Documentation v0.0.9\n\n# File: guide.md\n\n# Old Guide"
                .to_string(),
            sections: build_sections(&old_pages),
            pages: old_pages,
        };
        old_cache.persist().unwrap();
        let old_pointer: Value =
            serde_json::from_slice(&fs::read(directory.path().join("current.json")).unwrap())
                .unwrap();
        let old_generation = old_pointer["generation"].as_str().unwrap().to_string();

        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "SKIP: loopback HTTP testing requires permission to bind a local TCP listener ({error})."
                );
                return;
            }
            Err(error) => panic!("could not bind the loopback HTTP test listener: {error}"),
        };
        let address = listener.local_addr().unwrap();
        let index =
            b"# Tyrus Documentation v0.1.0\n\n- [Guide](https://tyruslang.org/guide) (guide.md)\n";
        let full = b"# Tyrus Full Documentation v0.1.0\n\n# File: guide.md\n\n# New Guide\n\n## MCP tools\n\nUpdated documentation.";
        let responses = [index.as_slice(), full.as_slice()];
        let server = thread::spawn(move || {
            for body in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let _ = stream.read(&mut request).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(body).unwrap();
            }
        });
        let url = format!("http://{address}/llms.txt");
        let full_url = format!("http://{address}/llms-full.txt");

        let updated = DocsCache::download_from(directory.path(), "0.1.0", &url, &full_url)
            .await
            .unwrap();
        server.join().unwrap();

        assert_eq!(updated.docs_version, "v0.1.0");
        assert_eq!(
            updated.page("guide.md"),
            Some("# New Guide\n\n## MCP tools\n\nUpdated documentation.")
        );
        let new_pointer: Value =
            serde_json::from_slice(&fs::read(directory.path().join("current.json")).unwrap())
                .unwrap();
        let new_generation = new_pointer["generation"].as_str().unwrap();
        assert_ne!(new_generation, old_generation);
        assert!(!directory.path().join("cache").join(old_generation).exists());
        assert_eq!(
            DocsCache::load(directory.path()).unwrap().docs_version,
            "v0.1.0"
        );
    }
}
