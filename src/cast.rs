use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct RecordingSession {
    pub terminal_size: TerminalSize,
    pub events: Vec<AsciicastEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalSize {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsciicastEvent {
    pub time: f64,
    pub data: String,
}

#[derive(Debug, Deserialize)]
struct HeaderV2 {
    version: u8,
    width: usize,
    height: usize,
}

#[derive(Debug, Deserialize)]
struct HeaderV3 {
    version: u8,
    term: TermV3,
}

#[derive(Debug, Deserialize)]
struct TermV3 {
    cols: usize,
    rows: usize,
}

impl RecordingSession {
    pub fn read_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read asciicast from {}", path.display()))?;
        Self::read_from_str(&content)
    }

    pub fn read_from_str(content: &str) -> Result<Self> {
        let mut lines = content.lines();
        let header_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("invalid asciicast: missing header"))?;
        let header_json: Value =
            serde_json::from_str(header_line).context("invalid asciicast header JSON")?;
        let version = header_json
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow::anyhow!("invalid asciicast: missing version"))?;

        let terminal_size = match version {
            2 => {
                let header: HeaderV2 =
                    serde_json::from_value(header_json).context("invalid asciicast v2 header")?;
                if header.version != 2 {
                    anyhow::bail!("unsupported asciicast version {}", header.version);
                }
                TerminalSize {
                    width: header.width.max(1),
                    height: header.height.max(1),
                }
            }
            3 => {
                let header: HeaderV3 =
                    serde_json::from_value(header_json).context("invalid asciicast v3 header")?;
                if header.version != 3 {
                    anyhow::bail!("unsupported asciicast version {}", header.version);
                }
                TerminalSize {
                    width: header.term.cols.max(1),
                    height: header.term.rows.max(1),
                }
            }
            other => anyhow::bail!("unsupported asciicast version {}", other),
        };

        let mut events = Vec::new();
        let mut elapsed = 0.0;

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let value: Value = match serde_json::from_str(line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let Some(parts) = value.as_array() else {
                continue;
            };
            if parts.len() < 3 {
                continue;
            }
            let Some(time) = parts[0].as_f64() else {
                continue;
            };
            let kind = parts[1].as_str().unwrap_or_default();
            if kind != "o" {
                if version == 3 {
                    elapsed += time;
                }
                continue;
            }
            let data = parts[2].as_str().unwrap_or_default().to_string();
            let absolute_time = if version == 3 {
                elapsed += time;
                elapsed
            } else {
                time
            };

            events.push(AsciicastEvent {
                time: absolute_time.max(0.0),
                data,
            });
        }

        Ok(Self {
            terminal_size,
            events,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v2_cast() {
        let content = r#"{"version":2,"width":80,"height":24,"timestamp":0}
[0.1,"o","hello"]
[0.5,"o"," world"]
"#;
        let session = RecordingSession::read_from_str(content).unwrap();
        assert_eq!(session.terminal_size.width, 80);
        assert_eq!(session.events.len(), 2);
        assert_eq!(session.events[1].time, 0.5);
    }

    #[test]
    fn parses_v3_cast_with_relative_timing() {
        let content = r#"{"version":3,"term":{"cols":90,"rows":30},"timestamp":0}
[0.1,"o","hello"]
[0.0,"i","ignored"]
[0.2,"o"," world"]
"#;
        let session = RecordingSession::read_from_str(content).unwrap();
        assert_eq!(session.terminal_size.height, 30);
        assert_eq!(session.events.len(), 2);
        assert!((session.events[1].time - 0.3).abs() < 1e-9);
    }

    #[test]
    fn rejects_unknown_version() {
        let result = RecordingSession::read_from_str(r#"{"version":9}"#);
        assert!(result.is_err());
    }
}
