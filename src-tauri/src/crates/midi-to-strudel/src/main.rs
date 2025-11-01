mod drums;
mod instruments;
mod midi;
mod note;
mod output;
mod track;

use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

use midi::MidiData;
use output::OutputFormatter;
use track::TrackBuilder;

#[derive(Parser, Debug)]
#[command(name = "midi-to-strudel")]
#[command(about = "Convert MIDI files to Strudel code", long_about = None)]
struct Args {
    /// Path to the MIDI file (default: uses first .mid file in current directory)
    #[arg(short, long)]
    midi: Option<PathBuf>,

    /// Output file path (default: `<midi-name>.strudel`)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Print output to stdout instead of file
    #[arg(long)]
    stdout: bool,

    /// Suppress informational messages (only errors)
    #[arg(short, long)]
    quiet: bool,

    /// The amount of bars to convert. 0 means no limit.
    #[arg(short, long, default_value = "0")]
    bar_limit: usize,

    /// No complex timing or chords
    #[arg(short, long, default_value = "false")]
    flat_sequences: bool,

    /// How many spaces to use for indentation in the output
    #[arg(short, long, default_value = "2")]
    tab_size: usize,

    /// The resolution. Usually in steps of 4 (4, 8, 16...).
    /// Higher gives better note placement but can get big.
    #[arg(short, long, default_value = "64")]
    notes_per_bar: usize,

    /// Tempo scaling factor (e.g., 0.5 for half-speed, 2.0 for double-speed)
    #[arg(long, default_value = "1.0")]
    tempo_scale: f64,

    /// Compress repetitive patterns using replication operator (!)
    #[arg(short, long, default_value = "false")]
    compact: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Find MIDI file
    let midi_path = if let Some(path) = args.midi {
        if !path.exists() {
            anyhow::bail!("MIDI file not found: {}", path.display());
        }
        path
    } else {
        find_first_midi_file()?
    };

    // Determine output path (use .strudel extension)
    let output_path = if let Some(path) = args.output {
        path
    } else {
        let stem = midi_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        PathBuf::from(format!("{}.strudel", stem))
    };

    if !args.quiet {
        eprintln!("Processing MIDI file: {}", midi_path.display());
    }

    // Parse MIDI file
    let midi_data = MidiData::from_file(&midi_path)?;

    // Build tracks
    let track_builder = TrackBuilder::new(
        midi_data.cycle_len,
        args.bar_limit,
        args.flat_sequences,
        args.notes_per_bar,
    );
    let tracks = track_builder.build_tracks(&midi_data.track_info);

    // Format output
    let formatter = OutputFormatter::new(args.tab_size, args.compact);
    let scaled_bpm = midi_data.bpm * args.tempo_scale;
    let output = formatter.build_output(&tracks, scaled_bpm);

    // Output handling
    if args.stdout {
        // Print directly to stdout (clean, no logs)
        println!("{}", output);
    } else {
        // Write to file
        fs::write(&output_path, format!("{}\n", output))
            .with_context(|| format!("Failed to write {}", output_path.display()))?;

        if !args.quiet {
            eprintln!("Output saved to {}", output_path.display());
        }
    }

    Ok(())
}

fn find_first_midi_file() -> Result<PathBuf> {
    let entries = fs::read_dir(".")
        .context("Failed to read current directory")?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("mid") {
            return Ok(path);
        }
    }

    anyhow::bail!("No MIDI files found in current directory")
}
