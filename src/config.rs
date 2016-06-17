use serde_json::from_reader;
use itertools::Itertools;
use std::fs::File;

#[derive(Deserialize, Debug)]
struct Feedback {
    // The amount to shift the value to the left before adding.
    shift: u32,
    // The stream this feedback is applied to.
    segment: u32,
    // The index of the base value to alter.
    index: u32,
}

#[derive(Deserialize, Debug)]
struct Capture {
    // The base the number is to be interpreted as.
    base: u32,
    // All the places the value is inserted in this ruling.
    feedbacks: Vec<Feedback>,
}

#[derive(Deserialize, Debug)]
struct Rule {
    regex_string: String,
    segment_values: Vec<Vec<u64>>,
    captures: Vec<Capture>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    segment_widths: Vec<u32>,
    rules: Vec<Rule>,
}

impl Config {
    pub fn new_from_filename(filename: &str) -> Config {
        // Open file and parse JSON into a Config
        let config: Self = from_reader(File::open(filename)
                .unwrap_or_else(|e| panic!("Error: Failed to open config file: {}", e)))
            .unwrap_or_else(|e| panic!("Error: Failed to parse file to JSON: {}", e));

        // Check the config to provide error feedback
        config.consistency_check();

        config
    }

    pub fn consistency_check(&self) {
        for rule in &self.rules {
            let segment_counts = rule.segment_values.iter().map(|v| v.len()).collect_vec();
            if segment_counts.len() != self.segment_widths.len() {
                panic!("Error: Rule \"{}\" contains an invalid amount of segment values.",
                       rule.regex_string);
            }
            for capture in &rule.captures {
                for feedback in &capture.feedbacks {
                    let count = *segment_counts.get(feedback.segment as usize)
                        .unwrap_or_else(|| {
                            panic!("Error: Rule \"{}\" attempts to access invalid segment {}.",
                                   rule.regex_string,
                                   feedback.segment)
                        });

                    if feedback.index as usize >= count {
                        panic!("Error: Rule \"{}\" attempts to access invalid segment value \
                                {}:{}.",
                               rule.regex_string,
                               feedback.segment,
                               feedback.index);
                    }
                }
            }
        }
    }
}
