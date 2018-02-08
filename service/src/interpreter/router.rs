use std::collections::HashMap;

use actix_web::dev::Pattern;
use failure::{Error, ResultExt};
use regex::RegexSet;

use error::ErrorKind;

pub struct Router {
    regset: Option<RegexSet>,
    patterns: Vec<Pattern>,
    raw_patterns: Vec<String>,
    handlers: HashMap<(String, String), String>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            regset: None,
            patterns: Vec::new(),
            raw_patterns: Vec::new(),
            handlers: HashMap::new(),
        }
    }

    pub fn add_route(&mut self, uri: String, method: String,
                     key: String) -> Result<Option<String>, Error> {
        self.add_pattern(uri.clone(), uri.clone()).context(ErrorKind::AddRoute)?;

        if let Some(old_key) = self.handlers.insert((uri, method), key) {
            Ok(Some(old_key))
        } else {
            Ok(None)
        }
    }

    pub fn match_route<'a>(&'a self, uri: &'a str,
                           method: &'a str) -> Option<(String, HashMap<&'a str, &'a str>)> {
        if let Some((name, matches)) = self.match_pattern(uri) {
            let key = match self.handlers.get(&(String::from(name), String::from(method))) {
                Some(v) => v,
                None => return None,
            };
            Some((key.clone(), matches))
        } else {
            None
        }
    }

    fn add_pattern(&mut self, name: String, pattern: String) -> Result<(), Error> {
        let pattern = Pattern::new(&name, &pattern, "/");
        self.patterns.push(pattern.clone());
        self.raw_patterns.push(pattern.pattern().to_owned());

        self.regset = Some(RegexSet::new(&self.raw_patterns).context(
                ErrorKind::RouterCreateRegexSet)?);
        Ok(())
    }

    fn match_pattern<'a>(&'a self, uri: &'a str) -> Option<(String, HashMap<&'a str, &'a str>)> {
        let regset = match self.regset {
            Some(ref regset) => regset,
            None => return None,
        };

        let idx = if uri.is_empty() {
            if let Some(i) = regset.matches("/").into_iter().next() {
                Some(i)
            } else {
                None
            }
        } else if let Some(i) = regset.matches(&uri).into_iter().next() {
            Some(i)
        } else {
            None
        };

        if let Some(idx) = idx {
            let pattern = &self.patterns[idx];
            let name = String::from(pattern.name());
            let matches = pattern.get_match_info(&uri);
            Some((name, matches))
        } else {
            None
        }
    }
}
