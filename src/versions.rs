pub const HAS_FEATURE: bool = cfg!(any(
    feature = "4_1_5",
));

#[cfg(not(any(
    feature = "4_1_5",
)))]
pub const VERSION: &str = "N/A";

#[cfg(feature = "4_1_5")]
pub const VERSION: &str = "4.1.5";