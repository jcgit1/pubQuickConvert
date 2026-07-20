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

fn pi() -> Decimal {
    Decimal::from_str("3.14159265358979323846264338").expect("pi constant")
}

fn deg_to_rad_factor() -> Decimal {
    pi().checked_div(dec!(180)).expect("deg→rad factor")
}

fn rad_to_deg_factor() -> Decimal {
    dec!(180).checked_div(pi()).expect("rad→deg factor")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitSystem {
    Imperial,
    Metric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitCategory {
    Length,
    Area,
    Weight,
    Volume,
    Speed,
    Force,
    Pressure,
    Energy,
    Power,
    Temperature,
    Angle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    // Length
    Feet,
    Inches,
    Yards,
    Miles,
    Meters,
    Centimeters,
    Millimeters,
    Micrometers,
    Kilometers,
    // Area
    SquareMeters,
    SquareCentimeters,
    SquareFeet,
    SquareInches,
    Acres,
    // Weight (mass)
    Pounds,
    Ounces,
    Kilograms,
    Grams,
    MetricTons,
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
    MetersPerSecond,
    KilometersPerHour,
    MilesPerHour,
    FeetPerSecond,
    Knots,
    // Force
    Newtons,
    Kilonewtons,
    PoundForce,
    // Pressure
    Pascals,
    Kilopascals,
    Megapascals,
    Psi,
    Bar,
    Atmospheres,
    // Energy
    Joules,
    Kilojoules,
    Calories,
    Kilocalories,
    Btu,
    // Power
    Watts,
    Kilowatts,
    Horsepower,
    // Angle
    Degrees,
    Radians,
}

impl UnitCategory {
    pub const ALL: [Self; 11] = [
        Self::Length,
        Self::Area,
        Self::Weight,
        Self::Volume,
        Self::Speed,
        Self::Force,
        Self::Pressure,
        Self::Energy,
        Self::Power,
        Self::Temperature,
        Self::Angle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Area => "Area",
            Self::Weight => "Weight (mass)",
            Self::Temperature => "Temperature",
            Self::Volume => "Volume",
            Self::Speed => "Speed",
            Self::Force => "Force",
            Self::Pressure => "Pressure",
            Self::Energy => "Energy",
            Self::Power => "Power",
            Self::Angle => "Angle",
        }
    }
}

impl Unit {
    pub fn category(self) -> UnitCategory {
        match self {
            Self::Feet | Self::Inches | Self::Yards | Self::Miles | Self::Meters
            | Self::Centimeters | Self::Millimeters | Self::Micrometers | Self::Kilometers => {
                UnitCategory::Length
            }
            Self::SquareMeters | Self::SquareCentimeters | Self::SquareFeet
            | Self::SquareInches | Self::Acres => UnitCategory::Area,
            Self::Pounds | Self::Ounces | Self::Kilograms | Self::Grams | Self::MetricTons => {
                UnitCategory::Weight
            }
            Self::Fahrenheit | Self::Celsius | Self::Kelvin => UnitCategory::Temperature,
            Self::Gallons | Self::Quarts | Self::FluidOunces | Self::Liters
            | Self::Milliliters | Self::CubicMeters => UnitCategory::Volume,
            Self::MetersPerSecond | Self::KilometersPerHour | Self::MilesPerHour
            | Self::FeetPerSecond | Self::Knots => UnitCategory::Speed,
            Self::Newtons | Self::Kilonewtons | Self::PoundForce => UnitCategory::Force,
            Self::Pascals | Self::Kilopascals | Self::Megapascals | Self::Psi | Self::Bar
            | Self::Atmospheres => UnitCategory::Pressure,
            Self::Joules | Self::Kilojoules | Self::Calories | Self::Kilocalories | Self::Btu => {
                UnitCategory::Energy
            }
            Self::Watts | Self::Kilowatts | Self::Horsepower => UnitCategory::Power,
            Self::Degrees | Self::Radians => UnitCategory::Angle,
        }
    }

    pub fn system(self) -> UnitSystem {
        match self {
            Self::Feet | Self::Inches | Self::Yards | Self::Miles | Self::Pounds | Self::Ounces
            | Self::Gallons | Self::Quarts | Self::FluidOunces | Self::Fahrenheit
            | Self::SquareFeet | Self::SquareInches | Self::Acres | Self::MilesPerHour
            | Self::FeetPerSecond | Self::PoundForce | Self::Psi | Self::Btu | Self::Horsepower => {
                UnitSystem::Imperial
            }
            Self::Meters | Self::Centimeters | Self::Millimeters | Self::Micrometers
            | Self::Kilometers | Self::Kilograms | Self::Grams | Self::MetricTons | Self::Liters
            | Self::Milliliters | Self::CubicMeters | Self::Celsius | Self::Kelvin
            | Self::SquareMeters | Self::SquareCentimeters | Self::MetersPerSecond
            | Self::KilometersPerHour | Self::Knots | Self::Newtons | Self::Kilonewtons
            | Self::Pascals | Self::Kilopascals | Self::Megapascals | Self::Bar
            | Self::Atmospheres | Self::Joules | Self::Kilojoules | Self::Calories
            | Self::Kilocalories | Self::Watts | Self::Kilowatts | Self::Degrees
            | Self::Radians => UnitSystem::Metric,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Feet => "Feet",
            Self::Inches => "Inches",
            Self::Yards => "Yards",
            Self::Miles => "Miles",
            Self::Meters => "Meters",
            Self::Centimeters => "Centimeters",
            Self::Millimeters => "Millimeters",
            Self::Micrometers => "Micrometers (µm)",
            Self::Kilometers => "Kilometers",
            Self::SquareMeters => "Square meters",
            Self::SquareCentimeters => "Square centimeters",
            Self::SquareFeet => "Square feet",
            Self::SquareInches => "Square inches",
            Self::Acres => "Acres",
            Self::Pounds => "Pounds (lbs)",
            Self::Ounces => "Ounces",
            Self::Kilograms => "Kilograms",
            Self::Grams => "Grams",
            Self::MetricTons => "Metric tons (t)",
            Self::Fahrenheit => "°F",
            Self::Celsius => "°C",
            Self::Kelvin => "Kelvin",
            Self::Gallons => "Gallons (US)",
            Self::Quarts => "Quarts (US)",
            Self::FluidOunces => "Fluid ounces (US)",
            Self::Liters => "Liters",
            Self::Milliliters => "Milliliters",
            Self::CubicMeters => "Cubic meters",
            Self::MetersPerSecond => "Meters per second (m/s)",
            Self::KilometersPerHour => "Kilometers per hour (km/h)",
            Self::MilesPerHour => "Miles per hour (mph)",
            Self::FeetPerSecond => "Feet per second (ft/s)",
            Self::Knots => "Knots",
            Self::Newtons => "Newtons (N)",
            Self::Kilonewtons => "Kilonewtons (kN)",
            Self::PoundForce => "Pound-force (lbf)",
            Self::Pascals => "Pascals (Pa)",
            Self::Kilopascals => "Kilopascals (kPa)",
            Self::Megapascals => "Megapascals (MPa)",
            Self::Psi => "Pounds per sq in (psi)",
            Self::Bar => "Bar",
            Self::Atmospheres => "Atmospheres (atm)",
            Self::Joules => "Joules (J)",
            Self::Kilojoules => "Kilojoules (kJ)",
            Self::Calories => "Calories (cal)",
            Self::Kilocalories => "Kilocalories (kcal)",
            Self::Btu => "BTU",
            Self::Watts => "Watts (W)",
            Self::Kilowatts => "Kilowatts (kW)",
            Self::Horsepower => "Horsepower (hp)",
            Self::Degrees => "Degrees (°)",
            Self::Radians => "Radians (rad)",
        }
    }

    pub fn units_for_category(category: UnitCategory) -> &'static [Unit] {
        match category {
            UnitCategory::Length => &[
                Unit::Feet,
                Unit::Inches,
                Unit::Yards,
                Unit::Miles,
                Unit::Meters,
                Unit::Centimeters,
                Unit::Millimeters,
                Unit::Micrometers,
                Unit::Kilometers,
            ],
            UnitCategory::Area => &[
                Unit::SquareFeet,
                Unit::SquareInches,
                Unit::Acres,
                Unit::SquareMeters,
                Unit::SquareCentimeters,
            ],
            UnitCategory::Weight => &[
                Unit::Pounds,
                Unit::Ounces,
                Unit::Kilograms,
                Unit::Grams,
                Unit::MetricTons,
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
            UnitCategory::Speed => &[
                Unit::FeetPerSecond,
                Unit::MilesPerHour,
                Unit::Knots,
                Unit::MetersPerSecond,
                Unit::KilometersPerHour,
            ],
            UnitCategory::Force => &[Unit::PoundForce, Unit::Newtons, Unit::Kilonewtons],
            UnitCategory::Pressure => &[
                Unit::Psi,
                Unit::Atmospheres,
                Unit::Pascals,
                Unit::Kilopascals,
                Unit::Megapascals,
                Unit::Bar,
            ],
            UnitCategory::Energy => &[
                Unit::Btu,
                Unit::Joules,
                Unit::Kilojoules,
                Unit::Calories,
                Unit::Kilocalories,
            ],
            UnitCategory::Power => &[Unit::Horsepower, Unit::Watts, Unit::Kilowatts],
            UnitCategory::Angle => &[Unit::Degrees, Unit::Radians],
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

    pub fn compatible_targets(from: Unit) -> Vec<Unit> {
        Unit::units_for_category(from.category()).to_vec()
    }

    pub fn paired_unit(unit: Unit) -> Unit {
        match unit {
            Unit::Feet => Unit::Meters,
            Unit::Meters => Unit::Feet,
            Unit::Inches => Unit::Centimeters,
            Unit::Centimeters => Unit::Inches,
            Unit::Millimeters => Unit::Inches,
            Unit::Micrometers => Unit::Inches,
            Unit::Yards => Unit::Meters,
            Unit::Miles => Unit::Kilometers,
            Unit::Kilometers => Unit::Miles,
            Unit::SquareFeet => Unit::SquareMeters,
            Unit::SquareMeters => Unit::SquareFeet,
            Unit::SquareInches => Unit::SquareCentimeters,
            Unit::SquareCentimeters => Unit::SquareInches,
            Unit::Acres => Unit::SquareMeters,
            Unit::Pounds => Unit::Kilograms,
            Unit::Kilograms => Unit::Pounds,
            Unit::Ounces => Unit::Grams,
            Unit::Grams => Unit::Ounces,
            Unit::MetricTons => Unit::Pounds,
            Unit::Fahrenheit => Unit::Celsius,
            Unit::Celsius => Unit::Fahrenheit,
            Unit::Kelvin => Unit::Celsius,
            Unit::Gallons => Unit::Liters,
            Unit::Liters => Unit::Gallons,
            Unit::Quarts => Unit::Liters,
            Unit::FluidOunces => Unit::Milliliters,
            Unit::Milliliters => Unit::FluidOunces,
            Unit::CubicMeters => Unit::Gallons,
            Unit::MetersPerSecond => Unit::FeetPerSecond,
            Unit::FeetPerSecond => Unit::MetersPerSecond,
            Unit::MilesPerHour => Unit::KilometersPerHour,
            Unit::KilometersPerHour => Unit::MilesPerHour,
            Unit::Knots => Unit::KilometersPerHour,
            Unit::Newtons => Unit::PoundForce,
            Unit::PoundForce => Unit::Newtons,
            Unit::Kilonewtons => Unit::PoundForce,
            Unit::Pascals => Unit::Psi,
            Unit::Psi => Unit::Pascals,
            Unit::Kilopascals => Unit::Psi,
            Unit::Megapascals => Unit::Psi,
            Unit::Bar => Unit::Psi,
            Unit::Atmospheres => Unit::Psi,
            Unit::Joules => Unit::Btu,
            Unit::Btu => Unit::Joules,
            Unit::Kilojoules => Unit::Btu,
            Unit::Calories => Unit::Joules,
            Unit::Kilocalories => Unit::Joules,
            Unit::Watts => Unit::Horsepower,
            Unit::Horsepower => Unit::Watts,
            Unit::Kilowatts => Unit::Horsepower,
            Unit::Degrees => Unit::Radians,
            Unit::Radians => Unit::Degrees,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInput {
    pub value: Decimal,
    pub unit: Option<Unit>,
}

/// Optional unit aliases (longest match first). Case-insensitive.
const UNIT_ALIASES: &[(&str, Unit)] = &[
    // Speed
    ("meters per second", Unit::MetersPerSecond),
    ("metres per second", Unit::MetersPerSecond),
    ("feet per second", Unit::FeetPerSecond),
    ("foot per second", Unit::FeetPerSecond),
    ("miles per hour", Unit::MilesPerHour),
    ("kilometers per hour", Unit::KilometersPerHour),
    ("kilometres per hour", Unit::KilometersPerHour),
    ("km/h", Unit::KilometersPerHour),
    ("kmph", Unit::KilometersPerHour),
    ("kph", Unit::KilometersPerHour),
    ("mph", Unit::MilesPerHour),
    ("mi/h", Unit::MilesPerHour),
    ("m/s", Unit::MetersPerSecond),
    ("ft/s", Unit::FeetPerSecond),
    ("knot", Unit::Knots),
    ("knots", Unit::Knots),
    // Pressure
    ("pounds per square inch", Unit::Psi),
    ("megapascal", Unit::Megapascals),
    ("megapascals", Unit::Megapascals),
    ("kilopascal", Unit::Kilopascals),
    ("kilopascals", Unit::Kilopascals),
    ("atmosphere", Unit::Atmospheres),
    ("atmospheres", Unit::Atmospheres),
    ("pound-force", Unit::PoundForce),
    ("pound force", Unit::PoundForce),
    ("newton", Unit::Newtons),
    ("newtons", Unit::Newtons),
    ("kilonewton", Unit::Kilonewtons),
    ("kilonewtons", Unit::Kilonewtons),
    ("kilojoule", Unit::Kilojoules),
    ("kilojoules", Unit::Kilojoules),
    ("kilocalorie", Unit::Kilocalories),
    ("kilocalories", Unit::Kilocalories),
    ("horsepower", Unit::Horsepower),
    ("micrometer", Unit::Micrometers),
    ("micrometers", Unit::Micrometers),
    ("micrometre", Unit::Micrometers),
    ("micrometres", Unit::Micrometers),
    ("square meter", Unit::SquareMeters),
    ("square meters", Unit::SquareMeters),
    ("square metre", Unit::SquareMeters),
    ("square foot", Unit::SquareFeet),
    ("square feet", Unit::SquareFeet),
    ("square inch", Unit::SquareInches),
    ("square inches", Unit::SquareInches),
    ("metric ton", Unit::MetricTons),
    ("metric tons", Unit::MetricTons),
    ("tonne", Unit::MetricTons),
    ("tonnes", Unit::MetricTons),
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
    ("″", Unit::Inches),
    ("\"", Unit::Inches),
    ("foot", Unit::Feet),
    ("feet", Unit::Feet),
    ("′", Unit::Feet),
    ("'", Unit::Feet),
    ("\u{2019}", Unit::Feet),
    ("yard", Unit::Yards),
    ("yards", Unit::Yards),
    ("mile", Unit::Miles),
    ("miles", Unit::Miles),
    ("acre", Unit::Acres),
    ("acres", Unit::Acres),
    ("radian", Unit::Radians),
    ("radians", Unit::Radians),
    ("degree", Unit::Degrees),
    ("degrees", Unit::Degrees),
    ("joule", Unit::Joules),
    ("joules", Unit::Joules),
    ("watt", Unit::Watts),
    ("watts", Unit::Watts),
    ("pascal", Unit::Pascals),
    ("pascals", Unit::Pascals),
    ("calorie", Unit::Calories),
    ("calories", Unit::Calories),
    ("µm", Unit::Micrometers),
    ("um", Unit::Micrometers),
    ("μm", Unit::Micrometers),
    ("mpa", Unit::Megapascals),
    ("kpa", Unit::Kilopascals),
    ("mm", Unit::Millimeters),
    ("cm", Unit::Centimeters),
    ("km", Unit::Kilometers),
    ("ml", Unit::Milliliters),
    ("oz", Unit::Ounces),
    ("lb", Unit::Pounds),
    ("lbs", Unit::Pounds),
    ("lbf", Unit::PoundForce),
    ("kg", Unit::Kilograms),
    ("ft", Unit::Feet),
    ("yd", Unit::Yards),
    ("mi", Unit::Miles),
    ("gal", Unit::Gallons),
    ("qt", Unit::Quarts),
    ("°f", Unit::Fahrenheit),
    ("°c", Unit::Celsius),
    ("°", Unit::Degrees),
    ("deg", Unit::Degrees),
    ("rad", Unit::Radians),
    ("f", Unit::Fahrenheit),
    ("c", Unit::Celsius),
    ("k", Unit::Kelvin),
    ("m", Unit::Meters),
    ("g", Unit::Grams),
    ("l", Unit::Liters),
    ("in", Unit::Inches),
    ("m3", Unit::CubicMeters),
    ("m2", Unit::SquareMeters),
    ("ft2", Unit::SquareFeet),
    ("in2", Unit::SquareInches),
    ("sqft", Unit::SquareFeet),
    ("sqin", Unit::SquareInches),
    ("psi", Unit::Psi),
    ("atm", Unit::Atmospheres),
    ("bar", Unit::Bar),
    ("pa", Unit::Pascals),
    ("n", Unit::Newtons),
    ("j", Unit::Joules),
    ("kj", Unit::Kilojoules),
    ("cal", Unit::Calories),
    ("kcal", Unit::Kilocalories),
    ("btu", Unit::Btu),
    ("w", Unit::Watts),
    ("kw", Unit::Kilowatts),
    ("hp", Unit::Horsepower),
    ("t", Unit::MetricTons),
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
            Self::Yards => "yd",
            Self::Miles => "mi",
            Self::Meters => "m",
            Self::Centimeters => "cm",
            Self::Millimeters => "mm",
            Self::Micrometers => "µm",
            Self::Kilometers => "km",
            Self::SquareFeet => "ft²",
            Self::SquareInches => "in²",
            Self::SquareMeters => "m²",
            Self::SquareCentimeters => "cm²",
            Self::Acres => "ac",
            Self::Pounds => "lb",
            Self::Ounces => "oz",
            Self::Kilograms => "kg",
            Self::Grams => "g",
            Self::MetricTons => "t",
            Self::Fahrenheit => "°F",
            Self::Celsius => "°C",
            Self::Kelvin => "K",
            Self::Gallons => "gal",
            Self::Quarts => "qt",
            Self::FluidOunces => "fl oz",
            Self::Liters => "L",
            Self::Milliliters => "ml",
            Self::CubicMeters => "m³",
            Self::MetersPerSecond => "m/s",
            Self::MilesPerHour => "mph",
            Self::KilometersPerHour => "km/h",
            Self::FeetPerSecond => "ft/s",
            Self::Knots => "kt",
            Self::Newtons => "N",
            Self::Kilonewtons => "kN",
            Self::PoundForce => "lbf",
            Self::Pascals => "Pa",
            Self::Kilopascals => "kPa",
            Self::Megapascals => "MPa",
            Self::Psi => "psi",
            Self::Bar => "bar",
            Self::Atmospheres => "atm",
            Self::Joules => "J",
            Self::Kilojoules => "kJ",
            Self::Calories => "cal",
            Self::Kilocalories => "kcal",
            Self::Btu => "BTU",
            Self::Watts => "W",
            Self::Kilowatts => "kW",
            Self::Horsepower => "hp",
            Self::Degrees => "°",
            Self::Radians => "rad",
        }
    }
}

pub fn peek_unit_from_input(raw: &str) -> Option<Unit> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(parsed) = parse_input_expression(trimmed) {
        return parsed.unit;
    }

    if let Some(u) = Unit::from_alias(trimmed) {
        return Some(u);
    }

    if let Ok((_, unit_token)) = split_value_and_unit(trimmed) {
        if let Some(token) = unit_token {
            return Unit::from_alias(&token);
        }
    }

    None
}

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
        UnitCategory::Angle => convert_angle(value, from, to)?,
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
        Unit::Micrometers => dec!(0.000001),
        Unit::Kilometers => dec!(1000),
        Unit::Feet => dec!(0.3048),
        Unit::Inches => dec!(0.0254),
        Unit::Yards => dec!(0.9144),
        Unit::Miles => dec!(1609.344),
        // Area → square meters
        Unit::SquareMeters => Decimal::ONE,
        Unit::SquareCentimeters => dec!(0.0001),
        Unit::SquareFeet => dec!(0.09290304),
        Unit::SquareInches => dec!(0.00064516),
        Unit::Acres => dec!(4046.8564224),
        // Weight → kilograms
        Unit::Kilograms => Decimal::ONE,
        Unit::Grams => dec!(0.001),
        Unit::Pounds => dec!(0.45359237),
        Unit::Ounces => dec!(0.028349523125),
        Unit::MetricTons => dec!(1000),
        // Volume → liters
        Unit::Liters => Decimal::ONE,
        Unit::Milliliters => dec!(0.001),
        Unit::CubicMeters => dec!(1000),
        Unit::Gallons => dec!(3.785411784),
        Unit::Quarts => dec!(0.946352946),
        Unit::FluidOunces => dec!(0.0295735295625),
        // Speed → m/s
        Unit::MetersPerSecond => Decimal::ONE,
        Unit::KilometersPerHour => dec!(0.2777777777777777777777777778),
        Unit::MilesPerHour => dec!(0.44704),
        Unit::FeetPerSecond => dec!(0.3048),
        Unit::Knots => dec!(0.514444),
        // Force → newtons
        Unit::Newtons => Decimal::ONE,
        Unit::Kilonewtons => dec!(1000),
        Unit::PoundForce => dec!(4.4482216152605),
        // Pressure → pascals
        Unit::Pascals => Decimal::ONE,
        Unit::Kilopascals => dec!(1000),
        Unit::Megapascals => dec!(1000000),
        Unit::Psi => dec!(6894.757293168),
        Unit::Bar => dec!(100000),
        Unit::Atmospheres => dec!(101325),
        // Energy → joules
        Unit::Joules => Decimal::ONE,
        Unit::Kilojoules => dec!(1000),
        Unit::Calories => dec!(4.184),
        Unit::Kilocalories => dec!(4184),
        Unit::Btu => dec!(1055.05585262),
        // Power → watts
        Unit::Watts => Decimal::ONE,
        Unit::Kilowatts => dec!(1000),
        Unit::Horsepower => dec!(745.69987158227),
        Unit::Fahrenheit | Unit::Celsius | Unit::Kelvin | Unit::Degrees | Unit::Radians => {
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

fn convert_angle(value: Decimal, from: Unit, to: Unit) -> ConvertResult<Decimal> {
    let radians = match from {
        Unit::Radians => value,
        Unit::Degrees => value
            .checked_mul(deg_to_rad_factor())
            .ok_or(ConvertError::Overflow)?,
        _ => return Err(ConvertError::IncompatibleUnits),
    };

    match to {
        Unit::Radians => Ok(radians),
        Unit::Degrees => radians
            .checked_mul(rad_to_deg_factor())
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
    fn parse_inch_double_quote_symbol() {
        let p = parse_input_expression("36\"").unwrap();
        assert_eq!(p.value, dec!(36));
        assert_eq!(p.unit, Some(Unit::Inches));
    }

    #[test]
    fn parse_feet_apostrophe_symbol() {
        let p = parse_input_expression("10'").unwrap();
        assert_eq!(p.value, dec!(10));
        assert_eq!(p.unit, Some(Unit::Feet));
    }

    #[test]
    fn parse_typographic_prime_symbols() {
        let inches = parse_input_expression("12″").unwrap();
        assert_eq!(inches.unit, Some(Unit::Inches));
        let feet = parse_input_expression("8′").unwrap();
        assert_eq!(feet.unit, Some(Unit::Feet));
    }

    #[test]
    fn peek_inch_and_foot_symbols() {
        assert_eq!(peek_unit_from_input("36\""), Some(Unit::Inches));
        assert_eq!(peek_unit_from_input("5'"), Some(Unit::Feet));
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

    #[test]
    fn peek_unit_without_number() {
        assert_eq!(peek_unit_from_input("mm"), Some(Unit::Millimeters));
    }

    #[test]
    fn peek_unit_in_expression() {
        assert_eq!(peek_unit_from_input("60 mph"), Some(Unit::MilesPerHour));
    }

    #[test]
    fn psi_to_kpa() {
        let out = convert(dec!(14.7), Unit::Psi, Unit::Kilopascals, 2).unwrap();
        assert_eq!(out, dec!(101.35));
    }

    #[test]
    fn degrees_to_radians() {
        let out = convert(dec!(180), Unit::Degrees, Unit::Radians, 4).unwrap();
        assert_eq!(out, pi().round_dp(4));
    }

    #[test]
    fn newtons_to_lbf() {
        let out = convert(dec!(1), Unit::Newtons, Unit::PoundForce, 4).unwrap();
        assert_eq!(out, dec!(0.2248));
    }

    #[test]
    fn huge_numeric_suffix_does_not_panic() {
        let input = format!("{} mm", "9".repeat(100_000));
        let _ = parse_input_expression(&input);
    }

    #[test]
    fn huge_garbage_peek_does_not_panic() {
        let input = "x".repeat(500_000);
        let _ = peek_unit_from_input(&input);
    }

    #[test]
    fn rejects_extreme_magnitude() {
        let big = format!("{} m", "9".repeat(20));
        assert_eq!(parse_input_expression(&big), Err(ConvertError::InputTooLarge));
    }
}
