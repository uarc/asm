use serde_json::from_reader;
use itertools::Itertools;
use std::fs::File;
use regex::Regex;

#[derive(Deserialize, Debug)]
pub struct Feedback {
    /// Should this be negated before using it?
    pub negate: bool,
    /// The amount to shift the value to the left before adding.
    pub shift: u32,
    /// The stream this feedback is applied to.
    pub segment: usize,
    /// The index of the base value to alter.
    pub index: usize,
    /// If this should perform an immediate fill of the amount specified instead of feeding it back.
    pub fill: bool,
    /// The offset of the fill amount.
    pub fill_offset: isize,
}

#[derive(Deserialize, Debug)]
pub struct Capture {
    /// The base the number is to be interpreted as.
    pub base: u32,
    /// All the places the value is inserted in this ruling.
    pub feedbacks: Vec<Feedback>,
}

#[derive(Deserialize, Debug)]
pub struct Rule {
    /// The regex including captures for this rule.
    regex_string: String,
    #[serde(skip_deserializing)]
    pub regex: Option<Regex>,
    /// The unmodified values to be inserted in order into each segment of the output.
    pub segment_values: Vec<Vec<u64>>,
    /// Capture structs for handling each capture group.
    pub captures: Vec<Capture>,
}

#[derive(Deserialize, Debug)]
pub struct TagCreateRule {
    /// The regex which should have exactly one capture group for the tag.
    regex_string: String,
    #[serde(skip_deserializing)]
    pub regex: Option<Regex>,
}

#[derive(Deserialize, Debug)]
pub struct TagUseFeedback {
    /// The segment to add the feedback to.
    pub add_segment: usize,
    /// Whether or not to use relative position.
    pub relative: bool,
    /// The segment to get the position from to add.
    pub pos_segment: usize,
    /// An offset to add to the position.
    pub pos_offset: isize,
}

#[derive(Deserialize, Debug)]
pub struct TagUseRule {
    /// The regex which should have exactly one capture group for the tag.
    regex_string: String,
    #[serde(skip_deserializing)]
    pub regex: Option<Regex>,
    /// Feedback of the tag's data into the segments.
    pub feedbacks: Vec<TagUseFeedback>,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    /// The widths of each output segment in octets.
    pub segment_widths: Vec<usize>,
    /// If the regexes are on a word basis
    pub split_whitespace: bool,
    /// The rule for creating tags.
    pub tag_create: TagCreateRule,
    /// The rule for using tags.
    pub tag_use_rules: Vec<TagUseRule>,
    /// The rules for everything else.
    pub rules: Vec<Rule>,
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
        for width in &self.segment_widths {
            if *width == 0 {
                panic!("Error: A segment width of 0 is not allowed.");
            }
        }
        self.tag_create.regex = Some(Regex::new(&self.tag_create.regex_string)
            .unwrap_or_else(|e| panic!("Error: Failed to parse tag create regex: {}", e)));
        if self.tag_create.regex.as_ref().unwrap().captures_len() != 2 {
            panic!("Error: The tag create regex must always have one capture group for the tag.");
        }
        for tag_use in &mut self.tag_use_rules {
            tag_use.regex = Some(Regex::new(&tag_use.regex_string).unwrap_or_else(|e| {
                panic!("Error: Failed to parse tag use regex \"{}\": {}",
                       tag_use.regex_string,
                       e)
            }));
            if tag_use.regex.as_ref().unwrap().captures_len() != 2 {
                panic!("Error: The tag use regex \"{}\" must always have one capture group for \
                        the tag.",
                       tag_use.regex_string);
            }
            for feedback in &tag_use.feedbacks {
                if feedback.add_segment >= self.segment_widths.len() {
                    panic!("Error: A feedback in the tag use \"{}\" struct uses a non-existent \
                            add segment.",
                           tag_use.regex_string);
                }
                if feedback.pos_segment >= self.segment_widths.len() {
                    panic!("Error: A feedback in the tag use \"{}\" struct uses a non-existent \
                            pos segment.",
                           tag_use.regex_string);
                }
            }
        }
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
