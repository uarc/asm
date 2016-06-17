use super::config::Config;
use std::collections::BTreeMap;
use std::io::BufRead;
use itertools::Itertools;

struct Replacement {
    add_segment: usize,
    index: usize,
    tag: String,
    pos_segment: usize,
    pos_offset: isize,
}

pub struct Parser<'a> {
    config: &'a Config,
    segments: Vec<Vec<u64>>,
    tags: BTreeMap<String, Vec<usize>>,
    replacements: Vec<Replacement>,
}

impl<'a> Parser<'a> {
    pub fn new(config: &'a Config) -> Self {
        Parser {
            config: config,
            segments: Vec::new(),
            tags: BTreeMap::new(),
            replacements: Vec::new(),
        }
    }

    pub fn parse<B>(&mut self, bufread: B)
        where B: BufRead
    {
        for (index, line) in bufread.lines().enumerate() {
            let line = line.unwrap_or_else(|e| panic!("Error: Failed to read from buffer: {}", e));

            if self.config.split_whitespace {
                for word in line.split_whitespace() {
                    self.parse_segment(word, index + 1);
                }
            } else {
                self.parse_segment(&line, index + 1);
            }
        }
    }

    pub fn parse_segment(&mut self, segment: &str, line: usize) {
        if self.attempt_tag_create(segment) {
            return;
        }
        if self.attempt_tag_use(segment) {
            return;
        }
        if self.attempt_rules(segment) {
            return;
        }
        panic!("Error: Unrecognized symbol \"{}\" on line {}.",
               segment,
               line);
    }

    fn attempt_tag_create(&mut self, segment: &str) -> bool {
        if let Some(caps) = self.config.tag_create.regex.as_ref().unwrap().captures(segment) {
            let config = &self.config;
            self.tags.insert(caps.at(1).unwrap().to_string(),
                             self.segments
                                 .iter()
                                 .map(|v| v.len())
                                 .collect_vec());
            true
        } else {
            false
        }
    }

    fn attempt_tag_use(&mut self, segment: &str) -> bool {
        let config = &self.config;
        for rule in &config.tag_use_rules {
            if let Some(caps) = rule.regex.as_ref().unwrap().captures(segment) {
                let tag = caps.at(1).unwrap().to_string();
                for feedback in &rule.feedbacks {
                    let index = self.segments[feedback.add_segment].len();
                    let current_pos_index = self.segments[feedback.pos_segment].len();
                    self.replacements.push(Replacement {
                        add_segment: feedback.add_segment,
                        index: index,
                        tag: tag.clone(),
                        pos_segment: feedback.pos_segment,
                        pos_offset: if feedback.relative {
                            feedback.pos_offset - current_pos_index as isize
                        } else {
                            feedback.pos_offset
                        },
                    });

                    // Add the padding
                    self.segments[feedback.add_segment].push(0);
                }
                return true;
            }
        }
        false
    }

    fn attempt_rules(&mut self, segment: &str) -> bool {
        let config = &self.config;
        for rule in &config.rules {
            if let Some(caps) = rule.regex.as_ref().unwrap().captures(segment) {
                let mut segvals = rule.segment_values.clone();
                for (index, capture) in rule.captures.iter().enumerate() {
                    use std::mem::transmute;
                    let cap_string = caps.at(index + 1).unwrap();
                    let pval = i64::from_str_radix(cap_string, capture.base).unwrap_or_else(|e| {
                        panic!("Error: Failed to parse captured string \"{}\" from \"{}\": {}",
                               cap_string,
                               segment,
                               e);
                    });
                    let val: u64 = unsafe { transmute(pval) };

                    for feedback in &capture.feedbacks {
                        let shiftval = val << feedback.shift;
                        if feedback.fill {
                            let baseval = segvals[feedback.segment][index];
                            for _ in 0..(shiftval + feedback.fill_offset as u64) {
                                self.segments[feedback.segment].push(baseval);
                            }
                        } else {
                            segvals[feedback.segment][index] += shiftval;
                        }
                    }
                }
                for (segvec, segment) in segvals.iter_mut().zip(self.segments.iter_mut()) {
                    segment.append(segvec);
                }
                return true;
            }
        }
        false
    }
}
