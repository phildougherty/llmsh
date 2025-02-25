use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::HashMap;

pub struct SuggestionEngine {
    history: Vec<String>,
    frequency_map: HashMap<String, usize>,
    matcher: SkimMatcherV2,
}

impl SuggestionEngine {
    pub fn new() -> Self {
        SuggestionEngine {
            history: Vec::new(),
            frequency_map: HashMap::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn add_command(&mut self, command: &str) {
        self.history.push(command.to_string());
        *self.frequency_map.entry(command.to_string()).or_insert(0) += 1;
    }

    pub fn get_suggestions(&self, partial_input: &str) -> Vec<String> {
        let mut matches: Vec<(i64, String)> = self.history
            .iter()
            .filter_map(|cmd| {
                self.matcher
                    .fuzzy_match(cmd, partial_input)
                    .map(|score| (score, cmd.clone()))
            })
            .collect();

        matches.sort_by(|a, b| b.0.cmp(&a.0));
        matches.into_iter()
            .map(|(_, cmd)| cmd)
            .take(3)
            .collect()
    }
}