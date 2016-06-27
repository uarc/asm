use serde_json::from_reader;
use itertools::Itertools;
use std::fs::File;
use regex::Regex;

fn feedback_default_negate() -> bool {
    false
}

fn feedback_default_shift() -> u32 {
    0
}

fn feedback_default_segment() -> usize {
    0
}

fn feedback_default_index() -> usize {
    0
}

fn feedback_default_fill() -> bool {
    false
}

// Default is -1 so that the number specified copies the number this many times and inserts it.
fn feedback_default_fill_offset() -> isize {
    -1
}

#[derive(Deserialize, Debug)]
pub struct NumFeedback {
    /// Should this be negated before using it?
    #[serde(default="feedback_default_negate")]
    pub negate: bool,
    /// The amount to shift the value to the left before adding.
    #[serde(default="feedback_default_shift")]
    pub shift: u32,
    /// The stream this feedback is applied to.
    #[serde(default="feedback_default_segment")]
    pub segment: usize,
    /// The index of the base value to alter.
    #[serde(default="feedback_default_index")]
    pub index: usize,
    /// If this should perform an immediate fill of the amount specified instead of feeding it back.
    #[serde(default="feedback_default_fill")]
    pub fill: bool,
    /// The offset of the fill amount.
    #[serde(default="feedback_default_fill_offset")]
    pub fill_offset: isize,
}

fn tag_feedback_default_relative() -> bool {
    false
}

fn tag_feedback_default_shift() -> u32 {
    0
}

fn tag_feedback_default_offset() -> isize {
    0
}

#[derive(Deserialize, Debug)]
pub struct TagFeedback {
    /// The segment from which to draw the absolute position.
    pub from_segment: usize,
    /// Is this tag usage relative?
    #[serde(default="tag_feedback_default_relative")]
    pub relative: bool,
    /// The amount to left shift the absolute position before adding.
    #[serde(default="tag_feedback_default_shift")]
    pub shift: u32,
    /// The segment to add the absolute position to.
    pub add_segment: usize,
    /// The index at which to add the absolute position.
    pub add_index: usize,
    /// An offset to add.
    #[serde(default="tag_feedback_default_offset")]
    pub offset: isize,
}

#[derive(Deserialize, Debug)]
pub enum Capture {
    Tag {
        feedbacks: Vec<TagFeedback>,
    },
    Num {
        /// The base the number is to be interpreted as.
        base: u32,
        /// All the places the value is inserted in this ruling.
        feedbacks: Vec<NumFeedback>,
    },
}

#[derive(Deserialize, Debug)]
pub struct Rule {
    /// The regex including captures for this rule.
    regex_string: String,
    #[serde(skip_deserializing)]
    pub regex: Option<Regex>,
    /// The unmodified values to be inserted in order into each segment of the output.
    pub segment_values: Vec<Vec<u64>>,
    /// The additions of the absolute position back into the segment values (relative ignored).
    #[serde(default)]
    pub self_references: Vec<TagFeedback>,
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
pub struct Config {
    /// The widths of each output segment in octets.
    pub segment_widths: Vec<usize>,
    /// If the regexes are on a word basis
    pub split_whitespace: bool,
    /// The rule for creating tags.
    pub tag_create: TagCreateRule,
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
        // for tag_use in &mut self.tag_use_rules {
        // tag_use.regex = Some(Regex::new(&tag_use.regex_string).unwrap_or_else(|e| {
        // panic!("Error: Failed to parse tag use regex \"{}\": {}",
        // tag_use.regex_string,
        // e)
        // }));
        // if tag_use.regex.as_ref().unwrap().captures_len() != 2 {
        // panic!("Error: The tag use regex \"{}\" must always have one capture group for \
        // the tag.",
        // tag_use.regex_string);
        // }
        // for feedback in &tag_use.feedbacks {
        // if feedback.add_segment >= self.segment_widths.len() {
        // panic!("Error: A feedback in the tag use \"{}\" struct uses a non-existent \
        // add segment.",
        // tag_use.regex_string);
        // }
        // if feedback.pos_segment >= self.segment_widths.len() {
        // panic!("Error: A feedback in the tag use \"{}\" struct uses a non-existent \
        // pos segment.",
        // tag_use.regex_string);
        // }
        // }
        // }
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
                        regex has captures.",
                       rule.regex_string);
            }
            for self_reference in &rule.self_references {
                if self_reference.from_segment >= self.segment_widths.len() {
                    panic!("Error: Rule \"{}\" attempts to self-reference an invalid segment {}.",
                           rule.regex_string,
                           self_reference.from_segment);
                }
                if self_reference.add_segment >= segment_counts.len() {
                    panic!("Error: Rule \"{}\" attempts to add a self-reference to an invalid \
                            segment {}.",
                           rule.regex_string,
                           self_reference.add_segment);
                }
                if self_reference.add_index >= segment_counts[self_reference.add_segment] {
                    panic!("Error: Rule \"{}\" attempts to add a self-reference to an invalid \
                            index {} of segment {}.",
                           rule.regex_string,
                           self_reference.add_index,
                           self_reference.add_segment);
                }
            }
            for capture in &rule.captures {
                match capture {
                    &Capture::Tag { ref feedbacks } => {
                        for feedback in feedbacks {
                            if feedback.from_segment >= segment_counts.len() {
                                panic!("Error: Rule \"{}\" attempts to access invalid tag \
                                        segment {}.",
                                       rule.regex_string,
                                       feedback.from_segment);
                            }
                            if feedback.add_segment >= segment_counts.len() {
                                panic!("Error: Rule \"{}\" attempts to access invalid feedback \
                                        segment {}.",
                                       rule.regex_string,
                                       feedback.add_segment);
                            }
                            if feedback.add_index >= segment_counts[feedback.add_segment] {
                                panic!("Error: Rule \"{}\" attempts to access invalid index {} \
                                        in segment {}.",
                                       rule.regex_string,
                                       feedback.add_index,
                                       feedback.add_segment);
                            }
                        }
                    }
                    &Capture::Num { ref feedbacks, .. } => {
                        for feedback in feedbacks {
                            let count = *segment_counts.get(feedback.segment)
                                .unwrap_or_else(|| {
                                    panic!("Error: Rule \"{}\" attempts to access invalid \
                                            segment {}.",
                                           rule.regex_string,
                                           feedback.segment);
                                });
                            if feedback.index >= count {
                                panic!("Error: Rule \"{}\" attempts to access invalid segment \
                                        value {}:{}.",
                                       rule.regex_string,
                                       feedback.segment,
                                       feedback.index);
                            }
                        }
                    }
                }
            }
        }
    }
}
