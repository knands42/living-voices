use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
enum GraphicsCard {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
#[repr(u8)]
enum VramAvailable {
    EightGB = 8,
    TwelveGB = 12,
    SixteenGB = 16,
    TwentyFourGB = 24,
    Unknown = 0,
}
