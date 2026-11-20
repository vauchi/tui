// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The terminal's file picker: a path line under the title Core prepared.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vauchi_contract::{Event, FilePickFailure, FilePickOutcome};

pub(crate) struct PathLine {
    request_id: String,
    title: String,
    max_bytes: u64,
    typed: String,
}

impl PathLine {
    pub(crate) fn new(request_id: String, title: String, max_bytes: u64) -> Self {
        Self {
            request_id,
            title,
            max_bytes,
            typed: String::new(),
        }
    }

    pub(crate) fn answers(&self, request_id: &str) -> bool {
        self.request_id == request_id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn typed(&self) -> &str {
        &self.typed
    }

    /// The pick's answer once the key ends it; `None` while the line is
    /// still being typed.
    pub(crate) fn key(&mut self, key: KeyEvent) -> Option<Event> {
        let outcome = match key.code {
            KeyCode::Esc => FilePickOutcome::Cancelled,
            KeyCode::Enter if !self.typed.trim().is_empty() => self.read(),
            KeyCode::Backspace => {
                self.typed.pop();
                return None;
            }
            KeyCode::Char(character) if !key.modifiers.intersects(KeyModifiers::CONTROL) => {
                self.typed.push(character);
                return None;
            }
            _ => return None,
        };
        Some(Event::FilePickCompleted {
            request_id: self.request_id.clone(),
            outcome,
        })
    }

    fn read(&self) -> FilePickOutcome {
        let failed = |code| FilePickOutcome::Failed { code };
        let path = std::path::Path::new(self.typed.trim());
        // The size is checked on the file's metadata, so a file over Core's
        // limit is never loaded; the read is capped as well because the
        // file can grow between the two.
        let Ok(metadata) = std::fs::metadata(path) else {
            return failed(FilePickFailure::ReadFailed);
        };
        if !metadata.is_file() {
            return failed(FilePickFailure::UnsupportedFile);
        }
        if metadata.len() > self.max_bytes {
            return failed(FilePickFailure::TooLarge);
        }
        let mut bytes = Vec::new();
        let read = std::fs::File::open(path).and_then(|file| {
            use std::io::Read;
            file.take(self.max_bytes.saturating_add(1))
                .read_to_end(&mut bytes)
        });
        match read {
            Err(_) => failed(FilePickFailure::ReadFailed),
            Ok(_) if bytes.len() as u64 > self.max_bytes => failed(FilePickFailure::TooLarge),
            Ok(_) => FilePickOutcome::Selected {
                bytes,
                filename: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_string(),
            },
        }
    }
}
