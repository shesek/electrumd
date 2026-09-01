pub const HAS_FEATURE: bool = cfg!(any(
    feature = "4_1_5",
    feature = "4_5_4",
    feature = "4_6_2",
    feature = "4_7_0"
));

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const EXE_REL_PATH: &str = "electrum.AppImage";

#[cfg(target_os = "macos")]
pub const EXE_REL_PATH: &str = "Electrum.app/Contents/MacOS/run_electrum";

#[cfg(not(any(
    feature = "4_1_5",
    feature = "4_5_4",
    feature = "4_6_2",
    feature = "4_7_0"
)))]
pub const VERSION: &str = "N/A";

#[cfg(feature = "4_1_5")]
pub const VERSION: &str = "4.1.5";

#[cfg(feature = "4_5_4")]
pub const VERSION: &str = "4.5.4";

#[cfg(feature = "4_6_2")]
pub const VERSION: &str = "4.6.2";

#[cfg(feature = "4_7_0")]
pub const VERSION: &str = "4.7.0";
