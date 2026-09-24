//! Número de punto fijo no negativo en base 10^4.
//!
//! `limbs[0]` es la parte entera (menor que [`BASE`]) y `limbs[1..]` las fraccionarias, de mayor
//! a menor peso: el valor es `Σ limbs[i] · BASE^(−i)`. Todas las operaciones son totales: los
//! operandos de distinta longitud se completan con ceros y las restas devuelven la diferencia
//! absoluta junto con el orden.

use std::cmp::Ordering;

/// Base de los limbs: cada limb guarda 4 dígitos decimales.
pub const BASE: u32 = 10_000;
/// Dígitos decimales por limb.
pub const LIMB_DIGITS: usize = 4;

/// Número de punto fijo no negativo en base 10^4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fixed {
    limbs: Vec<u32>,
}

impl Fixed {
    /// Construye a partir de limbs ya normalizados (cada uno menor que [`BASE`]). Los limbs
    /// fuera de rango se reducen módulo [`BASE`] para mantener el invariante.
    pub fn from_limbs(mut limbs: Vec<u32>) -> Self {
        if limbs.is_empty() {
            limbs.push(0);
        }
        for limb in &mut limbs {
            *limb %= BASE;
        }
        Self { limbs }
    }

    /// Cero con `len` limbs.
    pub fn zero(len: usize) -> Self {
        Self {
            limbs: vec![0; len.max(1)],
        }
    }

    /// Entero pequeño (`value < BASE`) con `len` limbs.
    pub fn from_u32(value: u32, len: usize) -> Self {
        let mut limbs = vec![0; len.max(1)];
        limbs[0] = value % BASE;
        Self { limbs }
    }

    /// Aproximación desde `f64` (`0 ≤ value < BASE`), exacta en los 3 primeros limbs
    /// fraccionarios (12 decimales); el resto queda a cero.
    pub fn from_f64(value: f64, len: usize) -> Self {
        let value = value.clamp(0.0, f64::from(BASE - 1));
        let mut limbs = vec![0; len.max(1)];
        let int = value.floor();
        limbs[0] = int as u32;
        let mut frac = value - int;
        for limb in limbs.iter_mut().skip(1).take(3) {
            frac *= f64::from(BASE);
            let digit = frac.floor();
            *limb = (digit as u32).min(BASE - 1);
            frac -= digit;
        }
        Self { limbs }
    }

    /// Limbs de mayor a menor peso.
    pub fn limbs(&self) -> &[u32] {
        &self.limbs
    }

    /// Número de limbs (1 entero + fraccionarios).
    pub fn len(&self) -> usize {
        self.limbs.len()
    }

    /// Nunca: siempre hay al menos el limb entero.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// `true` si todos los limbs son cero.
    pub fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&l| l == 0)
    }

    /// Copia con exactamente `len` limbs: trunca o completa con ceros.
    pub fn resized(&self, len: usize) -> Self {
        let len = len.max(1);
        let mut limbs = self.limbs.clone();
        limbs.resize(len, 0);
        Self { limbs }
    }

    /// Valor aproximado en `f64` a partir de los 4 primeros limbs.
    pub fn to_f64(&self) -> f64 {
        let mut value = 0.0;
        let mut weight = 1.0;
        for &limb in self.limbs.iter().take(4) {
            value += f64::from(limb) * weight;
            weight /= f64::from(BASE);
        }
        value
    }

    /// Parte entera.
    pub fn integer_part(&self) -> u32 {
        self.limbs[0]
    }

    /// Número de limbs a cero desde el principio (incluido el entero).
    pub fn leading_zero_limbs(&self) -> usize {
        self.limbs.iter().take_while(|&&l| l == 0).count()
    }

    /// Los `digits` primeros decimales, como texto ASCII. Si el número tiene menos, se completa
    /// con ceros.
    pub fn fraction_digits(&self, digits: usize) -> String {
        let mut out = String::with_capacity(digits);
        for &limb in self.limbs.iter().skip(1) {
            if out.len() >= digits {
                break;
            }
            let text = format!("{limb:04}");
            let take = (digits - out.len()).min(LIMB_DIGITS);
            out.push_str(&text[..take]);
        }
        while out.len() < digits {
            out.push('0');
        }
        out
    }

    /// Suma. El resultado tiene la longitud del operando más largo; un desbordamiento de la
    /// parte entera se pierde módulo [`BASE`] (no ocurre con valores menores que 5000).
    pub fn add(&self, other: &Self) -> Self {
        let len = self.len().max(other.len());
        let mut limbs = vec![0; len];
        let mut carry = 0;
        for i in (0..len).rev() {
            let sum = self.limb_at(i) + other.limb_at(i) + carry;
            limbs[i] = sum % BASE;
            carry = sum / BASE;
        }
        Self { limbs }
    }

    /// Diferencia absoluta y orden de `self` respecto a `other`.
    pub fn abs_diff(&self, other: &Self) -> (Self, Ordering) {
        match self.cmp_value(other) {
            Ordering::Less => (other.sub_smaller(self), Ordering::Less),
            Ordering::Equal => (Self::zero(self.len().max(other.len())), Ordering::Equal),
            Ordering::Greater => (self.sub_smaller(other), Ordering::Greater),
        }
    }

    /// Compara valores numéricos (independiente de la longitud).
    pub fn cmp_value(&self, other: &Self) -> Ordering {
        let len = self.len().max(other.len());
        for i in 0..len {
            match self.limb_at(i).cmp(&other.limb_at(i)) {
                Ordering::Equal => {}
                ord => return ord,
            }
        }
        Ordering::Equal
    }

    /// Mitad (división entera por 2 en la última posición; el residuo se trunca).
    pub fn half(&self) -> Self {
        self.div_small(2)
    }

    /// División por un entero pequeño, truncando.
    pub fn div_small(&self, divisor: u32) -> Self {
        let divisor = u64::from(divisor.max(1));
        let mut limbs = vec![0; self.len()];
        let mut rem = 0_u64;
        for (i, &limb) in self.limbs.iter().enumerate() {
            let cur = rem * u64::from(BASE) + u64::from(limb);
            limbs[i] = (cur / divisor) as u32;
            rem = cur % divisor;
        }
        Self { limbs }
    }

    /// Producto por un entero. La parte entera del resultado debe caber en un limb; si no, se
    /// pierde módulo [`BASE`].
    pub fn mul_small(&self, factor: u32) -> Self {
        let factor = u64::from(factor);
        let mut limbs = vec![0; self.len()];
        let mut carry = 0_u64;
        for i in (0..self.len()).rev() {
            let cur = u64::from(self.limbs[i]) * factor + carry;
            limbs[i] = (cur % u64::from(BASE)) as u32;
            carry = cur / u64::from(BASE);
        }
        Self { limbs }
    }

    #[inline]
    fn limb_at(&self, i: usize) -> u32 {
        self.limbs.get(i).copied().unwrap_or(0)
    }

    /// `self − other` sabiendo que `self ≥ other`.
    fn sub_smaller(&self, other: &Self) -> Self {
        let len = self.len().max(other.len());
        let mut limbs = vec![0; len];
        let mut borrow = 0;
        for i in (0..len).rev() {
            let a = self.limb_at(i);
            let b = other.limb_at(i) + borrow;
            if a >= b {
                limbs[i] = a - b;
                borrow = 0;
            } else {
                limbs[i] = a + BASE - b;
                borrow = 1;
            }
        }
        Self { limbs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(limbs: &[u32]) -> Fixed {
        Fixed::from_limbs(limbs.to_vec())
    }

    #[test]
    fn add_carries() {
        let a = fx(&[1, 9999, 9999]);
        let b = fx(&[0, 0, 1]);
        assert_eq!(a.add(&b), fx(&[2, 0, 0]));
    }

    #[test]
    fn abs_diff_both_directions() {
        let a = fx(&[1, 0, 0]);
        let b = fx(&[0, 9999, 9999]);
        let (d, ord) = a.abs_diff(&b);
        assert_eq!(ord, Ordering::Greater);
        assert_eq!(d, fx(&[0, 0, 1]));
        let (d, ord) = b.abs_diff(&a);
        assert_eq!(ord, Ordering::Less);
        assert_eq!(d, fx(&[0, 0, 1]));
        assert_eq!(a.abs_diff(&a).1, Ordering::Equal);
    }

    #[test]
    fn half_and_mul_small() {
        let one = Fixed::from_u32(1, 3);
        assert_eq!(one.half(), fx(&[0, 5000, 0]));
        assert_eq!(one.half().half(), fx(&[0, 2500, 0]));
        assert_eq!(fx(&[0, 2500, 0]).mul_small(4), one);
        assert_eq!(fx(&[0, 3333, 3333]).mul_small(3), fx(&[0, 9999, 9999]));
    }

    #[test]
    fn f64_round_trip() {
        let x = Fixed::from_f64(std::f64::consts::PI, 5);
        assert_eq!(x.limbs(), &[3, 1415, 9265, 3589, 0]);
        assert!((x.to_f64() - std::f64::consts::PI).abs() < 1e-12);
        assert_eq!(x.fraction_digits(10), "1415926535");
        assert_eq!(x.fraction_digits(14), "14159265358900");
    }

    #[test]
    fn different_lengths_are_zero_extended() {
        let a = fx(&[1, 5000]);
        let b = fx(&[0, 0, 5000]);
        assert_eq!(a.add(&b), fx(&[1, 5000, 5000]));
        assert_eq!(a.cmp_value(&fx(&[1, 5000, 0, 0])), Ordering::Equal);
        assert_eq!(a.resized(4).len(), 4);
        assert_eq!(fx(&[0, 0, 7, 1]).leading_zero_limbs(), 2);
    }
}
