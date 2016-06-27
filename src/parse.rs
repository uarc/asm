use super::config::Config;
use std::collections::BTreeMap;
use std::io::{BufRead, Write};

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    LittleEndian,
    BigEndian,
    HexList,
}

struct Replacement {
    // Line for purposes of printing errors.
    line: usize,
    // Segment to add the value to.
    add_segment: usize,
    // The index in the add_segment to add the value.
    index: usize,
    // The tag to retrieve the position to add.
    tag: String,
    // The segment for which we are retrieving the position from the tag.
    pos_segment: usize,
    // The offset we add after retrieving the value from the tag.
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
            segments: {
                let mut v = Vec::new();
                for _ in 0..config.segment_widths.len() {
                    v.push(Vec::new());
                }
                v
            },
            tags: BTreeMap::new(),
            replacements: Vec::new(),
        }
    }

    pub fn link(&mut self) {
        // Iterate through every replacement.
        for r in &self.replacements {
            // Get the tag offset vector corresponding to the replacement.
            let tag = self.tags.get(&r.tag).unwrap_or_else(|| {
                panic!("Error: Tag \"{}\" used on line {} never defined.",
                       r.tag,
                       r.line);
            });
            self.segments[r.add_segment][r.index] =
                (tag[r.pos_segment] as isize + r.pos_offset) as u64;
        }
    }

    pub fn parse<B>(&mut self, bufread: B)
        where B: BufRead
    {
        for (index, line) in bufread.lines().enumerate() {
            let line = line.unwrap_or_else(|e| panic!("Error: Failed to read from buffer: {}", e));

            // Remove everything after the first #, which denotes a comment.
            let line = line.splitn(2, '#').next().unwrap();

            if self.config.split_whitespace {
                for word in line.split_whitespace() {
                    self.parse_segment(word, index + 1);
                }
            } else {
                self.parse_segment(&line, index + 1);
            }
        }
    }

    pub fn output<W>(&self, format: OutputFormat, segment: usize, w: &mut W)
        where W: Write
    {
        use byteorder::{ByteOrder, LittleEndian, BigEndian};
        match format {
            OutputFormat::LittleEndian => {
                // Allocate enough bytes to store a u64
                let mut bytes = [0; 8];
                let width = self.config.segment_widths[segment];
                for val in &self.segments[segment] {
                    LittleEndian::write_u64(&mut bytes, *val);
                    w.write_all(&bytes[0..width]).unwrap_or_else(|e| {
                        panic!("Error: Writing to output file for segment {} failed: {}",
                               segment,
                               e);
                    });
                }
            }
            OutputFormat::BigEndian => {
                // Allocate enough bytes to store a u64
                let mut bytes = [0; 8];
                let width = self.config.segment_widths[segment];
                for val in &self.segments[segment] {
                    BigEndian::write_u64(&mut bytes, *val);
                    w.write_all(&bytes[0..width]).unwrap_or_else(|e| {
                        panic!("Error: Writing to output file for segment {} failed: {}",
                               segment,
                               e);
                    });
                }
            }
            OutputFormat::HexList => {
                use std::iter::FromIterator;
                // Allocate enough bytes to store a u64
                let mut bytes = [0u8; 8];
                let width = self.config.segment_widths[segment];
                for val in &self.segments[segment] {
                    BigEndian::write_u64(&mut bytes, *val);
                    w.write_all(&(String::from_iter(bytes[(8 - width)..8]
                                .iter()
                                .map(|&b| format!("{:02X}", b))) +
                                     "\n")
                            .as_bytes())
                        .unwrap_or_else(|e| {
                            panic!("Error: Writing to output file for segment {} failed: {}",
                                   segment,
                                   e);
                        });
                }
            }
        }
    }

    pub fn parse_segment(&mut self, segment: &str, line: usize) {
        if segment.is_empty() {
            return;
        }
        if self.attempt_tag_create(segment) {
            return;
        }
        if self.attempt_tag_use(segment, line) {
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
            self.tags.insert(caps.at(1).unwrap().to_string(),
                             self.segments
                                 .iter()
                                 .map(|v| v.len())
                                 .collect());
            true
        } else {
            false
        }
    }

    fn attempt_tag_use(&mut self, segment: &str, line: usize) -> bool {
        let config = &self.config;
        for rule in &config.tag_use_rules {
            if let Some(caps) = rule.regex.as_ref().unwrap().captures(segment) {
                let tag = caps.at(1).unwrap().to_string();
                println!("Captured segment \"{}\" in rule \"{:?}\"", segment, rule);
                for feedback in &rule.feedbacks {
                    let index = self.segments[feedback.add_segment].len();
                    let current_pos_index = self.segments[feedback.pos_segment].len();
                    self.replacements.push(Replacement {
                        line: line,
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
                        let mut shiftval = val << feedback.shift;
                        if feedback.negate {
                            shiftval = !shiftval + 1;
                        }
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
