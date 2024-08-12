pub const HAS_FEATURE: bool = cfg!(any(feature = "4_1_5", feature = "4_5_4"));

#[cfg(not(any(feature = "4_1_5", feature = "4_5_4")))]
pub const VERSION: &str = "N/A";

#[cfg(feature = "4_1_5")]
pub const VERSION: &str = "4.1.5";

#[cfg(feature = "4_5_4")]
pub const VERSION: &str = "4.5.4";
