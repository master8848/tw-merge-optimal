//! Candidate extraction using `tailwindcss-oxide` (the same extractor the
//! Tailwind CLI uses). Files are pre-processed by extension, then candidates
//! are extracted with their byte offsets (offsets refer to the pre-processed
//! content).

use std::path::Path;
use tailwindcss_oxide::extractor::{Extracted, Extractor};

pub struct CandidateHit {
    pub class: String,
    /// Byte offset in the (pre-processed) content.
    pub offset: usize,
}

pub struct FileScan {
    pub path: String,
    pub content: Vec<u8>,
    pub candidates: Vec<CandidateHit>,
}

/// Pre-process + extract candidates from a single file.
pub fn scan_content(path: &Path, content: &[u8]) -> FileScan {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let processed = pre_process(content, &ext);
    let mut extractor = Extractor::new(&processed);
    let mut candidates = Vec::new();
    for hit in extractor.extract() {
        if let Extracted::Candidate(bytes) = hit {
            let offset = bytes.as_ptr() as usize - processed.as_ptr() as usize;
            if let Ok(s) = std::str::from_utf8(bytes) {
                candidates.push(CandidateHit {
                    class: s.to_string(),
                    offset,
                });
            }
        }
    }
    FileScan {
        path: path.to_string_lossy().into_owned(),
        content: processed,
        candidates,
    }
}

fn pre_process(content: &[u8], extension: &str) -> Vec<u8> {
    tailwindcss_oxide::scanner::pre_process_input(content.to_vec(), extension)
}

/// Line/column of a byte offset in a content buffer (1-based).
pub fn line_col(content: &[u8], offset: usize) -> (usize, usize) {
    let offset = offset.min(content.len());
    let mut line = 1usize;
    let mut col = 1usize;
    for &b in &content[..offset] {
        if b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_html() {
        let scan = scan_content(
            Path::new("x.html"),
            r#"<div class="flex items-center px-2.5 hover:bg-red-500"></div>"#.as_bytes(),
        );
        let classes: Vec<&str> = scan.candidates.iter().map(|c| c.class.as_str()).collect();
        assert!(classes.contains(&"flex"));
        assert!(classes.contains(&"hover:bg-red-500"));
        assert!(classes.contains(&"px-2.5"));
    }

    #[test]
    fn line_col_works() {
        let content = b"ab\ncde\nf";
        assert_eq!(line_col(content, 0), (1, 1));
        assert_eq!(line_col(content, 3), (2, 1));
        assert_eq!(line_col(content, 5), (2, 3));
    }
}
