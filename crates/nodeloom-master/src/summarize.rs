//! Turning a long document into LLM prompts: cut it into parts small enough
//! for a small model, summarize the parts across machines, then combine the
//! summaries (in more rounds while they are still long) into one. Pure text
//! handling; `state` runs the rounds as ordinary LLM jobs.

use nodeloom_core::validate::{MAX_PROMPT_CHARS, MAX_PROMPTS};

/// Longest document accepted, in characters: about 60 pages of text.
pub const MAX_DOCUMENT_CHARS: usize = 200_000;
/// Target size of one part. A 1B-3B model on a laptop reads this in seconds
/// and still keeps the whole part in view (the runner asks for an 8K
/// context; Korean text takes about one token per character).
pub const PART_CHARS: usize = 2500;
/// Summaries are combined in prompts of at most this many characters.
const COMBINE_CHARS: usize = 6000;
/// Each summary is cut to this length when combined, so every combining
/// prompt takes at least three of them and the rounds always shrink.
const SUMMARY_CHARS: usize = 1500;
const MAX_TITLE_CHARS: usize = 120;

/// Answer lengths, in tokens: a part's summary, a combined summary, and the
/// final one.
pub const PART_TOKENS: u32 = 250;
pub const COMBINE_TOKENS: u32 = 350;
pub const FINAL_TOKENS: u32 = 700;

/// A title fit for prompts and the dashboard: one line, no control
/// characters, not too long.
pub fn clean_title(title: &str) -> String {
    let one_line: String = title
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let title: String = one_line
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TITLE_CHARS)
        .collect();
    if title.is_empty() {
        "document".into()
    } else {
        title
    }
}

/// Paragraphs of `text` with their inner whitespace collapsed. PDF text
/// arrives with line breaks inside sentences and stray spacing; blank lines
/// are the only breaks kept.
fn paragraphs(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    text.split("\n\n")
        .map(|p| {
            p.split(|c: char| c.is_whitespace() || c.is_control())
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|p| !p.is_empty())
        .collect()
}

/// Splits after sentence ends, keeping the punctuation with its sentence.
fn sentences(paragraph: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = paragraph.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let ends = matches!(c, '.' | '!' | '?' | '。' | '！' | '？');
        let next_is_space = chars.peek().is_some_and(|(_, n)| *n == ' ');
        if ends && next_is_space {
            let end = i + c.len_utf8() + 1;
            out.push(&paragraph[start..end]);
            start = end;
            chars.next();
        }
    }
    if start < paragraph.len() {
        out.push(&paragraph[start..]);
    }
    out
}

/// Cuts a run of text with no sentence break into pieces of at most `max`
/// characters, at a space when there is one in the second half.
fn hard_split(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest: Vec<char> = text.chars().collect();
    while rest.len() > max {
        let cut = rest[max / 2..max]
            .iter()
            .rposition(|c| *c == ' ')
            .map_or(max, |p| max / 2 + p + 1);
        out.push(rest[..cut].iter().collect::<String>().trim().to_owned());
        rest.drain(..cut);
    }
    let last: String = rest.into_iter().collect();
    if !last.trim().is_empty() {
        out.push(last.trim().to_owned());
    }
    out
}

/// Cuts `text` into parts of at most `PART_CHARS` characters, at paragraph
/// and then sentence boundaries, so a part reads as a whole.
pub fn split_document(text: &str) -> Result<Vec<String>, String> {
    let chars = text.chars().count();
    if chars > MAX_DOCUMENT_CHARS {
        return Err(format!(
            "the document has {chars} characters; at most {MAX_DOCUMENT_CHARS} can be summarized"
        ));
    }
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_len = 0;
    let mut push = |piece: &str, sep: &str, parts: &mut Vec<String>| {
        let len = piece.chars().count();
        if current_len > 0 && current_len + sep.len() + len > PART_CHARS {
            parts.push(std::mem::take(&mut current));
            current_len = 0;
        }
        if current_len > 0 {
            current.push_str(sep);
            current_len += sep.len();
        }
        current.push_str(piece);
        current_len += len;
    };
    for paragraph in paragraphs(text) {
        let mut sep = "\n\n";
        for sentence in sentences(&paragraph) {
            let sentence = sentence.trim();
            if sentence.chars().count() > PART_CHARS {
                for piece in hard_split(sentence, PART_CHARS) {
                    push(&piece, sep, &mut parts);
                    sep = " ";
                }
            } else {
                push(sentence, sep, &mut parts);
            }
            sep = " ";
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    if parts.is_empty() {
        return Err("the document has no text; a scanned PDF needs text recognition first".into());
    }
    if parts.len() > MAX_PROMPTS {
        return Err(format!(
            "the document splits into {} parts; at most {MAX_PROMPTS}",
            parts.len()
        ));
    }
    Ok(parts)
}

/// The prompt that summarizes part `number` of `total`.
pub fn part_prompt(title: &str, number: usize, total: usize, part: &str) -> String {
    format!(
        "Below is part {number} of {total} of the document \"{title}\". Summarize this part \
         in 3 to 5 sentences. Keep names, numbers and conclusions. Write in the same language \
         as the text. Reply with the summary only.\n\n---\n{part}\n---"
    )
}

/// The prompt for a document short enough to summarize in one go.
pub fn whole_prompt(title: &str, text: &str) -> String {
    format!(
        "Summarize the document \"{title}\" below: a short overview paragraph, then the key \
         points as a bulleted list. Write in the same language as the document. Reply with \
         the summary only.\n\n---\n{text}\n---"
    )
}

fn joined(summaries: &[&str], first_number: usize) -> String {
    summaries
        .iter()
        .enumerate()
        .map(|(i, s)| format!("[{}]\n{s}", first_number + i))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// What to do with the summaries of one round.
#[derive(Debug, PartialEq, Eq)]
pub enum Combine {
    /// They fit in one prompt: this one writes the final summary.
    Final(String),
    /// Too long together: combine them in groups first, one prompt each.
    Round(Vec<String>),
}

/// Plans the next round for `summaries`, in document order.
pub fn plan_combine(title: &str, summaries: &[String]) -> Combine {
    let cut: Vec<String> = summaries
        .iter()
        .map(|s| s.trim().chars().take(SUMMARY_CHARS).collect())
        .collect();
    let all: Vec<&str> = cut.iter().map(String::as_str).collect();
    let text = joined(&all, 1);
    if text.chars().count() <= COMBINE_CHARS {
        return Combine::Final(format!(
            "Below are summaries of consecutive parts of the document \"{title}\", in order. \
             Write one summary of the whole document: a short overview paragraph, then the key \
             points as a bulleted list. Write in the same language as the summaries. Reply with \
             the summary only.\n\n{text}"
        ));
    }
    let mut groups: Vec<Vec<&str>> = Vec::new();
    let mut size = 0;
    for summary in all {
        let len = summary.chars().count() + 8;
        match groups.last_mut() {
            Some(group) if size + len <= COMBINE_CHARS => {
                group.push(summary);
                size += len;
            }
            _ => {
                groups.push(vec![summary]);
                size = len;
            }
        }
    }
    let mut first = 1;
    let prompts = groups
        .iter()
        .map(|group| {
            let text = joined(group, first);
            first += group.len();
            format!(
                "Below are summaries of consecutive parts of the document \"{title}\", in \
                 order. Combine them into one summary of 4 to 6 sentences. Keep names, numbers \
                 and conclusions. Write in the same language as the summaries. Reply with the \
                 summary only.\n\n{text}"
            )
        })
        .collect();
    Combine::Round(prompts)
}

const _: () = assert!(PART_CHARS + 600 + MAX_TITLE_CHARS < MAX_PROMPT_CHARS);
const _: () = assert!(COMBINE_CHARS + 600 + MAX_TITLE_CHARS < MAX_PROMPT_CHARS);
const _: () = assert!(3 * (SUMMARY_CHARS + 8) <= COMBINE_CHARS);

#[cfg(test)]
mod tests {
    use super::*;

    fn sentence(i: usize) -> String {
        format!("Sentence number {i} says something about the topic at some length.")
    }

    #[test]
    fn parts_follow_paragraphs_and_sentences() {
        let text = (0..200)
            .map(|p| {
                (0..5)
                    .map(|s| sentence(p * 5 + s))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let parts = split_document(&text).unwrap();
        assert!(parts.len() > 1);
        for part in &parts {
            assert!(part.chars().count() <= PART_CHARS, "{}", part.len());
            assert!(part.starts_with("Sentence"), "{part}");
            assert!(part.ends_with("length."), "{part}");
        }
        // Nothing is lost or reordered: only whitespace changes.
        let squash = |s: &str| s.split_whitespace().collect::<String>();
        assert_eq!(squash(&parts.join(" ")), squash(&text));
    }

    #[test]
    fn long_runs_without_breaks_are_cut_and_korean_is_counted_in_characters() {
        let text = "가나다라마바사 ".repeat(1000); // 8000 characters, no sentence end
        let parts = split_document(&text).unwrap();
        assert!(parts.len() >= 4);
        assert!(parts.iter().all(|p| p.chars().count() <= PART_CHARS));
        let squash = |s: &str| s.split_whitespace().collect::<String>();
        assert_eq!(squash(&parts.concat()), squash(&text));
        let one = split_document("짧은 문서입니다. 두 문장.").unwrap();
        assert_eq!(one, vec!["짧은 문서입니다. 두 문장.".to_owned()]);
    }

    #[test]
    fn empty_or_huge_documents_are_refused() {
        assert!(split_document(" \n\n \t ").is_err());
        assert!(split_document(&"x".repeat(MAX_DOCUMENT_CHARS + 1)).is_err());
        assert!(split_document(&"x ".repeat(MAX_DOCUMENT_CHARS / 2)).is_ok());
    }

    #[test]
    fn titles_are_one_short_line() {
        assert_eq!(clean_title("  report\n2024\u{7}.pdf "), "report 2024 .pdf");
        assert_eq!(clean_title(""), "document");
        assert_eq!(
            clean_title(&"t".repeat(500)).chars().count(),
            MAX_TITLE_CHARS
        );
    }

    #[test]
    fn prompts_name_the_part_and_fit_the_limit() {
        let part = "p".repeat(PART_CHARS);
        let prompt = part_prompt(&"t".repeat(MAX_TITLE_CHARS), 3, 12, &part);
        assert!(prompt.starts_with("Below is part 3 of 12"));
        assert!(prompt.contains(&part));
        assert!(prompt.chars().count() <= MAX_PROMPT_CHARS);
    }

    #[test]
    fn few_summaries_make_the_final_prompt_in_order() {
        let summaries = vec!["first".to_owned(), "second".to_owned()];
        let Combine::Final(prompt) = plan_combine("doc", &summaries) else {
            panic!("should be final")
        };
        let first = prompt.find("[1]\nfirst").unwrap();
        let second = prompt.find("[2]\nsecond").unwrap();
        assert!(first < second);
        assert!(prompt.contains("bulleted list"));
    }

    #[test]
    fn many_summaries_are_combined_in_shrinking_rounds() {
        // Overlong answers are cut, so every round shrinks until one is left.
        let mut summaries: Vec<String> = (0..80)
            .map(|i| format!("{i} {}", "s".repeat(3000)))
            .collect();
        let mut rounds = 0;
        loop {
            match plan_combine("doc", &summaries) {
                Combine::Final(prompt) => {
                    assert!(prompt.chars().count() <= MAX_PROMPT_CHARS);
                    break;
                }
                Combine::Round(prompts) => {
                    assert!(
                        prompts.len() * 3 <= summaries.len() + 2,
                        "{} → {}",
                        summaries.len(),
                        prompts.len()
                    );
                    assert!(
                        prompts
                            .iter()
                            .all(|p| p.chars().count() <= MAX_PROMPT_CHARS)
                    );
                    if rounds == 0 {
                        assert!(prompts[0].contains("[1]\n0 "));
                        assert!(
                            prompts[1].contains(&format!(
                                "[{}]\n",
                                prompts[0].matches("\n[").count() + 2
                            ))
                        );
                    }
                    summaries = prompts.iter().map(|_| "x".repeat(3000)).collect();
                    rounds += 1;
                }
            }
        }
        assert!(rounds <= 4, "{rounds}");
    }
}
