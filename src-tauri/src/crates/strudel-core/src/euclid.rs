/// Euclidean rhythm generation using the Bjorklund algorithm
///
/// This module implements the Bjorklund algorithm for generating
/// Euclidean rhythms, which distributes pulses as evenly as possible
/// across a given number of steps.
/// Generate a Euclidean rhythm pattern using the Bjorklund algorithm
///
/// # Arguments
/// * `pulse` - Number of pulses (onsets) in the pattern
/// * `step` - Total number of steps in the pattern
/// * `rotation` - Number of steps to rotate the pattern
///
/// # Returns
/// A vector of booleans where `true` represents a pulse and `false` represents a rest
///
/// # Examples
/// ```
/// use strudel_core::euclid::bjorklund;
///
/// // Classic 3-against-8 pattern
/// let pattern = bjorklund(3, 8, 0);
/// assert_eq!(pattern.len(), 8);
/// assert_eq!(pattern.iter().filter(|&&x| x).count(), 3);
/// ```
pub fn bjorklund(pulse: usize, step: usize, rotation: usize) -> Vec<bool> {
    // Edge cases
    if step == 0 {
        return Vec::new();
    }

    if pulse == 0 {
        return vec![false; step];
    }

    if pulse >= step {
        return vec![true; step];
    }

    // Initialize pattern with pulses at the start
    let mut pattern = vec![true; pulse];
    pattern.extend(vec![false; step - pulse]);

    // Bjorklund algorithm
    let mut groups: Vec<Vec<bool>> = pattern.into_iter().map(|b| vec![b]).collect();

    loop {
        // Count groups that can be paired
        let ones = groups.iter().filter(|g| g.iter().all(|&x| x)).count();
        let zeros = groups.len() - ones;

        if zeros <= 1 {
            break;
        }

        let pairs = ones.min(zeros);

        if pairs == 0 {
            break;
        }

        // Pair groups
        let mut new_groups = Vec::new();

        for i in 0..pairs {
            let mut combined = groups[i].clone();
            combined.extend_from_slice(&groups[ones + i]);
            new_groups.push(combined);
        }

        // Add remaining groups
        for group in groups.iter().take(ones).skip(pairs) {
            new_groups.push(group.clone());
        }

        for group in groups.iter().skip(ones + pairs) {
            new_groups.push(group.clone());
        }

        groups = new_groups;
    }

    // Flatten groups into pattern
    let mut result: Vec<bool> = groups.into_iter().flatten().collect();

    // Apply rotation
    if rotation > 0 && !result.is_empty() {
        let rot = rotation % result.len();
        result.rotate_left(rot);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bjorklund_empty() {
        let pattern = bjorklund(0, 8, 0);
        assert_eq!(pattern, vec![false; 8]);
    }

    #[test]
    fn test_bjorklund_full() {
        let pattern = bjorklund(8, 8, 0);
        assert_eq!(pattern, vec![true; 8]);
    }

    #[test]
    fn test_bjorklund_3_8() {
        let pattern = bjorklund(3, 8, 0);
        assert_eq!(pattern.len(), 8);
        assert_eq!(pattern.iter().filter(|&&x| x).count(), 3);
        // Should be [T, F, F, T, F, F, T, F] or similar even distribution
    }

    #[test]
    fn test_bjorklund_5_8() {
        let pattern = bjorklund(5, 8, 0);
        assert_eq!(pattern.len(), 8);
        assert_eq!(pattern.iter().filter(|&&x| x).count(), 5);
    }

    #[test]
    fn test_bjorklund_rotation() {
        let pattern1 = bjorklund(3, 8, 0);
        let pattern2 = bjorklund(3, 8, 1);

        assert_eq!(pattern1.len(), pattern2.len());
        assert_ne!(pattern1, pattern2); // Should be different due to rotation
    }

    #[test]
    fn test_bjorklund_zero_steps() {
        let pattern = bjorklund(0, 0, 0);
        assert_eq!(pattern, Vec::<bool>::new());
    }

    #[test]
    fn test_bjorklund_pulse_exceeds_steps() {
        let pattern = bjorklund(10, 8, 0);
        assert_eq!(pattern, vec![true; 8]);
    }
}
