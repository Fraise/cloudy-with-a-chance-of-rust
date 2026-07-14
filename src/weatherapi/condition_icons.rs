//! Maps WeatherAPI.com [`Condition::code`] values to icon file names stored
//! under `icons/` (without the `.bmp` extension).
//!
//! Some WeatherAPI conditions are returned with a `is_day` flag (0 = night,
//! 1 = day). The mapping therefore exposes two functions:
//!
//! - [`icon_for`] — when the day/night flag is not relevant (or the caller
//!   wants a single icon).
//! - [`icon_for_with_day`] — selects a day or night variant when the API
//!   distinguishes between them, falling back to [`icon_for`] otherwise.
//!
//! WeatherAPI condition codes are documented at
//! <https://www.weatherapi.com/docs/weather_conditions.json>.

/// Return the icon name for a WeatherAPI condition `code`, choosing a day
/// variant when `is_day` is true and a night variant when it is false.
///
/// Falls back to `cloud` for any unknown code so callers always get a
/// renderable icon.
pub fn icon_for_with_day(code: i64, is_day: i64) -> &'static str {
    match code {
        // --- Clear / sunny ---
        // WeatherAPI uses the same code (1000) for "Sunny" during the day and
        // "Clear" at night.
        1000 => {
            if is_day == 0 {
                "nights_stay"
            } else {
                "sunny"
            }
        }

        // --- Partly cloudy ---
        // Code 1003 has distinct day/night illustrations, so we honour
        // `is_day` here too.
        1003 => {
            if is_day == 0 {
                "partly_cloudy_night"
            } else {
                "partly_cloudy_day"
            }
        }

        // For all other codes the day/night variants are visually similar
        // enough that we delegate to the day/night-agnostic mapping.
        other => icon_for(other),
    }
}

/// Return the icon name for a WeatherAPI condition `code`, without
/// considering day vs. night.
///
/// Falls back to `cloud` for any unknown code so callers always get a
/// renderable icon.
pub fn icon_for(code: i64) -> &'static str {
    match code {
        // --- Clear / sunny ---
        1000 => "sunny",

        // --- Cloud cover ---
        1003 => "partly_cloudy_day",
        1006 => "cloud",
        1009 => "cloud",

        // --- Atmospheric obstruction (haze, dust, smoke, smog, sand) ---
        // The icon set has no dedicated haze/dust/smoke glyph, so we reuse
        // the generic "air" icon for all of them. `mist` and `foggy` are
        // reserved for the dedicated mist/fog codes further down.
        1012 => "air", // Haze
        1015 => "air", // Dust haze
        1018 => "air", // Blowing dust
        1021 => "air", // Dust storm
        1024 => "air", // Sandstorm
        1027 => "air", // Severe sandstorm
        1030 => "mist",
        1033 => "air", // Smoke
        1036 => "air", // Smoky haze
        1039 => "air", // Smog
        1042 => "air", // Severe smog
        1045 => "air", // Saharan dust
        1048 => "air", // Dust

        // --- Patchy precipitation ---
        1063 => "rainy_light",  // Patchy rain nearby
        1066 => "snowing",      // Patchy snow nearby
        1069 => "weather_mix",  // Patchy sleet nearby
        1072 => "rainy_snow",   // Patchy freezing drizzle nearby
        1087 => "thunderstorm", // Thundery outbreaks nearby

        // --- Blowing / blizzard ---
        1114 => "snowing_heavy", // Blowing snow
        1117 => "snowing_heavy", // Blizzard

        // --- Fog ---
        1135 => "foggy", // Fog
        1147 => "foggy", // Freezing fog

        // --- Drizzle ---
        1150 => "rainy_light", // Patchy light drizzle
        1153 => "rainy_light", // Light drizzle
        1168 => "rainy_snow",  // Freezing drizzle
        1171 => "rainy_snow",  // Heavy freezing drizzle

        // --- Rain ---
        1180 => "rainy_light", // Patchy light rain
        1183 => "rainy_light", // Light rain
        1186 => "rainy",       // Moderate rain at times
        1189 => "rainy",       // Moderate rain
        1192 => "rainy_heavy", // Heavy rain at times
        1195 => "rainy_heavy", // Heavy rain
        1198 => "rainy_snow",  // Light freezing rain
        1201 => "rainy_snow",  // Moderate or heavy freezing rain

        // --- Sleet ---
        1204 => "weather_mix", // Light sleet
        1207 => "weather_mix", // Moderate or heavy sleet

        // --- Snow ---
        1210 => "snowing",       // Patchy light snow
        1213 => "snowing",       // Light snow
        1216 => "snowing",       // Patchy moderate snow
        1219 => "snowing",       // Moderate snow
        1222 => "snowing_heavy", // Patchy heavy snow
        1225 => "snowing_heavy", // Heavy snow

        // --- Ice pellets ---
        1237 => "weather_hail", // Ice pellets

        // --- Rain showers ---
        1240 => "rainy",       // Light rain shower
        1243 => "rainy_heavy", // Moderate or heavy rain shower
        1246 => "rainy_heavy", // Torrential rain shower

        // --- Sleet showers ---
        1249 => "weather_mix", // Light sleet showers
        1252 => "weather_mix", // Moderate or heavy sleet showers

        // --- Snow showers ---
        1255 => "snowing",       // Light snow showers
        1258 => "snowing_heavy", // Moderate or heavy snow showers

        // --- Ice pellet showers ---
        1261 => "weather_hail", // Light showers of ice pellets
        1264 => "weather_hail", // Moderate or heavy showers of ice pellets

        // --- Thunder with precipitation ---
        1273 => "thunderstorm", // Patchy light rain with thunder
        1276 => "thunderstorm", // Moderate or heavy rain with thunder
        1279 => "thunderstorm", // Patchy light snow with thunder
        1282 => "thunderstorm", // Moderate or heavy snow with thunder

        // --- Fallback for any unknown / future code ---
        _ => "cloud",
    }
}
