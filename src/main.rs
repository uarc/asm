#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
extern crate serde_json;
extern crate itertools;
extern crate clap;
extern crate regex;
extern crate byteorder;
extern crate rustc_serialize;

use clap::{App, Arg};
use itertools::{Itertools, EitherOrBoth};

mod config;
use config::Config;
mod parse;
use parse::{Parser, OutputFormat};

use std::io::BufReader;
use std::fs::File;

fn main() {
    let matches = App::new("uarc-asm")
        .version("0.1.0")
        .author("Geordon Worley <vadixidav@gmail.com>")
        .about("A configurable, generic assembler")
        .arg(Arg::with_name("format")
            .long("format")
            .short("f")
            .takes_value(true)
            .possible_values(&["little-endian", "big-endian", "hex-list"])
            .default_value("little-endian")
            .help("The format of the output files"))
        .arg(Arg::with_name("config")
            .long("config")
            .short("c")
            .help("Input JSON configuration file")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("outputs")
            .long("outputs")
            .short("o")
            .multiple(true)
            .number_of_values(1)
            .takes_value(true)
            .help("Output file names in order (one at a time)"))
        .arg(Arg::with_name("inputs")
            .index(1)
            .multiple(true)
            .help("List of the input assembly files in the order they are parsed"))
        .get_matches();

    let format = match matches.value_of("format").unwrap() {
        "little-endian" => OutputFormat::LittleEndian,
        "big-endian" => OutputFormat::BigEndian,
        "hex-list" => OutputFormat::HexList,
        v => panic!("Error: \"{}\" is not a valid format.", v),
    };

    let config_filename = matches.value_of("config").unwrap();
    let config = Config::new_from_filename(config_filename);

    let mut parser = Parser::new(&config);

    for name in matches.values_of("inputs")
        .map(|iter| iter.collect())
        .unwrap_or_else(|| Vec::new()) {
        parser.parse(BufReader::new(File::open(&name)
            .unwrap_or_else(|e| panic!("Error: Failed to open input file \"{}\": {}", name, e))));
    }

    // Link the program.
    parser.link();

    let outputs =
        matches.values_of("outputs").map(|iter| iter.collect()).unwrap_or_else(|| Vec::new());
    for (i, name) in (0..config.segment_widths.len())
        .zip_longest(outputs)
        .map(|v| {
            match v {
                EitherOrBoth::Both(_, specified) => specified.into(),
                EitherOrBoth::Left(n) => format!("oseg{}", n),
                EitherOrBoth::Right(specified) => {
                    panic!("Error: Output \"{}\" goes past the amount of segments for this \
                            architecture.",
                           specified);
                }
            }
        })
        .enumerate() {
        parser.output(format,
                      i,
                      &mut File::create(&name).unwrap_or_else(|e| {
                          panic!("Error: Failed to open output file \"{}\": {}", name, e)
                      }));
    }
}
