use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// Rational number representation for precise timing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fraction {
    pub numerator: i64,
    pub denominator: i64,
}

impl Fraction {
    /// Create a new fraction and simplify it
    pub fn new(numerator: i64, denominator: i64) -> Self {
        if denominator == 0 {
            panic!("Denominator cannot be zero");
        }
        let mut f = Fraction {
            numerator,
            denominator,
        };
        f.simplify();
        f
    }

    /// Create a fraction from a whole number
    pub fn from_int(n: i64) -> Self {
        Fraction {
            numerator: n,
            denominator: 1,
        }
    }

    /// Create a fraction from a float (approximation)
    pub fn from_float(f: f64) -> Self {
        // Simple approximation - use 1000000 as denominator
        let n = (f * 1_000_000.0).round() as i64;
        Fraction::new(n, 1_000_000)
    }

    /// Convert to float
    pub fn to_float(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    /// Simplify the fraction
    fn simplify(&mut self) {
        let gcd = Self::gcd(self.numerator.abs(), self.denominator.abs());
        self.numerator /= gcd;
        self.denominator /= gcd;

        // Keep denominator positive
        if self.denominator < 0 {
            self.numerator = -self.numerator;
            self.denominator = -self.denominator;
        }
    }

    /// Greatest common divisor
    fn gcd(mut a: i64, mut b: i64) -> i64 {
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a
    }

    /// Least common multiple
    pub fn lcm(a: i64, b: i64) -> i64 {
        (a * b) / Self::gcd(a, b)
    }

    /// Get the reciprocal
    pub fn reciprocal(self) -> Self {
        Fraction::new(self.denominator, self.numerator)
    }

    /// Check if fraction is zero
    pub fn is_zero(&self) -> bool {
        self.numerator == 0
    }

    /// Check if fraction is negative
    pub fn is_negative(&self) -> bool {
        self.numerator < 0
    }

    /// Absolute value
    pub fn abs(self) -> Self {
        Fraction::new(self.numerator.abs(), self.denominator)
    }

    /// Floor - round down to nearest integer
    pub fn floor(self) -> Self {
        let result = self.numerator / self.denominator;
        Fraction::from_int(result)
    }

    /// Ceiling - round up to nearest integer
    pub fn ceil(self) -> Self {
        let result = (self.numerator + self.denominator - 1) / self.denominator;
        Fraction::from_int(result)
    }
}

impl fmt::Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

impl From<i64> for Fraction {
    fn from(n: i64) -> Self {
        Fraction::from_int(n)
    }
}

impl From<f64> for Fraction {
    fn from(f: f64) -> Self {
        Fraction::from_float(f)
    }
}

impl From<(i64, i64)> for Fraction {
    fn from((num, den): (i64, i64)) -> Self {
        Fraction::new(num, den)
    }
}

impl Add for Fraction {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let lcm = Self::lcm(self.denominator, other.denominator);
        let num1 = self.numerator * (lcm / self.denominator);
        let num2 = other.numerator * (lcm / other.denominator);
        Fraction::new(num1 + num2, lcm)
    }
}

impl Sub for Fraction {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let lcm = Self::lcm(self.denominator, other.denominator);
        let num1 = self.numerator * (lcm / self.denominator);
        let num2 = other.numerator * (lcm / other.denominator);
        Fraction::new(num1 - num2, lcm)
    }
}

impl Mul for Fraction {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fraction::new(
            self.numerator * other.numerator,
            self.denominator * other.denominator,
        )
    }
}

impl Div for Fraction {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, other: Self) -> Self {
        // Division is multiplication by the reciprocal
        self * other.reciprocal()
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        let lcm = Self::lcm(self.denominator, other.denominator);
        let num1 = self.numerator * (lcm / self.denominator);
        let num2 = other.numerator * (lcm / other.denominator);
        num1.cmp(&num2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraction_creation() {
        let f = Fraction::new(1, 2);
        assert_eq!(f.numerator, 1);
        assert_eq!(f.denominator, 2);
    }

    #[test]
    fn test_fraction_simplification() {
        let f = Fraction::new(4, 8);
        assert_eq!(f.numerator, 1);
        assert_eq!(f.denominator, 2);
    }

    #[test]
    fn test_fraction_addition() {
        let f1 = Fraction::new(1, 2);
        let f2 = Fraction::new(1, 3);
        let result = f1 + f2;
        assert_eq!(result, Fraction::new(5, 6));
    }

    #[test]
    fn test_fraction_multiplication() {
        let f1 = Fraction::new(2, 3);
        let f2 = Fraction::new(3, 4);
        let result = f1 * f2;
        assert_eq!(result, Fraction::new(1, 2));
    }

    #[test]
    fn test_fraction_comparison() {
        let f1 = Fraction::new(1, 2);
        let f2 = Fraction::new(2, 3);
        assert!(f1 < f2);
    }
}
