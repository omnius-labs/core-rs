use std::{
    collections::BTreeSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use proc_macro2::TokenTree;
use syn::{Attribute, Item, Meta, spanned::Spanned as _};
use walkdir::WalkDir;

use crate::{
    config::Config,
    marker::{Marker, MarkerKind},
    report::{CheckReport, Finding},
    source_file::SourceFile,
};

pub struct Checker {
    base: PathBuf,
    config: Config,
}

impl Checker {
    pub fn new(base: impl AsRef<Path>, config: Config) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
            config,
        }
    }

    pub fn run(self) -> anyhow::Result<CheckReport> {
        let mut report = CheckReport::default();
        let mut walker = WalkDir::new(&self.base).sort_by_file_name().into_iter();

        while let Some(entry) = walker.next() {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type().is_dir() {
                if path != self.base && self.config.is_excluded(path) {
                    walker.skip_current_dir();
                }
                continue;
            }
            if !entry.file_type().is_file() || path.extension() != Some(OsStr::new("rs")) {
                continue;
            }
            if self.config.is_excluded(path) {
                continue;
            }

            let source = SourceFile::read(path, &self.base)?;
            Self::check_source(&source, &mut report);
        }

        Ok(report)
    }

    fn check_source(source: &SourceFile, report: &mut CheckReport) {
        let mut consumed_markers = BTreeSet::new();
        Self::check_items(source, &source.ast().items, false, &mut consumed_markers, report);

        for (line, marker) in source.markers() {
            if consumed_markers.contains(line) {
                continue;
            }
            let message = match marker {
                Ok(_) => "marker is not attached to a checked free function".to_string(),
                Err(error) => format!("invalid omnius-lint marker: {error}"),
            };
            report.errors.push(Finding::error(source.display_path().to_path_buf(), *line, message));
        }
    }

    fn check_items(source: &SourceFile, items: &[Item], in_test_scope: bool, consumed_markers: &mut BTreeSet<usize>, report: &mut CheckReport) {
        for item in items {
            match item {
                Item::Fn(function) => {
                    if in_test_scope || Self::attributes_are_test(&function.attrs) || function.sig.ident == "main" {
                        continue;
                    }
                    let line = function.sig.fn_token.span().start().line;
                    let subject = format!("fn {}", function.sig.ident);
                    let Some((marker_line, marker)) = source.marker_above(line) else {
                        report
                            .errors
                            .push(Finding::error(source.display_path().to_path_buf(), line, format!("unmarked free function: {subject}")));
                        continue;
                    };
                    consumed_markers.insert(marker_line);
                    match marker {
                        Ok(marker) => Self::record_marker(source, line, &subject, marker, report),
                        Err(error) => report.errors.push(Finding::error(
                            source.display_path().to_path_buf(),
                            marker_line,
                            format!("invalid omnius-lint marker: {error}"),
                        )),
                    }
                }
                Item::Mod(module) => {
                    let nested_test_scope = in_test_scope || Self::attributes_are_test(&module.attrs);
                    if let Some((_, nested_items)) = &module.content {
                        Self::check_items(source, nested_items, nested_test_scope, consumed_markers, report);
                    }
                }
                _ => {}
            }
        }
    }

    fn record_marker(source: &SourceFile, line: usize, subject: &str, marker: &Marker, report: &mut CheckReport) {
        match marker.kind() {
            MarkerKind::Allow => report.allow_count += 1,
            MarkerKind::Debt => report
                .debts
                .push(Finding::debt(source.display_path().to_path_buf(), line, format!("{subject} -- {}", marker.reason()))),
        }
    }

    fn attributes_are_test(attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| {
            if attr.path().segments.last().is_some_and(|segment| segment.ident == "test") {
                return true;
            }
            let Meta::List(list) = &attr.meta else {
                return false;
            };
            attr.path().is_ident("cfg") && Self::cfg_is_test(list.tokens.clone().into_iter())
        })
    }

    fn cfg_is_test(mut tokens: impl Iterator<Item = TokenTree>) -> bool {
        matches!(tokens.next(), Some(TokenTree::Ident(ident)) if ident == "test") && tokens.next().is_none()
    }
}

#[cfg(test)]
mod tests {
    use crate::source_file::SourceFile;

    use super::{CheckReport, Checker};

    fn check(text: &str) -> CheckReport {
        let source = SourceFile::for_test(text);
        let mut report = CheckReport::default();
        Checker::check_source(&source, &mut report);
        report
    }

    #[test]
    fn accepts_allow_and_reports_debt_without_error() {
        let report = check(
            r#"
// omnius-lint:allow(free-fn) OS boundary
fn allowed() {}

// omnius-lint:debt(free-fn) move to Owner in a separate refactor
fn debt() {}
"#,
        );

        assert!(report.errors.is_empty());
        assert_eq!(report.allow_count, 1);
        assert_eq!(report.debts.len(), 1);
    }

    #[test]
    fn reports_the_declaration_line_of_a_finding() {
        let report = check(
            r#"
fn first() {}

fn second() {}
"#,
        );

        assert_eq!(report.errors.len(), 2);
        assert!(report.errors[0].to_string().contains(":2:"));
        assert!(report.errors[1].to_string().contains(":4:"));
    }

    #[test]
    fn rejects_unmarked_function() {
        let report = check("fn helper() {}");
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].message().contains("unmarked free function"));
    }

    #[test]
    fn rejects_invalid_marker_without_duplicate_unmarked_error() {
        let report = check(
            r#"
// omnius-lint:allow(unknown) reason
fn helper() {}
"#,
        );
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].message().contains("unknown marker key"));
    }

    #[test]
    fn rejects_unused_marker() {
        let report = check("// omnius-lint:allow(free-fn) no function follows");
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].message().contains("not attached"));
    }

    #[test]
    fn excludes_main_and_test_functions() {
        let report = check(
            r#"
fn main() {}

#[test]
fn unit_test() {}

#[cfg(test)]
fn cfg_test() {}
"#,
        );
        assert!(report.errors.is_empty());
    }

    #[test]
    fn checks_functions_that_are_not_test_only() {
        let report = check(
            r#"
#[cfg(not(test))]
fn helper() {}
"#,
        );
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].message().contains("fn helper"));
    }

    #[test]
    fn checks_inline_modules_but_excludes_inline_test_modules() {
        let report = check(
            r#"
mod checked {
    fn helper() {}
}

#[cfg(test)]
mod tests {
    fn helper() {}
}
"#,
        );
        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].message().contains("fn helper"));
    }
}
