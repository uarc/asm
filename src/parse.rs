use super::config::{Config, Capture};
use std::collections::HashMap;
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
    // Left shift amount
    shift: i32,
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
    tags: HashMap<String, Vec<usize>>,
    plus_tags: Vec<(usize, Vec<usize>)>,
    minus_tags: Vec<(usize, Vec<usize>)>,
    replacements: Vec<Replacement>,
}

fn shift_left_or_right(a: u64, shift: i32) -> u64 {
    if shift < 0 {
        a >> (-shift)
    } else {
        a << shift
    }
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
            tags: HashMap::new(),
            plus_tags: Vec::new(),
            minus_tags: Vec::new(),
            replacements: Vec::new(),
        }
    }

    pub fn link(&mut self) {
        // Iterate through every replacement.
        'outer: for r in &self.replacements {
            if r.tag.chars().all(|c| c == '+') {
                for e in &self.plus_tags {
                    // Ensure they have the same amount of pluses and that the relevant segment is at least as high.
                    if e.0 == r.tag.len() && e.1[r.add_segment] >= r.index {
                        self.segments[r.add_segment][r.index] +=
                            shift_left_or_right((e.1[r.pos_segment] as isize + r.pos_offset) as u64, r.shift);

                        break 'outer;
                    }
                }
                panic!("Error: Forward + tag used on line {} was never defined.",
                       r.line);
            } else if r.tag.chars().all(|c| c == '-') {
                for e in self.minus_tags.iter().rev() {
                    // Ensure they have the same amount of minuses and that the relevant segment is at least as low.
                    if e.0 == r.tag.len() && e.1[r.add_segment] < r.index {
                        self.segments[r.add_segment][r.index] +=
                            shift_left_or_right((e.1[r.pos_segment] as isize + r.pos_offset) as u64, r.shift);

                        break 'outer;
                    }
                }
                panic!("Error: Forward + tag used on line {} was never defined.",
                       r.line);
            } else {
                // Get the tag offset vector corresponding to the replacement.
                let tag = self.tags.get(&r.tag).unwrap_or_else(|| {
                    panic!("Error: Tag \"{}\" used on line {} never defined.",
                           r.tag,
                           r.line);
                });
                self.segments[r.add_segment][r.index] +=
                    shift_left_or_right((tag[r.pos_segment] as isize + r.pos_offset) as u64,
                                        r.shift);
            }
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
                self.parse_segment(line, index + 1);
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
                    w.write_all((String::from_iter(bytes[(8 - width)..8]
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
        if self.attempt_tag_create(segment, line) {
            return;
        }
        if self.attempt_rules(segment, line) {
            return;
        }
        panic!("Error: Unrecognized symbol \"{}\" on line {}.",
               segment,
               line);
    }

    fn attempt_tag_create(&mut self, segment: &str, line: usize) -> bool {
        if let Some(caps) = self.config.tag_create.regex.as_ref().unwrap().captures(segment) {
            let s = caps.at(1).unwrap();
            if s.chars().all(|c| c == '+') {
                self.plus_tags.push((s.len(),
                                     self.segments
                    .iter()
                    .map(|v| v.len())
                    .collect()));
            } else if s.chars().all(|c| c == '-') {
                self.minus_tags.push((s.len(),
                                      self.segments
                    .iter()
                    .map(|v| v.len())
                    .collect()));
            } else {
                use std::collections::hash_map::Entry;
                match self.tags.entry(s.to_string()) {
                    Entry::Occupied(_) => {
                        panic!("Attempted to create duplicate tag \"{}\" on line {}",
                               s,
                               line)
                    }
                    Entry::Vacant(v) => {
                        v.insert(self.segments
                            .iter()
                            .map(|v| v.len())
                            .collect());
                    }
                }
            }
            true
        } else {
            false
        }
    }

    fn attempt_rules(&mut self, segment: &str, line: usize) -> bool {
        let config = &self.config;
        for rule in &config.rules {
            if let Some(caps) = rule.regex.as_ref().unwrap().captures(segment) {
                let mut segvals = rule.segment_values.clone();
                for self_reference in &rule.self_references {
                    segvals[self_reference.add_segment][self_reference.add_index] +=
                        if self_reference.shift < 0 {
                            (self.segments[self_reference.from_segment].len() as u64) >>
                            ((-self_reference.shift) as u32)
                        } else {
                            (self.segments[self_reference.from_segment].len() as u64) <<
                            (self_reference.shift as u32)
                        };
                }
                for (index, capture) in rule.captures.iter().enumerate() {
                    use std::mem::transmute;
                    let cap_string = caps.at(index + 1).unwrap();
                    match *capture {
                        Capture::Tag { ref feedbacks } => {
                            for feedback in feedbacks {
                                self.replacements.push(Replacement {
                                    line: line,
                                    shift: feedback.shift,
                                    add_segment: feedback.add_segment,
                                    index: self.segments[feedback.add_segment].len() +
                                           feedback.add_index,
                                    tag: String::from(cap_string),
                                    pos_segment: feedback.from_segment,
                                    pos_offset: if feedback.relative {
                                        feedback.offset -
                                        self.segments[feedback.from_segment].len() as isize
                                    } else {
                                        feedback.offset
                                    },
                                });
                            }
                        }
                        Capture::Str { add_segment } => {
                            for c in cap_string.chars() {
                                self.segments[add_segment].push(c as u64);
                            }
                        }
                        Capture::Num { ref feedbacks, ref base } => {
                            let pval = i64::from_str_radix(cap_string, *base).unwrap_or_else(|e| {
                                panic!("Error: Failed to parse captured string \"{}\" from \
                                        \"{}\": {}",
                                       cap_string,
                                       segment,
                                       e);
                            });
                            let val: u64 = unsafe { transmute(pval) };

                            for feedback in feedbacks {
                                let mut shiftval = shift_left_or_right(val, feedback.shift);
                                if feedback.negate {
                                    shiftval = !shiftval + 1;
                                }
                                if feedback.fill {
                                    let baseval = segvals[feedback.segment][feedback.index];
                                    let fill_amount = shiftval as isize + feedback.fill_offset;
                                    if fill_amount.is_negative() {
                                        panic!("Error: Got a negative fill amount!");
                                    } else if feedback.align {
                                        while self.segments[feedback.segment].len() <
                                              fill_amount as usize {
                                            self.segments[feedback.segment].push(baseval);
                                        }
                                    } else {
                                        for _ in 0..fill_amount {
                                            self.segments[feedback.segment].push(baseval);
                                        }
                                    }
                                    segvals[feedback.segment].pop();
                                } else {
                                    segvals[feedback.segment][feedback.index] += shiftval;
                                }
                            }
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
