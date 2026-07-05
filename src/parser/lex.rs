//! Shared lexing helpers used by the relative and absolute parsers.

/// A whitespace-delimited token with its byte range in the original input.
pub(crate) struct Token<'a> {
    pub text: &'a str,
    pub start: usize,
    pub end: usize,
}

/// Split `input` into whitespace-delimited tokens, preserving byte offsets so
/// spans point back into the original string on UTF-8 boundaries.
pub(crate) fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut start: Option<usize> = None;

    for (i, ch) in input.char_indices() {
        if ch.is_whitespace() {
            if let Some(s) = start.take() {
                tokens.push(Token {
                    text: &input[s..i],
                    start: s,
                    end: i,
                });
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }

    if let Some(s) = start {
        tokens.push(Token {
            text: &input[s..],
            start: s,
            end: input.len(),
        });
    }

    tokens
}

/// Structural connector words that introduce the message (`to`, `about`,
/// `that`).
pub(crate) fn is_connector(word: &str) -> bool {
    matches!(word.to_ascii_lowercase().as_str(), "to" | "about" | "that")
}

/// Words recognized as structural filler before the time expression.
pub(crate) fn is_filler_prefix(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "remind" | "reminder" | "me" | "please" | "ping" | "hey"
    )
}

/// If the first token is a connector, peel it off so it becomes filler and is
/// excluded from the message.
pub(crate) fn split_leading_connector<'a, 'b>(
    tokens: &'b [Token<'a>],
) -> (Option<&'b Token<'a>>, &'b [Token<'a>]) {
    if let Some(first) = tokens.first()
        && is_connector(first.text)
    {
        return (Some(first), &tokens[1..]);
    }
    (None, tokens)
}
