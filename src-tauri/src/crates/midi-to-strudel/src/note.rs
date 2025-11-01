/// Convert a MIDI note number to a string representation (e.g., "c4", "g#5")
pub fn note_num_to_str(note_num: u8) -> String {
    const NOTE_NAMES: [&str; 12] = [
        "c", "c#", "d", "d#", "e", "f", "f#", "g", "g#", "a", "a#", "b"
    ];

    let note_name = NOTE_NAMES[(note_num % 12) as usize];
    let octave = (note_num / 12) as i32 - 1;

    format!("{}{}", note_name, octave)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_conversion() {
        assert_eq!(note_num_to_str(60), "c4"); // Middle C
        assert_eq!(note_num_to_str(69), "a4"); // A440
        assert_eq!(note_num_to_str(61), "c#4");
    }
}
