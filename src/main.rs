#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
extern crate serde_json;
extern crate itertools;
extern crate clap;
extern crate regex;

use clap::{App, Arg};
mod config;
use config::Config;
mod parse;
use parse::Parser;

#[derive(Debug)]
enum OutputFormat {
    LittleEndian,
    BigEndian,
    HexList,
}

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
            .help("List of the output file names"))
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

    println!("Format: {:?}, Config: {:?}",
             format,
             Config::new_from_filename(config_filename));
}
