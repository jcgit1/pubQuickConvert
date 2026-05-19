use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::error::{ConvertError, ConvertResult};

/// Maximum magnitude accepted for user input (rejects values like 1e1000).
fn max_input() -> Decimal {
    dec!(9999999999999999)
}

fn min_input() -> Decimal {
    dec!(-9999999999999999)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitSystem {
    Imperial,
    Metric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitCategory {
    Length,
    Weight,
    Temperature,
    Volume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    // Length
    Feet,
    Inches,
    Miles,
    Meters,
    Centimeters,
    Kilometers,
    // Weight
    Pounds,
    Ounces,
    Kilograms,
    Grams,
    // Temperature
    Fahrenheit,
    Celsius,
    Kelvin,
    // Volume
    Gallons,
    Quarts,
    FluidOunces,
    Liters,
    Milliliters,
    CubicMeters,
}

impl UnitCategory {
    pub const ALL: [Self; 4] = [
        Self::Length,
        Self::Weight,
        Self::Temperature,
        Self::Volume,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Weight => "Weight",
            Self::Temperature => "Temperature",
            Self::Volume => "Volume",
        }
    }
}

impl Unit {
    pub fn category(self) -> UnitCategory {
        match self {
            Self::Feet | Self::Inches | Self::Miles | Self::Meters | Self::Centimeters
            | Self::Kilometers => UnitCategory::Length,
            Self::Pounds | Self::Ounces | Self::Kilograms | Self::Grams => {
                UnitCategory::Weight
            }
            Self::Fahrenheit | Self::Celsius | Self::Kelvin => UnitCategory::Temperature,
            Self::Gallons | Self::Quarts | Self::FluidOunces | Self::Liters
            | Self::Milliliters | Self::CubicMeters => UnitCategory::Volume,
        }
    }

    pub fn system(self) -> UnitSystem {
        match self {
            Self::Feet | Self::Inches | Self::Miles | Self::Pounds | Self::Ounces
            | Self::Gallons | Self::Quarts | Self::FluidOunces | Self::Fahrenheit => {
                UnitSystem::Imperial
            }
            Self::Meters | Self::Centimeters | Self::Kilometers | Self::Kilograms | Self::Grams
            | Self::Liters | Self::Milliliters | Self::CubicMeters | Self::Celsius | Self::Kelvin => {
                UnitSystem::Metric
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Feet => "Feet",
            Self::Inches => "Inches",
            Self::Miles => "Miles",
            Self::Meters => "Meters",
            Self::Centimeters => "Centimeters",
            Self::Kilometers => "Kilometers",
            Self::Pounds => "Pounds (lbs)",
            Self::Ounces => "Ounces",
            Self::Kilograms => "Kilograms",
            Self::Grams => "Grams",
            Self::Fahrenheit => "°F",
            Self::Celsius => "°C",
            Self::Kelvin => "Kelvin",
            Self::Gallons => "Gallons (US)",
            Self::Quarts => "Quarts (US)",
            Self::FluidOunces => "Fluid Ounces (US)",
            Self::Liters => "Liters",
            Self::Milliliters => "Milliliters",
            Self::CubicMeters => "Cubic Meters",
        }
    }

    pub fn units_for_category(category: UnitCategory) -> &'static [Unit] {
        match category {
            UnitCategory::Length => &[
                Unit::Feet,
                Unit::Inches,
                Unit::Miles,
                Unit::Meters,
                Unit::Centimeters,
                Unit::Kilometers,
            ],
            UnitCategory::Weight => &[
                Unit::Pounds,
                Unit::Ounces,
                Unit::Kilograms,
                Unit::Grams,
            ],
            UnitCategory::Temperature => &[Unit::Fahrenheit, Unit::Celsius, Unit::Kelvin],
            UnitCategory::Volume => &[
                Unit::Gallons,
                Unit::Quarts,
                Unit::FluidOunces,
                Unit::Liters,
                Unit::Milliliters,
                Unit::CubicMeters,
            ],
        }
    }

    pub fn default_for_system(category: UnitCategory, system: UnitSystem) -> Unit {
        let units = Self::units_for_category(category);
        units
            .iter()
            .copied()
            .find(|u| u.system() == system)
            .unwrap_or(units[0])
    }

    pub fn paired_unit(unit: Unit) -> Unit {
        match unit {
            Unit::Feet => Unit::Meters,
            Unit::Meters => Unit::Feet,
            Unit::Inches => Unit::Centimeters,
            Unit::Centimeters => Unit::Inches,
            Unit::Miles => Unit::Kilometers,
            Unit::Kilometers => Unit::Miles,
            Unit::Pounds => Unit::Kilograms,
            Unit::Kilograms => Unit::Pounds,
            Unit::Ounces => Unit::Grams,
            Unit::Grams => Unit::Ounces,
            Unit::Fahrenheit => Unit::Celsius,
            Unit::Celsius => Unit::Fahrenheit,
            Unit::Kelvin => Unit::Celsius,
            Unit::Gallons => Unit::Liters,
            Unit::Liters => Unit::Gallons,
            Unit::Quarts => Unit::Liters,
            Unit::FluidOunces => Unit::Milliliters,
            Unit::Milliliters => Unit::FluidOunces,
            Unit::CubicMeters => Unit::Gallons,
        }
    }
}

/// Parse and validate user input.
pub fn parse_input(raw: &str) -> ConvertResult<Decimal> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ConvertError::EmptyInput);
    }

    if trimmed.contains('e') || trimmed.contains('E') {
        return Err(ConvertError::InvalidNumber);
    }

    let value = Decimal::from_str(trimmed).map_err(|_| ConvertError::InvalidNumber)?;

    if value.is_sign_negative() && value.is_zero() {
        return Err(ConvertError::InvalidNumber);
    }

    let max = max_input();
    let min = min_input();

    if value > max {
        return Err(ConvertError::InputTooLarge);
    }
    if value < min {
        return Err(ConvertError::InputTooSmall);
    }

    Ok(value)
}

/// Convert `value` from `from` to `to`, rounding to `decimal_places` with HalfUp.
pub fn convert(
    value: Decimal,
    from: Unit,
    to: Unit,
    decimal_places: u32,
) -> ConvertResult<Decimal> {
    if from.category() != to.category() {
        return Err(ConvertError::IncompatibleUnits);
    }

    let places = decimal_places.clamp(2, 6);
    let result = match from.category() {
        UnitCategory::Temperature => convert_temperature(value, from, to)?,
        _ => {
            let base = to_base(value, from)?;
            from_base(base, to)?
        }
    };

    let rounded = result.round_dp_with_strategy(places, RoundingStrategy::MidpointAwayFromZero);

    if rounded > max_input() {
        return Err(ConvertError::Overflow);
    }
    if rounded < min_input() {
        return Err(ConvertError::Underflow);
    }

    Ok(rounded)
}

fn to_base(value: Decimal, unit: Unit) -> ConvertResult<Decimal> {
    let factor = factor_to_base(unit)?;
    value
        .checked_mul(factor)
        .ok_or(ConvertError::Overflow)
}

fn from_base(value: Decimal, unit: Unit) -> ConvertResult<Decimal> {
    let factor = factor_to_base(unit)?;
    if factor.is_zero() {
        return Err(ConvertError::DivisionByZero);
    }
    value
        .checked_div(factor)
        .ok_or(ConvertError::Underflow)
}

fn factor_to_base(unit: Unit) -> ConvertResult<Decimal> {
    let f = match unit {
        // Length → meters
        Unit::Meters => Decimal::ONE,
        Unit::Centimeters => dec!(0.01),
        Unit::Kilometers => dec!(1000),
        Unit::Feet => dec!(0.3048),
        Unit::Inches => dec!(0.0254),
        Unit::Miles => dec!(1609.344),
        // Weight → kilograms
        Unit::Kilograms => Decimal::ONE,
        Unit::Grams => dec!(0.001),
        Unit::Pounds => dec!(0.45359237),
        Unit::Ounces => dec!(0.028349523125),
        // Volume → liters
        Unit::Liters => Decimal::ONE,
        Unit::Milliliters => dec!(0.001),
        Unit::CubicMeters => dec!(1000),
        Unit::Gallons => dec!(3.785411784),
        Unit::Quarts => dec!(0.946352946),
        Unit::FluidOunces => dec!(0.0295735295625),
        Unit::Fahrenheit | Unit::Celsius | Unit::Kelvin => {
            return Err(ConvertError::IncompatibleUnits);
        }
    };
    Ok(f)
}

fn convert_temperature(value: Decimal, from: Unit, to: Unit) -> ConvertResult<Decimal> {
    let celsius = match from {
        Unit::Celsius => value,
        Unit::Fahrenheit => {
            (value - dec!(32))
                .checked_mul(dec!(5))
                .and_then(|v| v.checked_div(dec!(9)))
                .ok_or(ConvertError::Overflow)?
        }
        Unit::Kelvin => value
            .checked_sub(dec!(273.15))
            .ok_or(ConvertError::Underflow)?,
        _ => return Err(ConvertError::IncompatibleUnits),
    };

    match to {
        Unit::Celsius => Ok(celsius),
        Unit::Fahrenheit => celsius
            .checked_mul(dec!(9))
            .and_then(|v| v.checked_div(dec!(5)))
            .and_then(|v| v.checked_add(dec!(32)))
            .ok_or(ConvertError::Overflow),
        Unit::Kelvin => celsius
            .checked_add(dec!(273.15))
            .ok_or(ConvertError::Overflow),
        _ => Err(ConvertError::IncompatibleUnits),
    }
}

pub fn format_decimal(value: Decimal, decimal_places: u32) -> String {
    let places = decimal_places.clamp(2, 6);
    value.round_dp_with_strategy(places, RoundingStrategy::MidpointAwayFromZero).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meters_to_feet() {
        let v = parse_input("10.99").unwrap();
        let out = convert(v, Unit::Meters, Unit::Feet, 2).unwrap();
        assert_eq!(out, dec!(36.06));
    }

    #[test]
    fn rejects_scientific() {
        assert_eq!(parse_input("1e1000"), Err(ConvertError::InvalidNumber));
    }
}
