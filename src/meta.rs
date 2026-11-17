//! Reading, writing and validating Wii Homebrew Channel `meta.xml` files.

use std::borrow::Cow;

use quick_xml::events::{BytesRef, Event};
use quick_xml::{Reader, XmlVersion};

/// The data stored in a `meta.xml` file.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Meta {
    /// The `version` attribute of the `<app>` element. Usually "1".
    pub app_version: String,
    pub name: String,
    pub coder: String,
    pub version: String,
    /// Format: `YYYYMMDDHHMMSS`
    pub release_date: String,
    pub short_description: String,
    pub long_description: String,
    /// One argument per line.
    pub arguments: String,
}

impl Meta {
    /// A fresh document with sensible defaults.
    pub fn new() -> Self {
        Self {
            app_version: "1".to_string(),
            name: String::new(),
            coder: String::new(),
            version: "1.0".to_string(),
            release_date: now_release_date(),
            short_description: String::new(),
            long_description: String::new(),
            arguments: String::new(),
        }
    }

    /// Read a `meta.xml` document.
    pub fn parse(xml: &str) -> Result<Self, String> {
        // Tolerate a UTF-8 BOM at the start of the file.
        let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);

        let mut reader = Reader::from_str(xml);
        let mut meta = Meta::default();
        let mut args: Vec<String> = Vec::new();
        let mut saw_app = false;
        let mut current: Option<Field> = None;

        loop {
            let event = reader
                .read_event()
                .map_err(|err| format!("XML error at byte {}: {err}", reader.error_position()))?;

            match event {
                Event::Start(e) => {
                    let tag = e.name();
                    match tag.as_ref() {
                        "app" => {
                            saw_app = true;
                            meta.app_version =
                                read_app_version(&e).unwrap_or_else(|| "1".to_string());
                        }
                        "name" => current = Some(Field::Name),
                        "coder" => current = Some(Field::Coder),
                        "version" => current = Some(Field::Version),
                        "release_date" => current = Some(Field::ReleaseDate),
                        "short_description" => current = Some(Field::Short),
                        "long_description" => current = Some(Field::Long),
                        "arg" => {
                            args.push(String::new());
                            current = Some(Field::Arg);
                        }
                        _ => current = None,
                    }
                }
                Event::Empty(e) => {
                    let tag = e.name();
                    match tag.as_ref() {
                        "app" => {
                            saw_app = true;
                            meta.app_version =
                                read_app_version(&e).unwrap_or_else(|| "1".to_string());
                        }
                        "arg" => args.push(String::new()),
                        "name" | "coder" | "version" | "release_date" | "short_description"
                        | "long_description" => current = None,
                        _ => {}
                    }
                }
                Event::Text(t) => {
                    if let Some(field) = current {
                        let text = t.xml10_content();
                        append(field, &mut meta, &mut args, &text);
                    }
                }
                Event::CData(t) => {
                    if let Some(field) = current {
                        let text = t.xml10_content();
                        append(field, &mut meta, &mut args, &text);
                    }
                }
                Event::GeneralRef(r) => {
                    if let Some(field) = current {
                        let text = resolve_ref(&r);
                        append(field, &mut meta, &mut args, &text);
                    }
                }
                Event::End(e) => {
                    let tag = e.name();
                    if matches!(
                        tag.as_ref(),
                        "name"
                            | "coder"
                            | "version"
                            | "release_date"
                            | "short_description"
                            | "long_description"
                            | "arg"
                    ) {
                        current = None;
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        if !saw_app {
            return Err("Not a Wii meta.xml file: <app> root element is missing.".to_string());
        }

        // Ignore the indentation around the actual element contents.
        meta.name = meta.name.trim().to_string();
        meta.coder = meta.coder.trim().to_string();
        meta.version = meta.version.trim().to_string();
        meta.release_date = meta.release_date.trim().to_string();
        meta.short_description = meta.short_description.trim().to_string();
        meta.long_description = meta.long_description.trim().to_string();
        meta.arguments = args
            .iter()
            .map(|arg| arg.trim())
            .collect::<Vec<_>>()
            .join("\n");
        if meta.app_version.trim().is_empty() {
            meta.app_version = "1".to_string();
        }

        Ok(meta)
    }

    /// Serialize the document in the layout used by the Homebrew Channel.
    pub fn to_xml(&self) -> String {
        let mut out = String::with_capacity(512);

        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str("<app version=\"");
        out.push_str(&escape_attr(&self.app_version));
        out.push_str("\">\n");

        write_element(&mut out, "name", &self.name);
        write_element(&mut out, "coder", &self.coder);
        write_element(&mut out, "version", &self.version);
        write_element(&mut out, "release_date", &self.release_date);
        write_element(&mut out, "short_description", &self.short_description);
        write_element(&mut out, "long_description", &self.long_description);

        out.push_str("  <arguments>\n");
        for arg in self.arguments.split('\n') {
            let arg = arg.trim_end_matches('\r');
            if arg.is_empty() {
                out.push_str("    <arg/>\n");
            } else {
                out.push_str("    <arg>");
                out.push_str(&escape_text(arg));
                out.push_str("</arg>\n");
            }
        }
        out.push_str("  </arguments>\n</app>\n");

        out
    }

    /// Check the values that the Homebrew Channel expects.
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();

        if self.app_version.trim().is_empty() {
            issues.push(Issue::warning(
                "App version (attribute) is empty. The Homebrew Channel expects \"1\".",
            ));
        } else if !self.app_version.trim().bytes().all(|b| b.is_ascii_digit()) {
            issues.push(Issue::warning(
                "App version (attribute) should be a number, usually \"1\".",
            ));
        }

        require(&mut issues, "Name", &self.name);
        require(&mut issues, "Coder", &self.coder);
        require(&mut issues, "Version", &self.version);

        if self.release_date.trim().is_empty() {
            issues.push(Issue::error("Release date is required (YYYYMMDDHHMMSS)."));
        } else if let Err(message) = validate_date(self.release_date.trim()) {
            issues.push(Issue::error(format!("Release date: {message}")));
        }

        require(&mut issues, "Short description", &self.short_description);

        if self.long_description.trim().is_empty() {
            issues.push(Issue::warning("Long description is empty."));
        }

        if self
            .arguments
            .split('\n')
            .any(|arg| arg.trim().contains('\u{0}'))
        {
            issues.push(Issue::error("Arguments contain an invalid NUL character."));
        }

        issues
    }
}

#[derive(Clone, Copy)]
enum Field {
    Name,
    Coder,
    Version,
    ReleaseDate,
    Short,
    Long,
    Arg,
}

fn append(field: Field, meta: &mut Meta, args: &mut [String], text: &str) {
    let target = match field {
        Field::Name => &mut meta.name,
        Field::Coder => &mut meta.coder,
        Field::Version => &mut meta.version,
        Field::ReleaseDate => &mut meta.release_date,
        Field::Short => &mut meta.short_description,
        Field::Long => &mut meta.long_description,
        Field::Arg => match args.last_mut() {
            Some(arg) => arg,
            None => return,
        },
    };
    target.push_str(text);
}

fn read_app_version(e: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    let attr = e.try_get_attribute("version").ok()??;
    let value = attr.normalized_value(XmlVersion::Implicit1_0).ok()?;
    Some(value.into_owned())
}

fn resolve_ref(r: &BytesRef<'_>) -> String {
    if r.is_char_ref() {
        return match r.resolve_char_ref() {
            Ok(Some(c)) => c.to_string(),
            _ => format!("&{};", &**r),
        };
    }

    match &**r {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        other => format!("&{other};"),
    }
}

fn require(issues: &mut Vec<Issue>, label: &str, value: &str) {
    if value.trim().is_empty() {
        issues.push(Issue::error(format!("{label} is required.")));
    }
}

fn write_element(out: &mut String, tag: &str, value: &str) {
    out.push_str("  <");
    out.push_str(tag);
    out.push('>');
    out.push_str(&escape_text(&normalize_newlines(value)));
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

fn normalize_newlines(value: &str) -> Cow<'_, str> {
    if value.contains('\r') {
        Cow::Owned(value.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(value)
    }
}

fn escape_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub level: Level,
    pub message: String,
}

impl Issue {
    fn error(message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            level: Level::Warning,
            message: message.into(),
        }
    }
}

fn validate_date(value: &str) -> Result<(), &'static str> {
    if value.len() != 14 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("use exactly 14 digits: YYYYMMDDHHMMSS.");
    }

    let number = |range: std::ops::Range<usize>| value[range].parse::<u32>().unwrap_or(0);
    let year = number(0..4);
    let month = number(4..6);
    let day = number(6..8);
    let hour = number(8..10);
    let minute = number(10..12);
    let second = number(12..14);

    if !(1970..=9999).contains(&year) {
        return Err("year must be between 1970 and 9999.");
    }
    if !(1..=12).contains(&month) {
        return Err("month must be between 01 and 12.");
    }
    if day < 1 || day > days_in_month(year, month) {
        return Err("day is not valid for that month.");
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err("time must be a valid 24-hour time.");
    }

    Ok(())
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Current UTC time as `YYYYMMDDHHMMSS`.
pub fn now_release_date() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let days = seconds.div_euclid(86_400);
    let time = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);

    format!(
        "{year:04}{month:02}{day:02}{:02}{:02}{:02}",
        time / 3_600,
        (time % 3_600) / 60,
        time % 60
    )
}

/// Convert days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;

    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<app version="1">
  <name>Homebrew Template</name>
  <coder>Lenny</coder>
  <version>1.0</version>
  <release_date>20260905000000</release_date>
  <short_description>Reusable template for Wii homebrew projects</short_description>
  <long_description>A template with an ampersand &amp; a less-than sign &lt;3.</long_description>
  <arguments>
    <arg>--fullscreen</arg>
    <arg/>
  </arguments>
</app>
"#;

    #[test]
    fn parses_example() {
        let meta = Meta::parse(EXAMPLE).unwrap();
        assert_eq!(meta.app_version, "1");
        assert_eq!(meta.name, "Homebrew Template");
        assert_eq!(meta.coder, "Lenny");
        assert_eq!(meta.version, "1.0");
        assert_eq!(meta.release_date, "20260905000000");
        assert_eq!(
            meta.short_description,
            "Reusable template for Wii homebrew projects"
        );
        assert_eq!(
            meta.long_description,
            "A template with an ampersand & a less-than sign <3."
        );
        assert_eq!(meta.arguments, "--fullscreen\n");
    }

    #[test]
    fn round_trips() {
        let meta = Meta::parse(EXAMPLE).unwrap();
        let reparsed = Meta::parse(&meta.to_xml()).unwrap();
        assert_eq!(meta, reparsed);
    }

    #[test]
    fn writes_homebrew_channel_layout() {
        let meta = Meta::parse(EXAMPLE).unwrap();
        let xml = meta.to_xml();
        assert!(
            xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<app version=\"1\">\n")
        );
        assert!(xml.contains("  <name>Homebrew Template</name>\n"));
        assert!(xml.contains("    <arg>--fullscreen</arg>\n"));
        assert!(xml.contains("    <arg/>\n"));
        assert!(xml.ends_with("</app>\n"));
    }

    #[test]
    fn escapes_special_characters() {
        let mut meta = Meta::new();
        meta.name = "A & B <C>".to_string();
        let xml = meta.to_xml();
        assert!(xml.contains("<name>A &amp; B &lt;C&gt;</name>"));
        assert_eq!(Meta::parse(&xml).unwrap().name, "A & B <C>");
    }

    #[test]
    fn rejects_other_xml() {
        let err = Meta::parse("<html><body>nope</body></html>").unwrap_err();
        assert!(err.contains("<app>"));
    }

    #[test]
    fn validates_release_date() {
        assert!(validate_date("20260905000000").is_ok());
        assert!(validate_date("20260229000000").is_err()); // 2026 is not a leap year
        assert!(validate_date("20240229000000").is_ok()); // 2024 is
        assert!(validate_date("20261305000000").is_err());
        assert!(validate_date("20260905240000").is_err());
        assert!(validate_date("2026090500000").is_err());
        assert!(validate_date("2026090500000a").is_err());
    }

    #[test]
    fn reports_missing_required_fields() {
        let meta = Meta::default();
        let issues = meta.validate();
        assert!(issues.iter().any(|i| i.message.contains("Name")));
        assert!(issues.iter().any(|i| i.message.contains("Coder")));
        assert!(issues.iter().any(|i| i.message.contains("Release date")));
    }

    #[test]
    fn timestamp_has_14_digits() {
        let timestamp = now_release_date();
        assert_eq!(timestamp.len(), 14);
        assert!(timestamp.bytes().all(|b| b.is_ascii_digit()));
    }
}
