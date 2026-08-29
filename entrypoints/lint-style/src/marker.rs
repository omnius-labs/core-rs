#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerKind {
    Allow,
    Debt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    kind: MarkerKind,
    reason: String,
}

impl Marker {
    pub fn parse(line: &str) -> Option<Result<Self, String>> {
        let comment = line.trim().strip_prefix("//")?.trim_start();
        let rest = comment.strip_prefix("omnius-lint:")?;
        Some(Self::parse_body(rest))
    }

    fn parse_body(body: &str) -> Result<Self, String> {
        let Some(open) = body.find('(') else {
            return Err("marker must use allow(free-fn) or debt(free-fn)".to_string());
        };
        let Some(close_offset) = body[open + 1..].find(')') else {
            return Err("marker is missing ')'".to_string());
        };
        let close = open + 1 + close_offset;
        let kind = match &body[..open] {
            "allow" => MarkerKind::Allow,
            "debt" => MarkerKind::Debt,
            other => return Err(format!("unknown marker kind '{other}'")),
        };
        let key = &body[open + 1..close];
        if key != "free-fn" {
            return Err(format!("unknown marker key '{key}'"));
        }

        let suffix = &body[close + 1..];
        if !suffix.is_empty() && !suffix.starts_with(char::is_whitespace) {
            return Err("marker reason must be separated by whitespace".to_string());
        }
        let reason = suffix.trim();
        if reason.is_empty() {
            return Err("marker reason is required".to_string());
        }

        Ok(Self { kind, reason: reason.to_string() })
    }

    pub fn kind(&self) -> MarkerKind {
        self.kind
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[cfg(test)]
mod tests {
    use super::{Marker, MarkerKind};

    #[test]
    fn parses_allow_and_debt() {
        let allow = Marker::parse("// omnius-lint:allow(free-fn) OS boundary").expect("marker").expect("valid marker");
        assert_eq!(allow.kind(), MarkerKind::Allow);
        assert_eq!(allow.reason(), "OS boundary");

        let debt = Marker::parse("  // omnius-lint:debt(free-fn) move to ArchiveFormat")
            .expect("marker")
            .expect("valid marker");
        assert_eq!(debt.kind(), MarkerKind::Debt);
        assert_eq!(debt.reason(), "move to ArchiveFormat");
    }

    #[test]
    fn rejects_missing_reason() {
        let error = Marker::parse("// omnius-lint:allow(free-fn)").expect("marker").expect_err("reason must be required");
        assert_eq!(error, "marker reason is required");
    }

    #[test]
    fn rejects_unknown_kind_and_key() {
        let kind_error = Marker::parse("// omnius-lint:waive(free-fn) reason").expect("marker").expect_err("kind must be checked");
        assert_eq!(kind_error, "unknown marker kind 'waive'");

        let key_error = Marker::parse("// omnius-lint:allow(unit-struct) reason").expect("marker").expect_err("key must be checked");
        assert_eq!(key_error, "unknown marker key 'unit-struct'");
    }

    #[test]
    fn rejects_malformed_marker() {
        let error = Marker::parse("// omnius-lint:allow free-fn").expect("marker").expect_err("syntax must be checked");
        assert_eq!(error, "marker must use allow(free-fn) or debt(free-fn)");
    }
}
