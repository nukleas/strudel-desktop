use clap::{Parser, Subcommand};
use anyhow::Result;
use strudel_mini::{parse, format, evaluate, extract_patterns, combine_patterns, CombineStrategy};
use strudel_core::{Fraction, State, TimeSpan};

#[derive(Parser)]
#[command(name = "strudel-mini")]
#[command(about = "Mini notation parser and validator for Strudel", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a mini notation pattern
    Validate {
        /// Pattern to validate
        pattern: String,
    },
    /// Format a mini notation pattern (TODO)
    Fmt {
        /// Pattern to format
        pattern: String,
    },
    /// Generate AST for a pattern
    Ast {
        /// Pattern to parse
        pattern: String,

        /// Output format (json or debug)
        #[arg(short, long, default_value = "debug")]
        output_format: String,
    },
    /// Evaluate a pattern and show events
    Eval {
        /// Pattern to evaluate
        pattern: String,

        /// Start cycle (default: 0)
        #[arg(short, long, default_value = "0")]
        from: f64,

        /// Duration in cycles (default: 1)
        #[arg(short, long, default_value = "1")]
        duration: f64,

        /// Output format (json or debug)
        #[arg(long, default_value = "debug")]
        format: String,
    },
    /// Extract mini notation patterns from a .strudel file
    Extract {
        /// Path to .strudel file
        file: String,

        /// Combine strategy (stack, sequence, first, separate)
        #[arg(short, long, default_value = "separate")]
        strategy: String,
    },
    /// Play a pattern using audio output
    #[cfg(feature = "audio")]
    Play {
        /// Pattern to play (or use --file)
        #[arg(required_unless_present_any = ["file", "strudel_file"])]
        pattern: Option<String>,

        /// Read mini notation pattern from file
        #[arg(short, long, conflicts_with_all = ["pattern", "strudel_file"])]
        file: Option<String>,

        /// Read and extract from .strudel file
        #[arg(short = 's', long, conflicts_with_all = ["pattern", "file"])]
        strudel_file: Option<String>,

        /// Combine strategy for .strudel files (stack, sequence, first)
        #[arg(long, default_value = "stack", requires = "strudel_file")]
        combine: String,

        /// Tempo in BPM (default: 120)
        #[arg(short, long, default_value = "120")]
        tempo: f64,

        /// Duration in seconds (default: 10)
        #[arg(short, long, default_value = "10")]
        duration: f64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { pattern } => {
            match parse(&pattern) {
                Ok(_) => {
                    println!("✓ Pattern is valid");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Fmt { pattern } => {
            match parse(&pattern) {
                Ok(ast) => {
                    let formatted = format(&ast);
                    println!("{}", formatted);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Ast { pattern, output_format } => {
            match parse(&pattern) {
                Ok(ast) => {
                    match output_format.as_str() {
                        "json" => {
                            let json = serde_json::to_string_pretty(&ast)?;
                            println!("{}", json);
                        }
                        _ => {
                            println!("{:#?}", ast);
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Eval { pattern, from, duration, format } => {
            match parse(&pattern) {
                Ok(ast) => {
                    match evaluate(&ast) {
                        Ok(pat) => {
                            let begin = Fraction::from_float(from);
                            let end = Fraction::from_float(from + duration);
                            let span = TimeSpan::new(begin, end);
                            let state = State::new(span);

                            let haps = pat.query(state);

                            match format.as_str() {
                                "json" => {
                                    let json = serde_json::to_string_pretty(&haps)?;
                                    println!("{}", json);
                                }
                                _ => {
                                    println!("Events: {}", haps.len());
                                    for (i, hap) in haps.iter().enumerate() {
                                        println!("  [{}] {:?}", i, hap);
                                    }
                                }
                            }
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("✗ Evaluation error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Extract { file, strategy } => {
            use std::fs;

            // Read the .strudel file
            let source = fs::read_to_string(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", file, e))?;

            // Extract patterns
            let extracted = extract_patterns(&source);

            if extracted.is_empty() {
                println!("No patterns found in {}", file);
                return Ok(());
            }

            println!("Found {} pattern(s) in {}\n", extracted.len(), file);

            // Parse strategy
            let combine_strategy = match strategy.as_str() {
                "stack" => CombineStrategy::Stack,
                "sequence" => CombineStrategy::Sequence,
                "first" => CombineStrategy::First,
                _ => CombineStrategy::Separate,
            };

            // Combine and output
            let result = combine_patterns(&extracted, combine_strategy);
            println!("{}", result);

            Ok(())
        }
        #[cfg(feature = "audio")]
        Commands::Play { pattern, file, strudel_file, combine, tempo, duration } => {
            use strudel_audio::{Player, PlayerConfig};
            use std::thread;
            use std::time::Duration as StdDuration;
            use std::fs;

            println!("Strudel Audio Player");
            println!("====================\n");

            // Get pattern from file, strudel file, or argument
            let pattern_str = if let Some(strudel_path) = strudel_file {
                println!("Reading .strudel file: {}", strudel_path);
                let source = fs::read_to_string(&strudel_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", strudel_path, e))?;

                // Extract patterns
                let extracted = extract_patterns(&source);

                if extracted.is_empty() {
                    return Err(anyhow::anyhow!("No patterns found in {}", strudel_path));
                }

                println!("Extracted {} pattern(s)", extracted.len());

                // Parse combine strategy
                let combine_strategy = match combine.as_str() {
                    "sequence" => CombineStrategy::Sequence,
                    "first" => CombineStrategy::First,
                    _ => CombineStrategy::Stack,
                };

                combine_patterns(&extracted, combine_strategy)
            } else if let Some(file_path) = file {
                println!("Reading pattern from: {}", file_path);
                fs::read_to_string(&file_path)
                    .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", file_path, e))?
            } else {
                pattern.unwrap()
            };

            // Parse the pattern
            let ast = parse(&pattern_str)?;
            let pat = evaluate(&ast)?;

            println!("Pattern parsed successfully!");
            println!("Tempo: {} BPM", tempo);
            println!("Duration: {} seconds\n", duration);

            // Create player with custom tempo
            let config = PlayerConfig {
                tempo,
                ..Default::default()
            };

            let player = Player::new(config)
                .map_err(|e| anyhow::anyhow!("Failed to create audio player: {}", e))?;

            println!("Starting playback...");
            player.play(pat)
                .map_err(|e| anyhow::anyhow!("Failed to start playback: {}", e))?;

            // Play for the specified duration
            thread::sleep(StdDuration::from_secs_f64(duration));

            player.stop()
                .map_err(|e| anyhow::anyhow!("Failed to stop playback: {}", e))?;

            println!("\nPlayback finished!");
            Ok(())
        }
    }
}
