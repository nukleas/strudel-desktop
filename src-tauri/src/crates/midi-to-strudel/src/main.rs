use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

use midi_to_strudel::{MidiData, OutputFormatter, TrackBuilder};

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    /// Strudel pattern code (default)
    Strudel,
    /// JSON representation of the AST
    Json,
}

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

    /// Output format (strudel or json)
    #[arg(long, default_value = "strudel")]
    format: OutputFormat,

    /// Minimum track density (0.0-1.0) - filters out sparse tracks
    #[arg(long)]
    min_density: Option<f32>,

    /// Exclude specific channels (comma-separated, e.g., "9,10")
    #[arg(long)]
    exclude_channels: Option<String>,

    /// Only include specific channels (comma-separated)
    #[arg(long)]
    solo_channels: Option<String>,

    /// Maximum number of tracks to include (keeps busiest tracks)
    #[arg(long)]
    max_tracks: Option<usize>,

    /// Detect drums by track name patterns (kick, snare, hat, etc.)
    /// This will treat tracks with drum-related names as drums even if they're not on channel 10
    #[arg(long)]
    detect_drum_names: bool,

    /// Force specific channels to be treated as drums (comma-separated, e.g., "0,1,2")
    /// Use this for MIDI files where drums are on non-standard channels
    #[arg(long)]
    force_drums: Option<String>,
}

fn filter_tracks(mut tracks: Vec<midi_to_strudel::track::ProcessedTrack>, args: &Args) -> Vec<midi_to_strudel::track::ProcessedTrack> {
    // 1. Filter by channel exclusion
    if let Some(exclude) = &args.exclude_channels {
        let excluded: Vec<u8> = exclude
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        tracks.retain(|t| !t.channel.map(|ch| excluded.contains(&ch)).unwrap_or(false));
    }

    // 2. Filter by channel solo
    if let Some(solo) = &args.solo_channels {
        let soloed: Vec<u8> = solo
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        tracks.retain(|t| t.channel.map(|ch| soloed.contains(&ch)).unwrap_or(false));
    }

    // 3. Filter by minimum density
    if let Some(min_density) = args.min_density {
        tracks.retain(|t| {
            let non_empty = t.bars.iter().filter(|b| !b.is_silent()).count();
            let density = non_empty as f32 / t.bars.len() as f32;
            density >= min_density
        });
    }

    // 4. Limit to max tracks (keep busiest)
    if let Some(max) = args.max_tracks {
        if tracks.len() > max {
            // Calculate density for each track
            let mut track_densities: Vec<(usize, f32, midi_to_strudel::track::ProcessedTrack)> = tracks
                .into_iter()
                .enumerate()
                .map(|(idx, t)| {
                    let non_empty = t.bars.iter().filter(|b| !b.is_silent()).count();
                    let density = non_empty as f32 / t.bars.len().max(1) as f32;
                    (idx, density, t)
                })
                .collect();

            // Sort by density (descending)
            track_densities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            // Take top N
            tracks = track_densities.into_iter().take(max).map(|(_, _, t)| t).collect();
        }
    }

    tracks
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Find MIDI file
    let midi_path = if let Some(ref path) = args.midi {
        if !path.exists() {
            anyhow::bail!("MIDI file not found: {}", path.display());
        }
        path.clone()
    } else {
        find_first_midi_file()?
    };

    // Determine output path (use appropriate extension based on format)
    let default_extension = match args.format {
        OutputFormat::Strudel => "strudel",
        OutputFormat::Json => "json",
    };

    let output_path = if let Some(ref path) = args.output {
        path.clone()
    } else {
        let stem = midi_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        PathBuf::from(format!("{}.{}", stem, default_extension))
    };

    if !args.quiet {
        eprintln!("Processing MIDI file: {}", midi_path.display());
    }

    // Parse MIDI file
    let midi_data = MidiData::from_file(&midi_path)?;

    // Parse forced drum channels if provided
    let forced_drum_channels = if let Some(ref force_drums) = args.force_drums {
        force_drums
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    } else {
        Vec::new()
    };

    // Build tracks
    let track_builder = TrackBuilder::new(
        midi_data.cycle_len,
        args.bar_limit,
        args.flat_sequences,
        args.notes_per_bar,
        args.detect_drum_names,
        forced_drum_channels,
    );
    let mut tracks = track_builder.build_tracks(&midi_data.track_info);

    // Apply filters
    tracks = filter_tracks(tracks, &args);

    // Format output based on requested format
    let formatter = OutputFormatter::new(args.tab_size, args.compact);
    let scaled_bpm = midi_data.bpm * args.tempo_scale;
    let output = match args.format {
        OutputFormat::Strudel => formatter.build_output(&tracks, scaled_bpm),
        OutputFormat::Json => formatter.build_output_json(&tracks, scaled_bpm),
    };

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
