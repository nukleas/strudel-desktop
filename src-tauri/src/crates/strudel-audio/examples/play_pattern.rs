//! Example: Play a simple drum pattern

use strudel_audio::Player;
use strudel_core::{pure, sequence, Value};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Strudel Audio Playback Example");
    println!("===============================\n");

    // Create a player with default configuration
    let player = Player::with_defaults()?;

    println!("Audio player created successfully!");
    println!("Sample rate: {} Hz\n", 44100);

    // Create a simple bass drum pattern
    // This creates a pattern that plays "bd" on beats 0, 1, 2, 3
    let bd_pattern = sequence(vec![
        pure(Value::String("bd".to_string())),
        pure(Value::String("bd".to_string())),
        pure(Value::String("bd".to_string())),
        pure(Value::String("bd".to_string())),
    ]);

    println!("Playing a simple 4-on-the-floor bass drum pattern...");
    println!("(This would play if samples were loaded)\n");

    // Try to play (will fail gracefully if no samples are available)
    match player.play(bd_pattern) {
        Ok(_) => {
            println!("Playback started! Press Ctrl+C to stop.");
            // Play for 10 seconds
            thread::sleep(Duration::from_secs(10));
            player.stop()?;
            println!("\nPlayback stopped.");
        }
        Err(e) => {
            println!("Note: Playback not available yet (no samples bundled)");
            println!("Error: {}", e);
            println!("\nNext steps:");
            println!("1. Download sample files");
            println!("2. Bundle them with the crate");
            println!("3. Try again!");
        }
    }

    Ok(())
}
