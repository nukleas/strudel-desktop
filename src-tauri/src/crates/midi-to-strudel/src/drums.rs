/// General MIDI Drum mapping to Strudel drum sample names
/// Based on GM Level 1 Percussion Key Map (Channel 10)
/// Convert a MIDI drum note number to a Strudel drum sample name
pub fn gm_drum_to_sample(note_num: u8) -> Option<&'static str> {
    match note_num {
        // Bass Drums
        35 => Some("bd"), // Acoustic Bass Drum
        36 => Some("bd"), // Bass Drum 1 (most common)

        // Snare Drums
        37 => Some("rim"), // Side Stick / Rimshot
        38 => Some("sd"), // Acoustic Snare (most common)
        40 => Some("sd"), // Electric Snare

        // Hand Percussion
        39 => Some("cp"), // Hand Clap

        // Toms
        41 => Some("lt"), // Low Floor Tom
        43 => Some("lt"), // High Floor Tom
        45 => Some("mt"), // Low Tom
        47 => Some("mt"), // Low-Mid Tom
        48 => Some("ht"), // Hi-Mid Tom
        50 => Some("ht"), // High Tom

        // Hi-Hats
        42 => Some("hh"), // Closed Hi-Hat (most common)
        44 => Some("hh"), // Pedal Hi-Hat
        46 => Some("oh"), // Open Hi-Hat

        // Cymbals
        49 => Some("cr"), // Crash Cymbal 1
        52 => Some("chin"), // Chinese Cymbal
        55 => Some("cr"), // Splash Cymbal
        57 => Some("cr"), // Crash Cymbal 2

        // Ride
        51 => Some("rd"), // Ride Cymbal 1
        53 => Some("rb"), // Ride Bell
        59 => Some("rd"), // Ride Cymbal 2

        // Percussion
        54 => Some("tambourine"), // Tambourine
        56 => Some("cb"), // Cowbell
        58 => Some("vibraslap"), // Vibraslap

        // Latin Percussion
        60 => Some("bongo"), // Hi Bongo
        61 => Some("bongo"), // Low Bongo
        62 => Some("conga"), // Mute Hi Conga
        63 => Some("conga"), // Open Hi Conga
        64 => Some("conga"), // Low Conga

        65 => Some("timbale"), // High Timbale
        66 => Some("timbale"), // Low Timbale

        67 => Some("agogo"), // High Agogo
        68 => Some("agogo"), // Low Agogo

        69 => Some("cabasa"), // Cabasa
        70 => Some("maracas"), // Maracas

        71 => Some("whistle"), // Short Whistle
        72 => Some("whistle"), // Long Whistle

        73 => Some("guiro"), // Short Guiro
        74 => Some("guiro"), // Long Guiro

        75 => Some("cp"), // Claves (mapped to clap - similar short sound)
        76 => Some("woodblock"), // Hi Wood Block
        77 => Some("woodblock"), // Low Wood Block

        78 => Some("cuica"), // Mute Cuica
        79 => Some("cuica"), // Open Cuica

        80 => Some("triangle"), // Mute Triangle
        81 => Some("triangle"), // Open Triangle

        // Everything else defaults to a generic percussion sound
        _ => None,
    }
}

/// Get the human-readable name for a GM drum note
#[allow(dead_code)]
pub fn gm_drum_name(note_num: u8) -> &'static str {
    match note_num {
        35 => "Acoustic Bass Drum",
        36 => "Bass Drum 1",
        37 => "Side Stick",
        38 => "Acoustic Snare",
        39 => "Hand Clap",
        40 => "Electric Snare",
        41 => "Low Floor Tom",
        42 => "Closed Hi-Hat",
        43 => "High Floor Tom",
        44 => "Pedal Hi-Hat",
        45 => "Low Tom",
        46 => "Open Hi-Hat",
        47 => "Low-Mid Tom",
        48 => "Hi-Mid Tom",
        49 => "Crash Cymbal 1",
        50 => "High Tom",
        51 => "Ride Cymbal 1",
        52 => "Chinese Cymbal",
        53 => "Ride Bell",
        54 => "Tambourine",
        55 => "Splash Cymbal",
        56 => "Cowbell",
        57 => "Crash Cymbal 2",
        58 => "Vibraslap",
        59 => "Ride Cymbal 2",
        60 => "Hi Bongo",
        61 => "Low Bongo",
        62 => "Mute Hi Conga",
        63 => "Open Hi Conga",
        64 => "Low Conga",
        65 => "High Timbale",
        66 => "Low Timbale",
        67 => "High Agogo",
        68 => "Low Agogo",
        69 => "Cabasa",
        70 => "Maracas",
        71 => "Short Whistle",
        72 => "Long Whistle",
        73 => "Short Guiro",
        74 => "Long Guiro",
        75 => "Claves",
        76 => "Hi Wood Block",
        77 => "Low Wood Block",
        78 => "Mute Cuica",
        79 => "Open Cuica",
        80 => "Mute Triangle",
        81 => "Open Triangle",
        _ => "Unknown Drum",
    }
}

/// Detect if a track name suggests it's a drum track
/// Checks for common drum-related keywords
pub fn is_drum_track_name(track_name: &str) -> bool {
    let name_lower = track_name.to_lowercase();

    // Common drum keywords
    let drum_keywords = [
        "kick", "bd", "bass drum",
        "snare", "sd",
        "hat", "hh", "hihat", "hi-hat",
        "cymbal", "crash", "ride",
        "tom", "toms",
        "perc", "percussion",
        "drum", "drums",
        "clap", "snap",
        "rim", "rimshot",
        "cowbell", "clave",
        "shaker", "tambourine", "maracas",
        "bongo", "conga", "timbale",
    ];

    // Check if any keyword is present in the track name
    drum_keywords.iter().any(|&keyword| name_lower.contains(keyword))
}

/// Convert a note name (from note conversion) back to MIDI note number for drum conversion
/// This is needed because we already converted notes to names like "c2", "d2", etc.
pub fn note_name_to_midi_num(note_name: &str) -> Option<u8> {
    // Parse note names like "c2", "d#3", "f#1" back to MIDI numbers
    let note_name = note_name.to_lowercase();
    let bytes = note_name.as_bytes();

    if bytes.is_empty() {
        return None;
    }

    // Get the note (c, d, e, f, g, a, b)
    let note_offset = match bytes[0] as char {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return None,
    };

    let mut pos = 1;
    let mut sharp = 0;

    // Check for sharp
    if pos < bytes.len() && bytes[pos] as char == '#' {
        sharp = 1;
        pos += 1;
    }

    // Get the octave number
    if pos >= bytes.len() {
        return None;
    }

    let octave_str = &note_name[pos..];
    let octave: i32 = octave_str.parse().ok()?;

    // Calculate MIDI note number: (octave + 1) * 12 + note_offset + sharp
    let midi_num = (octave + 1) * 12 + note_offset + sharp;

    if (0..=127).contains(&midi_num) {
        Some(midi_num as u8)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_drums() {
        assert_eq!(gm_drum_to_sample(36), Some("bd")); // Bass drum
        assert_eq!(gm_drum_to_sample(38), Some("sd")); // Snare
        assert_eq!(gm_drum_to_sample(42), Some("hh")); // Closed hi-hat
        assert_eq!(gm_drum_to_sample(46), Some("oh")); // Open hi-hat
        assert_eq!(gm_drum_to_sample(49), Some("cr")); // Crash
    }

    #[test]
    fn test_note_name_conversion() {
        assert_eq!(note_name_to_midi_num("c2"), Some(36)); // C2 = MIDI 36 = Bass Drum
        assert_eq!(note_name_to_midi_num("d2"), Some(38)); // D2 = MIDI 38 = Snare
        assert_eq!(note_name_to_midi_num("f#2"), Some(42)); // F#2 = MIDI 42 = Closed Hi-Hat
        assert_eq!(note_name_to_midi_num("c#3"), Some(49)); // C#3 = MIDI 49 = Crash
    }

    #[test]
    fn test_drum_names() {
        assert_eq!(gm_drum_name(36), "Bass Drum 1");
        assert_eq!(gm_drum_name(38), "Acoustic Snare");
        assert_eq!(gm_drum_name(42), "Closed Hi-Hat");
    }
}
