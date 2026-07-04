//! Time helpers shared across parsing and scheduling.

/// A unit of relative time understood by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl TimeUnit {
    /// Recognize a unit word, case-insensitively. Returns `None` if the word is
    /// not a known unit.
    pub fn from_word(word: &str) -> Option<TimeUnit> {
        match word.to_ascii_lowercase().as_str() {
            "s" | "sec" | "secs" | "second" | "seconds" => Some(TimeUnit::Seconds),
            "m" | "min" | "mins" | "minute" | "minutes" => Some(TimeUnit::Minutes),
            "h" | "hr" | "hrs" | "hour" | "hours" => Some(TimeUnit::Hours),
            "d" | "day" | "days" => Some(TimeUnit::Days),
            _ => None,
        }
    }

    /// The number of seconds in one of this unit.
    pub fn seconds(self) -> i64 {
        match self {
            TimeUnit::Seconds => 1,
            TimeUnit::Minutes => 60,
            TimeUnit::Hours => 3_600,
            TimeUnit::Days => 86_400,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_unit_aliases_case_insensitively() {
        assert_eq!(TimeUnit::from_word("mins"), Some(TimeUnit::Minutes));
        assert_eq!(TimeUnit::from_word("Minutes"), Some(TimeUnit::Minutes));
        assert_eq!(TimeUnit::from_word("H"), Some(TimeUnit::Hours));
        assert_eq!(TimeUnit::from_word("secs"), Some(TimeUnit::Seconds));
        assert_eq!(TimeUnit::from_word("days"), Some(TimeUnit::Days));
    }

    #[test]
    fn rejects_unknown_units() {
        assert_eq!(TimeUnit::from_word("fortnights"), None);
        assert_eq!(TimeUnit::from_word(""), None);
    }

    #[test]
    fn seconds_per_unit_are_correct() {
        assert_eq!(TimeUnit::Minutes.seconds(), 60);
        assert_eq!(TimeUnit::Hours.seconds(), 3_600);
        assert_eq!(TimeUnit::Days.seconds(), 86_400);
    }
}
