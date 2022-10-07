use std::collections::{BTreeMap, HashMap};
use std::ops::{RangeInclusive, Sub};
use schedual::{ClassBank, Day, Days, Time};

type Room = (String, String, u64); // Building & Room

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut default_day_map: BTreeMap<Day, TimeSeries> = BTreeMap::new();
    for day in Days::everyday().iter() {
        default_day_map.insert(day, TimeSeries::new(Time::new(0, 00), Time::new(24, 00)));
    }
    let default_day_map = default_day_map;

    let data = tokio::fs::read_to_string("fall2022/data.json").await.unwrap();
    let classes: ClassBank = serde_json::from_str(&data)?;

    let mut data: HashMap<Room, BTreeMap<Day, TimeSeries>> = HashMap::new();

    for (_, class) in &classes.classes {
        for meeting in &class.meetings {
            if let (Some(start), Some(end), Some(building), Some(room)) = (meeting.start_time, meeting.end_time, &meeting.building_code, meeting.room) {
                let room = (class.campus.clone(), building.clone(), room);
                let time_range = start..=end;

                for day in meeting.days.iter() {
                    let time_series = data.entry(room.clone()).or_insert_with(|| default_day_map.clone()).entry(day).or_default();
                    *time_series = time_series.clone() - time_range.clone();
                }
            }
        }
    }

    for (room, data) in data {
        println!();
        print!("{} {}{} ", room.0, room.1, room.2);

        for (day, time_series) in data {
            print!("\n\t{:?}: ", day);
            for time_block in time_series.ranges {
                let min = (time_block.end().hour as u64 * 60 + time_block.end().min as u64) - (time_block.start().hour as u64 * 60 + time_block.start().min as u64);
                if min > 0 {
                    print!(" {:02}:{:02} to {:02}:{:02}, {}min; ", time_block.start().hour, time_block.start().min, time_block.end().hour, time_block.end().min, min);
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct TimeSeries {
    ranges: Vec<RangeInclusive<Time>>, // Non overlapping ranges from least to greatest
}

impl TimeSeries {
    fn new(from: Time, to: Time) -> Self {
        Self {
            ranges: vec![from..=to]
        }
    }
}

impl Sub<RangeInclusive<Time>> for TimeSeries {
    type Output = Self;

    fn sub(self, rhs: RangeInclusive<Time>) -> Self::Output {
        let mut new_ranges = Vec::new();

        for range in self.ranges {
            if range.contains(rhs.start()) {
                if range.contains(rhs.end()) {
                    new_ranges.push(*range.start()..=*rhs.start());
                    new_ranges.push(*rhs.end()..=*range.end());
                } else {
                    new_ranges.push(*range.start()..=*rhs.start());
                }
            } else if range.contains(rhs.end()) {
                new_ranges.push(*rhs.end()..=*range.end());
            } else {
                new_ranges.push(range);
            }
        }

        Self {
            ranges: new_ranges
        }
    }
}
