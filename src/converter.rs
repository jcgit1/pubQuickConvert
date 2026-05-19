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
    Speed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    // Length
    Feet,
    Inches,
    Miles,
    Meters,
    Centimeters,
    Millimeters,
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
    // Speed
    MilesPerHour,
    KilometersPerHour,
}

impl UnitCategory {
    pub const ALL: [Self; 5] = [
        Self::Length,
        Self::Weight,
        Self::Temperature,
        Self::Volume,
        Self::Speed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Weight => "Weight",
            Self::Temperature => "Temperature",
            Self::Volume => "Volume",
            Self::Speed => "Speed",
        }
    }
}

impl Unit {
    pub fn category(self) -> UnitCategory {
        match self {
            Self::Feet | Self::Inches | Self::Miles | Self::Meters | Self::Centimeters
            | Self::Millimeters | Self::Kilometers => UnitCategory::Length,
            Self::Pounds | Self::Ounces | Self::Kilograms | Self::Grams => {
                UnitCategory::Weight
            }
            Self::Fahrenheit | Self::Celsius | Self::Kelvin => UnitCategory::Temperature,
            Self::Gallons | Self::Quarts | Self::FluidOunces | Self::Liters
            | Self::Milliliters | Self::CubicMeters => UnitCategory::Volume,
            Self::MilesPerHour | Self::KilometersPerHour => UnitCategory::Speed,
        }
    }

    pub fn system(self) -> UnitSystem {
        match self {
            Self::Feet | Self::Inches | Self::Miles | Self::Pounds | Self::Ounces
            | Self::Gallons | Self::Quarts | Self::FluidOunces | Self::Fahrenheit
            | Self::MilesPerHour => UnitSystem::Imperial,
            Self::Meters | Self::Centimeters | Self::Millimeters | Self::Kilometers
            | Self::Kilograms | Self::Grams | Self::Liters | Self::Milliliters
            | Self::CubicMeters | Self::Celsius | Self::Kelvin | Self::KilometersPerHour => {
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
            Self::Millimeters => "Millimeters",
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
            Self::MilesPerHour => "Miles per hour (mph)",
            Self::KilometersPerHour => "Kilometers per hour (km/h)",
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
                Unit::Millimeters,
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
            UnitCategory::Speed => &[Unit::MilesPerHour, Unit::KilometersPerHour],
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
            Unit::Millimeters => Unit::Inches,
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
            Unit::MilesPerHour => Unit::KilometersPerHour,
            Unit::KilometersPerHour => Unit::MilesPerHour,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInput {
    pub value: Decimal,
    /// Unit suffix from the input field, if the user typed one (e.g. `mm`, `in`).
    pub unit: Option<Unit>,
}

/// Optional unit aliases (longest match first). Case-insensitive.
const UNIT_ALIASES: &[(&str, Unit)] = &[
    ("miles per hour", Unit::MilesPerHour),
    ("kilometers per hour", Unit::KilometersPerHour),
    ("kilometres per hour", Unit::KilometersPerHour),
    ("km/h", Unit::KilometersPerHour),
    ("kmph", Unit::KilometersPerHour),
    ("kph", Unit::KilometersPerHour),
    ("mph", Unit::MilesPerHour),
    ("mi/h", Unit::MilesPerHour),
    ("fluid ounce", Unit::FluidOunces),
    ("fluid ounces", Unit::FluidOunces),
    ("fl oz", Unit::FluidOunces),
    ("floz", Unit::FluidOunces),
    ("cubic meter", Unit::CubicMeters),
    ("cubic meters", Unit::CubicMeters),
    ("cubic metre", Unit::CubicMeters),
    ("millimeter", Unit::Millimeters),
    ("millimeters", Unit::Millimeters),
    ("millilitre", Unit::Milliliters),
    ("millilitres", Unit::Milliliters),
    ("milliliter", Unit::Milliliters),
    ("milliliters", Unit::Milliliters),
    ("centimeter", Unit::Centimeters),
    ("centimeters", Unit::Centimeters),
    ("centimetre", Unit::Centimeters),
    ("centimetres", Unit::Centimeters),
    ("kilometer", Unit::Kilometers),
    ("kilometers", Unit::Kilometers),
    ("kilometre", Unit::Kilometers),
    ("kilometres", Unit::Kilometers),
    ("fahrenheit", Unit::Fahrenheit),
    ("celsius", Unit::Celsius),
    ("gallon", Unit::Gallons),
    ("gallons", Unit::Gallons),
    ("quart", Unit::Quarts),
    ("quarts", Unit::Quarts),
    ("pound", Unit::Pounds),
    ("pounds", Unit::Pounds),
    ("ounce", Unit::Ounces),
    ("ounces", Unit::Ounces),
    ("liter", Unit::Liters),
    ("liters", Unit::Liters),
    ("litre", Unit::Liters),
    ("litres", Unit::Liters),
    ("meter", Unit::Meters),
    ("meters", Unit::Meters),
    ("metre", Unit::Meters),
    ("metres", Unit::Meters),
    ("kelvin", Unit::Kelvin),
    ("gram", Unit::Grams),
    ("grams", Unit::Grams),
    ("inch", Unit::Inches),
    ("inches", Unit::Inches),
    ("foot", Unit::Feet),
    ("feet", Unit::Feet),
    ("mile", Unit::Miles),
    ("miles", Unit::Miles),
    ("mm", Unit::Millimeters),
    ("cm", Unit::Centimeters),
    ("km", Unit::Kilometers),
    ("ml", Unit::Milliliters),
    ("oz", Unit::Ounces),
    ("lb", Unit::Pounds),
    ("lbs", Unit::Pounds),
    ("kg", Unit::Kilograms),
    ("ft", Unit::Feet),
    ("mi", Unit::Miles),
    ("gal", Unit::Gallons),
    ("qt", Unit::Quarts),
    ("°f", Unit::Fahrenheit),
    ("°c", Unit::Celsius),
    ("f", Unit::Fahrenheit),
    ("c", Unit::Celsius),
    ("k", Unit::Kelvin),
    ("m", Unit::Meters),
    ("g", Unit::Grams),
    ("l", Unit::Liters),
    ("in", Unit::Inches),
    ("m3", Unit::CubicMeters),
];

impl Unit {
    pub fn from_alias(token: &str) -> Option<Self> {
        let key = token.trim().to_ascii_lowercase();
        UNIT_ALIASES
            .iter()
            .find(|(alias, _)| *alias == key)
            .map(|(_, unit)| *unit)
    }

    pub fn short_suffix(self) -> &'static str {
        match self {
            Self::Feet => "ft",
            Self::Inches => "in",
            Self::Miles => "mi",
            Self::Meters => "m",
            Self::Centimeters => "cm",
            Self::Millimeters => "mm",
            Self::Kilometers => "km",
            Self::Pounds => "lb",
            Self::Ounces => "oz",
            Self::Kilograms => "kg",
            Self::Grams => "g",
            Self::Fahrenheit => "°F",
            Self::Celsius => "°C",
            Self::Kelvin => "K",
            Self::Gallons => "gal",
            Self::Quarts => "qt",
            Self::FluidOunces => "fl oz",
            Self::Liters => "L",
            Self::Milliliters => "ml",
            Self::CubicMeters => "m³",
            Self::MilesPerHour => "mph",
            Self::KilometersPerHour => "km/h",
        }
    }
}

/// Parse a numeric value with an optional trailing unit (`10.99`, `25 mm`, `36in`).
pub fn parse_input_expression(raw: &str) -> ConvertResult<ParsedInput> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ConvertError::EmptyInput);
    }

    let (number_part, unit_token) = split_value_and_unit(trimmed)?;
    let value = parse_number(&number_part)?;

    let unit = match unit_token {
        Some(token) => Some(
            Unit::from_alias(&token).ok_or_else(|| ConvertError::UnknownUnit(token.clone()))?,
        ),
        None => None,
    };

    Ok(ParsedInput { value, unit })
}

/// Parse and validate a plain number (no unit suffix).
#[cfg_attr(not(test), allow(dead_code))]
pub fn parse_input(raw: &str) -> ConvertResult<Decimal> {
    parse_input_expression(raw).map(|p| p.value)
}

fn split_value_and_unit(raw: &str) -> ConvertResult<(String, Option<String>)> {
    if let Some((num, unit)) = raw.rsplit_once(char::is_whitespace) {
        let unit = unit.trim();
        if !unit.is_empty() {
            return Ok((num.trim().to_string(), Some(unit.to_string())));
        }
    }

    let lower = raw.to_ascii_lowercase();
    for (alias, _) in UNIT_ALIASES {
        if lower.ends_with(alias) && raw.len() > alias.len() {
            let split_at = raw.len() - alias.len();
            let num = raw[..split_at].trim();
            if !num.is_empty() {
                return Ok((num.to_string(), Some(alias.to_string())));
            }
        }
    }

    Ok((raw.to_string(), None))
}

fn parse_number(raw: &str) -> ConvertResult<Decimal> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ConvertError::InvalidNumber);
    }

    if trimmed.contains('e') || trimmed.contains('E') {
        return Err(ConvertError::InvalidNumber);
    }

    let value = Decimal::from_str(trimmed).map_err(|_| ConvertError::InvalidNumber)?;

    if value.is_sign_negative() && value.is_zero() {
        return Err(ConvertError::InvalidNumber);
    }

    if value > max_input() {
        return Err(ConvertError::InputTooLarge);
    }
    if value < min_input() {
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
        Unit::Millimeters => dec!(0.001),
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
        // Speed → meters per second
        Unit::KilometersPerHour => dec!(0.2777777777777777777777777778), // 1/3.6
        Unit::MilesPerHour => dec!(0.44704),                            // 1609.344/3600
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

    #[test]
    fn parse_with_mm_suffix() {
        let p = parse_input_expression("25 mm").unwrap();
        assert_eq!(p.value, dec!(25));
        assert_eq!(p.unit, Some(Unit::Millimeters));
    }

    #[test]
    fn parse_without_suffix() {
        let p = parse_input_expression("10.99").unwrap();
        assert_eq!(p.unit, None);
    }

    #[test]
    fn parse_glued_suffix() {
        let p = parse_input_expression("36in").unwrap();
        assert_eq!(p.unit, Some(Unit::Inches));
    }

    #[test]
    fn mph_to_kph() {
        let p = parse_input_expression("60 mph").unwrap();
        let out = convert(p.value, Unit::MilesPerHour, Unit::KilometersPerHour, 2).unwrap();
        assert_eq!(out, dec!(96.56));
    }

    #[test]
    fn parse_kph_alias() {
        let p = parse_input_expression("100 kph").unwrap();
        assert_eq!(p.unit, Some(Unit::KilometersPerHour));
    }
}
