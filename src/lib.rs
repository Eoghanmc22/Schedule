pub mod solver;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::{BitAnd, BitOr, Not};
use std::str::FromStr;
use anyhow::{bail, Context, ensure};
use itertools::Itertools;
use serde::{Serialize, Deserialize};

//TODO use refs

pub type Crn = u64;

pub type ClassBank =  BTreeMap<Crn, Class>;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Class {
    pub campus: String,
    pub crn: Crn,
    pub course_number: String,
    pub name: String,
    pub credit_hours: CreditHours,
    pub cross_list: Option<CrossList>,
    pub enrollment: Enrollment,
    pub wait_list: Enrollment,
    pub faculty: Vec<Faculty>,
    pub instructional_method: String,
    pub meetings: Vec<Session>,
    pub open: bool,
    pub part_of_term: String,
    pub schedule_type: String,
    pub sequence_number: String,
    pub special_approval: Option<String>,
    pub subject_course: String,
    pub subject_description: String,
    pub term: String,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CreditHours {
    pub credit_hour_high: Option<u64>,
    pub credit_hour_low: Option<u64>,
    pub credit_hours: Option<u64>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CrossList {
    pub cross_list: u64,
    pub cross_list_available: i64,
    pub cross_list_capacity: u64,
    pub cross_list_count: u64,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Enrollment {
    pub count: u64,
    pub capacity: u64,
    pub available: i64
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Faculty {
    pub name: String,
    pub email: Option<String>,
    pub primary: bool
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Session {
    // TODO represent time better
    pub start_time: Option<Time>,
    pub end_time: Option<Time>,
    pub start_date: String,
    pub end_date: String,
    pub days: Days,

    pub building_code: Option<String>,
    pub building_name: Option<String>,
    pub room: Option<u64>,

    pub meeting_type: String,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Days {
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
}

impl Days {
    pub const fn everyday() -> Self {
        Self {
            monday: true,
            tuesday: true,
            wednesday: true,
            thursday: true,
            friday: true,
            saturday: true,
            sunday: true
        }
    }

    pub const fn never() -> Self {
        Self {
            monday: false,
            tuesday: false,
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: false,
            sunday: false
        }
    }

    pub const fn weekdays() -> Self {
        Self {
            monday: true,
            tuesday: true,
            wednesday: true,
            thursday: true,
            friday: true,
            saturday: false,
            sunday: false
        }
    }

    pub const fn mwf() -> Self {
        Self {
            monday: true,
            tuesday: false,
            wednesday: true,
            thursday: false,
            friday: true,
            saturday: false,
            sunday: false
        }
    }

    pub const fn ttf() -> Self {
        Self {
            monday: false,
            tuesday: true,
            wednesday: false,
            thursday: true,
            friday: true,
            saturday: false,
            sunday: false
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Day> {
        let mut vec = Vec::new();

        if self.sunday {
            vec.push(Day::Sunday);
        }
        if self.monday {
            vec.push(Day::Monday);
        }
        if self.tuesday {
            vec.push(Day::Tuesday);
        }
        if self.wednesday {
            vec.push(Day::Wednesday);
        }
        if self.thursday {
            vec.push(Day::Thursday);
        }
        if self.friday {
            vec.push(Day::Friday);
        }
        if self.saturday {
            vec.push(Day::Saturday);
        }

        vec.into_iter()
    }
}

impl BitAnd for Days {
    type Output = Days;

    fn bitand(self, rhs: Self) -> Self::Output {
        Days {
            monday: self.monday && rhs.monday,
            tuesday: self.tuesday && rhs.tuesday,
            wednesday: self.wednesday && rhs.wednesday,
            thursday: self.thursday && rhs.thursday,
            friday: self.friday && rhs.friday,
            saturday: self.saturday && rhs.saturday,
            sunday: self.sunday && rhs.sunday
        }
    }
}

impl BitOr for Days {
    type Output = Days;

    fn bitor(self, rhs: Self) -> Self::Output {
        Days {
            monday: self.monday || rhs.monday,
            tuesday: self.tuesday || rhs.tuesday,
            wednesday: self.wednesday || rhs.wednesday,
            thursday: self.thursday || rhs.thursday,
            friday: self.friday || rhs.friday,
            saturday: self.saturday || rhs.saturday,
            sunday: self.sunday || rhs.sunday
        }
    }
}

impl Not for Days {
    type Output = Days;

    fn not(self) -> Self::Output {
        Days {
            monday: !self.monday,
            tuesday: !self.tuesday,
            wednesday: !self.wednesday,
            thursday: !self.thursday,
            friday: !self.friday,
            saturday: !self.saturday,
            sunday: !self.sunday
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Day {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Time {
    pub hour: u8,
    pub min: u8,
}

impl Time {
    pub fn new(hour: u8, min: u8) -> Self {
        Self { hour, min }
    }
}

impl FromStr for Time {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ensure!(s.len() == 5, "Bad time: {s}");

        let (hour, min) = s.split(':').collect_tuple().context("No separator")?;

        let hour : u8 = hour.parse().context("Bad hour")?;
        let min : u8 = min.parse().context("Bad minute")?;

        ensure!((0..24).contains(&hour), "Invalid hour {hour}");
        ensure!((0..60).contains(&min), "Invalid minute {min}");

        Ok(Self {
            hour,
            min
        })
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Time::cmp(self, other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else {
            match u8::cmp(&self.hour, &other.hour) {
                Ordering::Equal => {
                    u8::cmp(&self.min, &other.min)
                }
                ord => ord
            }
        }
    }
}
