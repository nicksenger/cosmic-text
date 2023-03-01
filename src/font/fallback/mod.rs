// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use unicode_script::Script;

use crate::{Font, FontKey};

use self::platform::*;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows",)))]
#[path = "other.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;

#[cfg(target_os = "linux")]
#[path = "unix.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;

pub struct FontFallbackIter<'a> {
    db: &'a fontdb::Database,
    font_keys: &'a [FontKey],
    default_families: &'a [&'a str],
    default_i: usize,
    scripts: Vec<Script>,
    locale: &'a str,
    script_i: (usize, usize),
    common_i: usize,
    other_i: usize,
    end: bool,
}

impl<'a> FontFallbackIter<'a> {
    pub fn new(
        db: &'a fontdb::Database,
        font_keys: &'a [FontKey],
        default_families: &'a [&'a str],
        scripts: Vec<Script>,
        locale: &'a str,
    ) -> Self {
        Self {
            db,
            font_keys,
            default_families,
            default_i: 0,
            scripts,
            locale,
            script_i: (0, 0),
            common_i: 0,
            other_i: 0,
            end: false,
        }
    }

    pub fn check_missing(&self, word: &str) {
        if self.end {
            log::debug!(
                "Failed to find any fallback for {:?} locale '{}': '{}'",
                self.scripts,
                self.locale,
                word
            );
        } else if self.other_i > 0 {
            let font_key = self.font_keys[self.other_i - 1];
            let font = Font::from_key(self.db, font_key).expect("invalid font key");
            log::debug!(
                "Failed to find preset fallback for {:?} locale '{}', used '{}': '{}'",
                self.scripts,
                self.locale,
                font.info.family,
                word
            );
        } else if !self.scripts.is_empty() && self.common_i > 0 {
            let family = common_fallback()[self.common_i - 1];
            log::debug!(
                "Failed to find script fallback for {:?} locale '{}', used '{}': '{}'",
                self.scripts,
                self.locale,
                family,
                word
            );
        }
    }
}

impl<'a> Iterator for FontFallbackIter<'a> {
    type Item = Font<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        while self.default_i < self.default_families.len() {
            let default_family = self.default_families[self.default_i];
            self.default_i += 1;

            for font_key in self.font_keys.iter().copied() {
                let font = Font::from_key(self.db, font_key).expect("invalid font key");
                if font.info.family == default_family {
                    return Some(font);
                }
            }
        }

        while self.script_i.0 < self.scripts.len() {
            let script = self.scripts[self.script_i.0];

            let script_families = script_fallback(script, self.locale);
            while self.script_i.1 < script_families.len() {
                let script_family = script_families[self.script_i.1];
                self.script_i.1 += 1;
                for font_key in self.font_keys.iter().copied() {
                    let font = Font::from_key(self.db, font_key).expect("invalid font key");
                    if font.info.family == script_family {
                        return Some(font);
                    }
                }
                log::debug!(
                    "failed to find family '{}' for script {:?} and locale '{}'",
                    script_family,
                    script,
                    self.locale
                );
            }

            self.script_i.0 += 1;
            self.script_i.1 = 0;
        }

        let common_families = common_fallback();
        while self.common_i < common_families.len() {
            let common_family = common_families[self.common_i];
            self.common_i += 1;
            for font_key in self.font_keys.iter().copied() {
                let font = Font::from_key(self.db, font_key).expect("invalid font key");
                if font.info.family == common_family {
                    return Some(font);
                }
            }
            log::debug!("failed to find family '{}'", common_family);
        }

        //TODO: do we need to do this?
        //TODO: do not evaluate fonts more than once!
        let forbidden_families = forbidden_fallback();
        while self.other_i < self.font_keys.len() {
            let font_key = self.font_keys[self.other_i];
            let font = Font::from_key(self.db, font_key).expect("invalid font key");
            self.other_i += 1;
            if !forbidden_families.contains(&font.info.family.as_str()) {
                return Some(font);
            }
        }

        self.end = true;
        None
    }
}
