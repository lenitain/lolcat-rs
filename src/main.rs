mod color;
mod process;

use lexopt::prelude::*;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, IsTerminal};

struct Args {
    files: Vec<String>,
    freq: f64,
    spread: f64,
    gradient: Option<String>,
    smoothness: f64,
    interpolate: color::InterpolationMode,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    let mut files = Vec::new();
    let mut freq = 0.1;
    let mut spread = 2.6;
    let mut gradient = None;
    let mut smoothness = 100.0;
    let mut interpolate = color::InterpolationMode::Oklch;
    let mut parser = lexopt::Parser::from_env();

    while let Some(arg) = parser.next()? {
        match arg {
            Short('f') | Long("freq") => {
                freq = parser.value()?.parse()?;
            }
            Short('s') | Long("spread") => {
                spread = parser.value()?.parse()?;
            }
            Short('g') | Long("gradient") => {
                gradient = Some(parser.value()?.into_string()?);
            }
            Short('m') | Long("smoothness") => {
                smoothness = parser.value()?.parse()?;
            }
            Short('i') | Long("interpolate") => {
                let val = parser.value()?.into_string()?;
                interpolate = match val.as_str() {
                    "linear" => color::InterpolationMode::Linear,
                    "oklch" => color::InterpolationMode::Oklch,
                    "cubic" => color::InterpolationMode::Cubic,
                    _ => {
                        return Err(lexopt::Error::from(format!(
                            "Unknown interpolation mode: {}",
                            val
                        )));
                    }
                };
            }
            Short('h') | Long("help") => {
                println!("lolcat-rs - A high-performance, vibrant rainbow coloring tool");
                println!("\nUsage: lolcat-rs [OPTIONS] [FILES]...");
                println!("\nOptions:");
                println!("  -f, --freq <FREQ>         Rainbow frequency [default: 0.1]");
                println!("  -s, --spread <SPREAD>     Rainbow spread [default: 2.6]");
                println!("  -g, --gradient <GRADIENT> Gradient color stops [default: rainbow]");
                println!("                            Format: POS:COLOR[,POS:COLOR...]");
                println!(
                    "                            Example: \"0:#ff0000,50:#00ff00,100:#0000ff\""
                );
                println!("  -m, --smoothness <VAL>    Smoothness (0-100) [default: 100]");
                println!(
                    "  -i, --interpolate <MODE>  Interpolation: linear, oklch, cubic [default: oklch]"
                );
                println!("  -h, --help                Print help");
                std::process::exit(0);
            }
            Value(val) => {
                files.push(val.into_string()?);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    Ok(Args {
        files,
        freq,
        spread,
        gradient,
        smoothness,
        interpolate,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let is_atty = io::stdout().is_terminal();
    let mut writer = BufWriter::new(io::stdout().lock());

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let mut x = (d.as_nanos() as u64) ^ (std::process::id() as u64).rotate_left(32);
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            ((x ^ (x >> 31)) % 256) as f64
        })
        .unwrap_or(0.0);

    let gradient = match args.gradient {
        Some(ref spec) => color::parse_gradient(spec)?,
        None => color::default_rainbow(),
    };

    let custom_gradient = args.gradient.is_some();

    let mut generator = color::ColorGenerator {
        freq: args.freq,
        spread: args.spread,
        seed,
        line_idx: 0,
        gradient,
        smoothness: args.smoothness / 100.0,
        interpolate: args.interpolate,
        custom_gradient,
    };

    for path in args.files {
        if path == "-" {
            process::process_input(io::stdin().lock(), &mut writer, &mut generator, is_atty)?;
        } else {
            let file = File::open(path)?;
            process::process_input(BufReader::new(file), &mut writer, &mut generator, is_atty)?;
        }
    }
    Ok(())
}
