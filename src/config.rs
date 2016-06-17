use serde_json::from_reader;
use itertools::Itertools;
use std::fs::File;
use regex::Regex;

#[derive(Deserialize, Debug)]
struct Feedback {
    /// The amount to shift the value to the left before adding.
    shift: u32,
    /// The stream this feedback is applied to.
    segment: usize,
    /// The index of the base value to alter.
    index: usize,
}

#[derive(Deserialize, Debug)]
struct Capture {
    /// The base the number is to be interpreted as.
    base: u32,
    /// All the places the value is inserted in this ruling.
    feedbacks: Vec<Feedback>,
}

#[derive(Deserialize, Debug)]
struct Rule {
    /// The regex including captures for this rule.
    regex_string: String,
    #[serde(skip_deserializing)]
    regex: Option<Regex>,
    /// The unmodified values to be inserted in order into each segment of the output.
    segment_values: Vec<Vec<u64>>,
    /// Capture structs for handling each capture group.
    captures: Vec<Capture>,
}

#[derive(Deserialize, Debug)]
struct TagCreateRule {
    /// The regex which should have exactly one capture group for the tag.
    regex_string: String,
    #[serde(skip_deserializing)]
    regex: Option<Regex>,
}

#[derive(Deserialize, Debug)]
struct TagUseFeedback {
    /// The segment to add the feedback to.
    add_segment: usize,
    /// Whether or not to use relative position.
    relative: bool,
    /// The segment to get the position from to add.
    pos_segment: usize,
}

#[derive(Deserialize, Debug)]
struct TagUseRule {
    regex_string: String,
    #[serde(skip_deserializing)]
    regex: Option<Regex>,
    captures: Vec<Vec<TagUseFeedback>>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    segment_widths: Vec<u32>,
    rules: Vec<Rule>,
}

impl Config {
    pub fn new_from_filename(filename: &str) -> Config {
        // Open file and parse JSON into a Config
        let mut config: Self = from_reader(File::open(filename)
                .unwrap_or_else(|e| panic!("Error: Failed to open config file: {}", e)))
            .unwrap_or_else(|e| panic!("Error: Failed to parse file to JSON: {}", e));

        // Check the config to provide error feedback
        config.consistency_check();

        config
    }

    pub fn consistency_check(&mut self) {
        for rule in &mut self.rules {
            let segment_counts = rule.segment_values.iter().map(|v| v.len()).collect_vec();
            if segment_counts.len() != self.segment_widths.len() {
                panic!("Error: Rule \"{}\" contains an invalid amount of segment values.",
                       rule.regex_string);
            }
            rule.regex = Some(Regex::new(&rule.regex_string)
                .unwrap_or_else(|e| panic!("Error: Failed to parse regex: {}", e)));
            if rule.regex.as_ref().unwrap().captures_len() - 1 != rule.captures.len() {
                panic!("Error: Rule \"{}\" has a different amount of capture structs than its \
                        regex has captures.");
            }
            for capture in &rule.captures {
                for feedback in &capture.feedbacks {
                    let count = *segment_counts.get(feedback.segment)
                        .unwrap_or_else(|| {
                            panic!("Error: Rule \"{}\" attempts to access invalid segment {}.",
                                   rule.regex_string,
                                   feedback.segment)
                        });

                    if feedback.index >= count {
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
