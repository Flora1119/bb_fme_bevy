use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum CardinalDirection {
    Up = 0,
    Right = 1,
    Down = 2,
    Left = 3,
}

impl CardinalDirection {
    pub const ALL: [Self; 4] = [Self::Up, Self::Right, Self::Down, Self::Left];

    pub const fn index(self) -> i32 {
        self as i32
    }

    pub const fn unit_offset(self) -> (i32, i32) {
        match self {
            Self::Up => (0, 1),
            Self::Right => (1, 0),
            Self::Down => (0, -1),
            Self::Left => (-1, 0),
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Right => Self::Left,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
        }
    }

    pub const fn clockwise(self) -> Self {
        match self {
            Self::Up => Self::Right,
            Self::Right => Self::Down,
            Self::Down => Self::Left,
            Self::Left => Self::Up,
        }
    }

    pub const fn counterclockwise(self) -> Self {
        match self {
            Self::Up => Self::Left,
            Self::Right => Self::Up,
            Self::Down => Self::Right,
            Self::Left => Self::Down,
        }
    }

    /// Unity BB_FME에서 사용하는 Z축 회전각입니다.
    pub const fn unity_angle_degrees(self) -> i32 {
        -90 * self.index()
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Right => "right",
            Self::Down => "down",
            Self::Left => "left",
        }
    }
}

impl TryFrom<i32> for CardinalDirection {
    type Error = InvalidCardinalDirection;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Up),
            1 => Ok(Self::Right),
            2 => Ok(Self::Down),
            3 => Ok(Self::Left),
            _ => Err(InvalidCardinalDirection { value }),
        }
    }
}

impl From<CardinalDirection> for i32 {
    fn from(direction: CardinalDirection) -> Self {
        direction.index()
    }
}

impl fmt::Display for CardinalDirection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidCardinalDirection {
    value: i32,
}

impl InvalidCardinalDirection {
    pub const fn value(self) -> i32 {
        self.value
    }
}

impl fmt::Display for InvalidCardinalDirection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "direction index must be in 0..=3, found {}",
            self.value
        )
    }
}

impl Error for InvalidCardinalDirection {}
