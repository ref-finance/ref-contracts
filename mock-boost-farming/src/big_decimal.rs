/*!
* from https://github.com/NearDeFi/burrowland/blob/main/contract/src/big_decimal.rs
*/
use crate::*;
use near_sdk::borsh::maybestd::io::Write;
use near_sdk::json_types::U128;
use near_sdk::serde::Serializer;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, Mul, Sub};
#[cfg(not(target_arch = "wasm32"))]
use std::str::FromStr;

uint::construct_uint!(
    pub struct U256(4);
);

uint::construct_uint!(
    pub struct U384(6);
);

pub const MS_PER_YEAR: u64 = 31536000000;
pub(crate) const MAX_RATIO: u32 = 10000;

const NUM_DECIMALS: u8 = 27;
const BIG_DIVISOR: u128 = 10u128.pow(NUM_DECIMALS as u32);
const HALF_DIVISOR: u128 = BIG_DIVISOR / 2;

pub type LowU128 = U128;

#[derive(Copy, Clone)]
pub struct BigDecimal(U384);

impl Default for BigDecimal {
    fn default() -> Self {
        BigDecimal::zero()
    }
}

impl Display for BigDecimal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let a = self.0 / U384::from(BIG_DIVISOR);
        let b = (self.0 - a * U384::from(BIG_DIVISOR)).as_u128();
        if b > 0 {
            write!(f, "{}", format!("{}.{:027}", a, b).trim_end_matches('0'))
        } else {
            write!(f, "{}.0", a)
        }
    }
}

impl std::fmt::Debug for BigDecimal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[cfg(not(target_arch = "wasm32"))]
const PARSE_INT_ERROR: &'static str = "Parse int error";

#[cfg(not(target_arch = "wasm32"))]
impl FromStr for BigDecimal {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dot_pos = s.find('.');
        let (int, dec) = if let Some(dot_pos) = dot_pos {
            (
                &s[..dot_pos],
                format!("{:0<27}", &s[dot_pos + 1..])
                    .parse()
                    .map_err(|_| PARSE_INT_ERROR)?,
            )
        } else {
            (s, 0u128)
        };
        let int = U384::from_str(&int).map_err(|_| PARSE_INT_ERROR)?;
        if dec >= BIG_DIVISOR {
            return Err(String::from("The decimal part is too large"));
        }
        Ok(Self(int * U384::from(BIG_DIVISOR) + U384::from(dec)))
    }
}

impl Serialize for BigDecimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<'de> Deserialize<'de> for BigDecimal {
    fn deserialize<D>(
        deserializer: D,
    ) -> Result<Self, <D as near_sdk::serde::Deserializer<'de>>::Error>
    where
        D: near_sdk::serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(Self::from_str(&s).map_err(|err| near_sdk::serde::de::Error::custom(err))?)
    }
}

impl From<u128> for BigDecimal {
    fn from(a: u128) -> Self {
        Self(U384::from(a) * U384::from(BIG_DIVISOR))
    }
}

impl From<u64> for BigDecimal {
    fn from(a: u64) -> Self {
        Self(U384::from(a) * U384::from(BIG_DIVISOR))
    }
}

impl From<u32> for BigDecimal {
    fn from(a: u32) -> Self {
        Self(U384::from(a) * U384::from(BIG_DIVISOR))
    }
}

impl From<f64> for BigDecimal {
    fn from(a: f64) -> Self {
        let base = a as u128;
        Self(
            U384::from(base) * U384::from(BIG_DIVISOR)
                + U384::from((a.fract() * (BIG_DIVISOR as f64)) as u128),
        )
    }
}

impl Add for BigDecimal {
    type Output = Self;

    fn add(self, rhs: BigDecimal) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for BigDecimal {
    type Output = Self;

    fn sub(self, rhs: BigDecimal) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul for BigDecimal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self((self.0 * rhs.0 + U384::from(HALF_DIVISOR)) / U384::from(BIG_DIVISOR))
    }
}

impl Div for BigDecimal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self((self.0 * U384::from(BIG_DIVISOR) + U384::from(HALF_DIVISOR)) / rhs.0)
    }
}

impl From<LowU128> for BigDecimal {
    fn from(low_u128: LowU128) -> Self {
        Self(U384::from(low_u128.0))
    }
}

impl From<BigDecimal> for LowU128 {
    fn from(bd: BigDecimal) -> Self {
        Self(bd.0.low_u128())
    }
}

impl BigDecimal {
    pub fn from_ratio(ratio: u32) -> Self {
        Self(U384::from(ratio) * U384::from(BIG_DIVISOR / (MAX_RATIO as u128)))
    }

    pub fn mul_ratio(&self, ratio: u32) -> Self {
        Self((self.0 * U384::from(ratio) + U384::from(MAX_RATIO / 2)) / U384::from(MAX_RATIO))
    }

    pub fn div_ratio(&self, ratio: u32) -> Self {
        Self((self.0 * U384::from(MAX_RATIO) + U384::from(MAX_RATIO / 2)) / U384::from(ratio))
    }

    // pub fn from_balance_price(balance: Balance, price: &Price, extra_decimals: u8) -> Self {
    //     let num = U384::from(price.multiplier) * U384::from(balance);
    //     let denominator_decimals = price.decimals + extra_decimals;
    //     if denominator_decimals > NUM_DECIMALS {
    //         Self(num / U384::exp10((denominator_decimals - NUM_DECIMALS) as usize))
    //     } else {
    //         Self(num * U384::exp10((NUM_DECIMALS - denominator_decimals) as usize))
    //     }
    // }

    pub fn round_u128(&self) -> u128 {
        ((self.0 + U384::from(HALF_DIVISOR)) / U384::from(BIG_DIVISOR)).as_u128()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn f64(&self) -> f64 {
        let base = (self.0 / U384::from(BIG_DIVISOR)).as_u128();
        let fract = (self.0 - U384::from(base)).as_u128() as f64;
        base as f64 + fract / (BIG_DIVISOR as f64)
    }

    pub fn round_mul_u128(&self, rhs: u128) -> u128 {
        ((self.0 * U384::from(rhs) + U384::from(HALF_DIVISOR)) / U384::from(BIG_DIVISOR)).as_u128()
    }

    pub fn round_down_mul_u128(&self, rhs: u128) -> u128 {
        (self.0 * U384::from(rhs) / U384::from(BIG_DIVISOR)).as_u128()
    }

    pub fn div_u128(&self, rhs: u128) -> BigDecimal {
        Self(self.0 / U384::from(rhs))
    }

    pub fn zero() -> Self {
        Self(U384::zero())
    }

    pub fn one() -> Self {
        Self(U384::from(BIG_DIVISOR))
    }

    pub fn pow(&self, mut exponent: u64) -> Self {
        let mut res = BigDecimal::one();
        let mut x = *self;

        while exponent != 0 {
            if (exponent & 1) != 0 {
                res = res * x;
            }
            exponent >>= 1;
            if exponent != 0 {
                x = x * x;
            }
        }

        res
    }
}

impl PartialEq<Self> for BigDecimal {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for BigDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl BorshSerialize for BigDecimal {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&self.0 .0, writer)
    }
}

impl BorshDeserialize for BigDecimal {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        Ok(Self(U384(BorshDeserialize::deserialize(buf)?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    // Number of milliseconds in a regular year.
    const N: u64 = MS_PER_YEAR;
    // X = 2
    const LOW_X: LowU128 = U128(2000000000000000000000000000);
    // R ** N = X. So R = X ** (1/N)
    const LOW_R: LowU128 = U128(1000000000021979552909930328);

    fn b(a: u128) -> BigDecimal {
        BigDecimal::from(a)
    }

    fn almost_eq(a: u128, b: u128, prec: u32) {
        let p = 10u128.pow(27 - prec);
        let ap = (a + p / 2) / p;
        let bp = (b + p / 2) / p;
        assert_eq!(
            ap,
            bp,
            "{}",
            format!("Expected {} to eq {}, with precision {}", a, b, prec)
        );
    }

    #[test]
    fn test_simple_add() {
        assert_eq!((b(0) + b(0)).round_u128(), 0);
        assert_eq!((b(5) + b(2)).round_u128(), 7);
        assert_eq!((b(2) + b(5)).round_u128(), 7);
        assert_eq!((b(5) + b(0)).round_u128(), 5);
        assert_eq!((b(0) + b(5)).round_u128(), 5);
    }

    #[test]
    fn test_simple_div() {
        assert_eq!((b(17) / b(5)).round_u128(), 3);
        assert_eq!((b(18) / b(5)).round_u128(), 4);
        assert_eq!((b(3) / b(5)).round_u128(), 1);
    }

    #[test]
    fn test_pow() {
        let r = BigDecimal::from(LOW_R);
        let x = r.pow(N);
        let low_x = LowU128::from(x);
        almost_eq(LOW_X.0, low_x.0, 15);
    }

    #[test]
    fn test_compound_pow() {
        fn test(split_n: u64) {
            let r = BigDecimal::from(LOW_R);
            let initial_val = 12345 * 10u128.pow(24);
            let mut val = initial_val;
            for i in 1..=split_n {
                let exponent = (N * i / split_n) - (N * (i - 1) / split_n);
                let interest = r.pow(exponent);
                val = interest.round_mul_u128(val);
            }
            almost_eq(val, initial_val * 2, 15);
        }

        (1..=100).for_each(test);
    }

    #[test]
    fn test_compound_pow_precision() {
        fn test(split_n: u64) {
            let r = BigDecimal::from(LOW_R);
            let initial_val = 12345 * 10u128.pow(24);
            let mut val = initial_val;
            let exponent = N / split_n;
            assert_eq!(exponent * split_n, N);
            let interest = r.pow(exponent);
            for _ in 1..=split_n {
                val = interest.round_mul_u128(val);
            }
            almost_eq(val, initial_val * 2, 15);
        }
        test(N / 60000);
        test(N / 1000000);
        test(N / (24 * 60 * 60));
    }

    #[test]
    fn test_compound_pow_random() {
        const MAX_STEP: u64 = 1000000;
        let r = BigDecimal::from(LOW_R);
        let initial_val = 12345 * 10u128.pow(24);
        let mut val = initial_val;
        let mut total_exponent = 0;
        let mut rng = rand::thread_rng();
        while total_exponent < N {
            let exponent = std::cmp::min(N - total_exponent, rng.next_u64() % MAX_STEP + 1);
            total_exponent += exponent;
            let interest = r.pow(exponent);
            val = interest.round_mul_u128(val);
        }
        almost_eq(val, initial_val * 2, 15);
    }

    #[test]
    fn test_display() {
        assert_eq!("1.0", BigDecimal::one().to_string());
        assert_eq!("2.0", BigDecimal::from(2u32).to_string());
        assert_eq!("0.0", BigDecimal::zero().to_string());
        assert!(BigDecimal::from(1.5f64).to_string().starts_with("1.500000"));
        assert!(BigDecimal::from(0.5f64).to_string().starts_with("0.500000"));
    }
}
