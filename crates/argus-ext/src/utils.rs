use std::borrow::Cow;

use regex::Regex;
use rustc_span::{source_map::SourceMap, Span};
use rustc_utils::source_map::range::CharRange;

pub trait CharRangeExt: Copy + Sized {
  /// Returns true if this range touches the `other`.
  fn overlaps(self, other: Self) -> bool;
}

impl CharRangeExt for CharRange {
  fn overlaps(self, other: Self) -> bool {
    self.start < other.end && other.start < self.end
  }
}

pub trait SpanExt {
  fn sanitized_snippet(self, map: &SourceMap) -> String;
}

fn trim_leading_whitespace(s: &str) -> Cow<'_, str> {
  if let Ok(re) = Regex::new(r"(?m)^\s*(\{|\})") {
    let result = re.replace_all(s, "$1");
    return result;
  }

  Cow::Borrowed(s)
}

impl SpanExt for Span {
  fn sanitized_snippet(self, map: &SourceMap) -> String {
    let snip = map
      .span_to_snippet(self)
      .unwrap_or_else(|_| format!("{self:?}"));
    trim_leading_whitespace(&snip).into_owned()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_trim_leading_whitespace_simple() {
    assert_eq!(trim_leading_whitespace("  {"), "{");
    assert_eq!(trim_leading_whitespace("  }"), "}");
    assert_eq!(trim_leading_whitespace("  {  "), "{  ");
    assert_eq!(trim_leading_whitespace("  }  "), "}  ");
    assert_eq!(trim_leading_whitespace("  {  }  "), "{  }  ");
  }

  #[test]
  fn test_trim_leading_whitespace_no_change() {
    assert_eq!(trim_leading_whitespace("{"), "{");
    assert_eq!(trim_leading_whitespace("}"), "}");
    assert_eq!(trim_leading_whitespace("{  "), "{  ");
    assert_eq!(trim_leading_whitespace("}  "), "}  ");
    assert_eq!(trim_leading_whitespace("{  }  "), "{  }  ");
  }

  #[test]
  fn test_trim_leading_whitespace_multiline() {
    assert_eq!(trim_leading_whitespace("  {\n  }"), "{\n}");
    assert_eq!(trim_leading_whitespace("  }\n  "), "}\n  ");
    assert_eq!(trim_leading_whitespace("  {\n}\n  "), "{\n}\n  ");
  }

  #[test]
  fn test_trim_leading_whitespace_interfering_chars() {
    assert_eq!(trim_leading_whitespace("{\n a }"), "{\n a }");
    assert_eq!(trim_leading_whitespace("}\nb  {"), "}\nb  {");
    assert_eq!(
      trim_leading_whitespace("{\n    a   {\n }"),
      "{\n    a   {\n}"
    );
  }
}
