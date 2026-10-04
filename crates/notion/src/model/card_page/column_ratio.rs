use std::fmt;

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

const LEGACY_COLUMN_RATIO_BASIS_POINTS_PER_WHOLE: u64 = 10_000;

#[derive(Clone, Copy)]
pub struct CardPageColumnRatio(u64);

impl CardPageColumnRatio {
    pub(crate) fn from_fraction(fraction: f64) -> Result<Self, String> {
        if !fraction.is_finite() || fraction <= 0.0 {
            return Err(format!(
                "Notion column ratio must be positive and finite, received {fraction}"
            ));
        }
        Ok(Self(fraction.to_bits()))
    }

    pub fn fraction(self) -> f64 {
        f64::from_bits(self.0)
    }

    fn from_legacy_basis_points(basis_points: u64) -> Result<Self, String> {
        if !(1..=LEGACY_COLUMN_RATIO_BASIS_POINTS_PER_WHOLE).contains(&basis_points) {
            return Err(format!(
                "legacy Notion column ratio must contain 1..={LEGACY_COLUMN_RATIO_BASIS_POINTS_PER_WHOLE} basis points, received {basis_points}"
            ));
        }
        Self::from_fraction(basis_points as f64 / LEGACY_COLUMN_RATIO_BASIS_POINTS_PER_WHOLE as f64)
    }
}

impl PartialEq for CardPageColumnRatio {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for CardPageColumnRatio {}

impl fmt::Debug for CardPageColumnRatio {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("CardPageColumnRatio")
            .field(&self.fraction())
            .finish()
    }
}

impl Serialize for CardPageColumnRatio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.fraction())
    }
}

impl<'de> Deserialize<'de> for CardPageColumnRatio {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(CardPageColumnRatioVisitor)
    }
}

struct CardPageColumnRatioVisitor;

impl<'de> Visitor<'de> for CardPageColumnRatioVisitor {
    type Value = CardPageColumnRatio;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a positive finite ratio or legacy integer basis-point ratio")
    }

    fn visit_f64<E>(self, fraction: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        CardPageColumnRatio::from_fraction(fraction).map_err(E::custom)
    }

    fn visit_u64<E>(self, basis_points: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        CardPageColumnRatio::from_legacy_basis_points(basis_points).map_err(E::custom)
    }

    fn visit_i64<E>(self, basis_points: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let basis_points = u64::try_from(basis_points).map_err(E::custom)?;
        self.visit_u64(basis_points)
    }
}
