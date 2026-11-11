//! Log output: one JSON object per line by default, or `key=value` text.
//!
//! Both formats render the record's structured fields (`log::info!(key = v; "msg")`).

use std::io::{self, IsTerminal, Write as _};

use env_logger::fmt::Formatter;
use log::Record;
use log::kv::{self, Key, Value, VisitSource};
use serde_json::Value as Json;

use crate::cli::{LogFormat, LogLevel};

/// Install the global logger, writing to stdout. `RUST_LOG` overrides `level`.
pub fn init(level: LogLevel, format: LogFormat) {
    let mut builder = env_logger::Builder::new();
    builder.filter_level(level.into()).parse_default_env();
    builder.target(env_logger::Target::Stdout);
    match resolve(format, std::io::stdout().is_terminal()) {
        LogFormat::Json => builder.format(format_json),
        LogFormat::Text => builder.format(format_text),
        LogFormat::Auto => unreachable!("resolve returns a concrete format"),
    };
    builder.init();
}

/// Resolve `Auto` into a concrete format: text on a terminal, JSON otherwise.
fn resolve(format: LogFormat, is_terminal: bool) -> LogFormat {
    match format {
        LogFormat::Auto if is_terminal => LogFormat::Text,
        LogFormat::Auto => LogFormat::Json,
        other => other,
    }
}

/// A structured field value as JSON: numbers, booleans and `:serde` values (such as
/// arrays) stay typed, the rest becomes a string.
fn to_json(value: &Value) -> Json {
    serde_json::to_value(value).unwrap_or_else(|_| Json::from(value.to_string()))
}

/// Appends `,"key":value` for every field.
///
/// The keys `time`, `level` and `message` are reserved for the prefix emitted by
/// [`json_prefix`]; log call sites must not reuse them.
struct JsonFields<'a>(&'a mut String);

impl<'kvs> VisitSource<'kvs> for JsonFields<'_> {
    fn visit_pair(
        &mut self,
        key: Key<'kvs>,
        value: Value<'kvs>,
    ) -> Result<(), kv::Error> {
        let pair = format!(",{}:{}", Json::from(key.as_str()), to_json(&value));
        self.0.push_str(&pair);
        Ok(())
    }
}

/// Appends ` key=value` for every field, quoting strings only when needed.
struct TextFields<'a>(&'a mut String);

impl<'kvs> VisitSource<'kvs> for TextFields<'_> {
    fn visit_pair(
        &mut self,
        key: Key<'kvs>,
        value: Value<'kvs>,
    ) -> Result<(), kv::Error> {
        let value = match to_json(&value) {
            Json::String(s) if needs_quotes(&s) => format!("{s:?}"),
            Json::String(s) => s,
            other => other.to_string(),
        };
        self.0.push_str(&format!(" {key}={value}"));
        Ok(())
    }
}

fn needs_quotes(s: &str) -> bool {
    s.is_empty() || s.contains(|c: char| c.is_whitespace() || c == '"' || c == '=')
}

/// The `{"time":..,"level":..,"message":..` prefix of a JSON log line, with a lowercase
/// level.
fn json_prefix(time: &str, level: log::Level, message: &str) -> String {
    let q = |s: &str| Json::from(s);
    format!(
        r#"{{"time":{},"level":{},"message":{}"#,
        q(time),
        q(&level.as_str().to_ascii_lowercase()),
        q(message),
    )
}

/// `{"time":..,"level":..,"message":..,<fields>}`; string values are escaped by
/// `serde_json`.
fn format_json(buf: &mut Formatter, record: &Record) -> io::Result<()> {
    let time = buf.timestamp_millis().to_string();
    let mut line = json_prefix(&time, record.level(), &record.args().to_string());
    let _ = record.key_values().visit(&mut JsonFields(&mut line));
    line.push('}');
    writeln!(buf, "{line}")
}

/// `[time LEVEL] message key=value ...`
fn format_text(buf: &mut Formatter, record: &Record) -> io::Result<()> {
    let mut line = format!(
        "[{} {}] {}",
        buf.timestamp_millis(),
        record.level(),
        record.args()
    );
    let _ = record.key_values().visit(&mut TextFields(&mut line));
    writeln!(buf, "{line}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_resolves_to_text_on_a_tty_and_json_otherwise() {
        assert_eq!(resolve(LogFormat::Auto, true), LogFormat::Text);
        assert_eq!(resolve(LogFormat::Auto, false), LogFormat::Json);
        assert_eq!(resolve(LogFormat::Text, false), LogFormat::Text);
        assert_eq!(resolve(LogFormat::Json, true), LogFormat::Json);
    }

    #[test]
    fn json_prefix_uses_lowercase_level() {
        assert_eq!(
            json_prefix("2026-10-13T19:00:00Z", log::Level::Info, "hi"),
            r#"{"time":"2026-10-13T19:00:00Z","level":"info","message":"hi""#
        );
        assert_eq!(
            json_prefix("t", log::Level::Error, "failed"),
            r#"{"time":"t","level":"error","message":"failed""#
        );
    }

    #[test]
    fn json_prefix_escapes_message() {
        assert_eq!(
            json_prefix("t", log::Level::Warn, r#"say "hi""#),
            r#"{"time":"t","level":"warn","message":"say \"hi\"""#
        );
    }

    #[test]
    fn values_keep_their_json_type() {
        assert_eq!(to_json(&Value::from(true)), Json::from(true));
        assert_eq!(to_json(&Value::from(120u64)), Json::from(120u64));
        assert_eq!(to_json(&Value::from(-3i64)), Json::from(-3i64));
        assert_eq!(to_json(&Value::from("eno1")), Json::from("eno1"));
    }

    #[test]
    fn serde_values_become_json_arrays() {
        let names = vec!["192.168.1.0/24", "10.0.0.0/24"];
        let value = Value::from_serde(&names);
        assert_eq!(
            to_json(&value),
            serde_json::json!(["192.168.1.0/24", "10.0.0.0/24"])
        );

        let mut out = String::new();
        JsonFields(&mut out)
            .visit_pair(Key::from("networks"), value.clone())
            .unwrap();
        assert_eq!(out, r#","networks":["192.168.1.0/24","10.0.0.0/24"]"#);

        let mut out = String::new();
        TextFields(&mut out)
            .visit_pair(Key::from("networks"), value)
            .unwrap();
        assert_eq!(out, r#" networks=["192.168.1.0/24","10.0.0.0/24"]"#);
    }

    #[test]
    fn json_fields_are_escaped_and_prefixed_with_a_comma() {
        let mut out = String::new();
        let kvs: [(&str, Value); 2] = [
            ("name", Value::from("a\"b\\c")),
            ("bytes", Value::from(7u64)),
        ];
        for (k, v) in kvs {
            JsonFields(&mut out).visit_pair(Key::from(k), v).unwrap();
        }
        assert_eq!(out, r#","name":"a\"b\\c","bytes":7"#);
    }

    #[test]
    fn text_fields_quote_only_when_needed() {
        let mut out = String::new();
        let kvs: [(&str, Value); 3] = [
            ("a", Value::from("eno1")),
            ("b", Value::from("two words")),
            ("c", Value::from(5u64)),
        ];
        for (k, v) in kvs {
            TextFields(&mut out).visit_pair(Key::from(k), v).unwrap();
        }
        assert_eq!(out, r#" a=eno1 b="two words" c=5"#);
    }
}
