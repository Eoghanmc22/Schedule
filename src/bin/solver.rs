use std::collections::{BTreeMap, HashMap, HashSet};
use rand::prelude::*;
use schedual::{Class, ClassBank, Crn, Days, Time};
use serde::{Serialize, Deserialize};
use std::fmt::Write;
use std::fs;
use cli_table::Table;
use itertools::Itertools;
use tokio::time::Instant;

type Classes = HashMap<Include, Vec<Class>>;

//#[tokio::main]
/*async*/ fn main() -> anyhow::Result<()> {
    let start = Instant::now();

    let constraints = &[
        /*Constraint::StartAfter {
            time: Time::new(9, 00),
            days: Days::everyday(),
        },
        Constraint::EndBefore {
            time: Time::new(17, 00),
            days: Days::everyday(),
        },
        Constraint::BlockDays {
            days: !Days::weekdays(),
        },
        Constraint::Campus {
            name: "Boca Raton".to_owned(),
        }*/
    ];
    let includes = &[
        Include::Course {
            subject: "COP2220".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "ENC1102".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "MAC2312".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "CHM2045".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "CHM2045L".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "EDF2911".to_owned(),
            course_type: None,
        },
    ];
    let priorities = Priorities {
        time_between_classes: 1.0,
        similar_start_time: 4.0,
        similar_end_time: 1.0,
        free_block: 0.0,
        free_day: 3.0,
        day_length: 1.0
    };

    //let data = tokio::fs::read_to_string("spring2023/data.json").await.unwrap();
    let data = fs::read_to_string("spring2023/data.json").unwrap();
    let classes: ClassBank = serde_json::from_str(&data)?;

    let classes = include_classes(classes, includes);
    let classes = filter_classes(classes, constraints);
    let classes = validate_classes(classes);

    /*let mut schedules = HashSet::new();
    for _ in 0..50000 {
        if let Some(schedule) = random_schedule(&classes) {
            schedules.insert(schedule);
        }
    }*/
    let schedules = bruteforce_schedules(classes, Vec::new());

    let mut scored_schedules = Vec::new();
    for schedule in schedules {
        scored_schedules.push((priorities.score(&schedule), schedule));
    }
    scored_schedules.sort_by(|((a, _), _), ((b, _), _)| {
        f64::total_cmp(a, b).reverse()
    });

    /*for (score, schedule) in scored_schedules.iter().take(10) {
        println!();
        println!();
        println!("Score: {:?}", score);
        //schedule.draw();
    }*/

    println!("{} solutions found in {:.4}s", scored_schedules.len(), start.elapsed().as_secs_f64());

    Ok(())
}

fn include_classes(classes: ClassBank, includes: &[Include]) -> Classes {
    let mut filtered_classes: Classes = HashMap::new();

    classes.classes.into_iter()
        .filter_map(|(_, class)| {
            for include in includes {
                match include {
                    Include::Class { crn, .. } => {
                        if &class.crn == crn {
                            return Some((include.clone(), class));
                        }
                    }
                    Include::Course { subject, course_type, .. } => {
                        if &class.subject_course == subject {
                            if let Some(course_type) = course_type {
                                if &class.schedule_type == course_type {
                                    return Some((include.clone(), class));
                                }
                            } else {
                                return Some((include.clone(), class));
                            }
                        }
                    }
                }
            }

            None
        })
        .for_each(|(include, class)| {
            filtered_classes.entry(include).or_default().push(class);
        });

    filtered_classes
}

fn filter_classes(mut classes: Classes, constraints: &[Constraint]) -> Classes {
    classes.values_mut().for_each(|class_group| {
        class_group.retain(|class| {
            for constraint in constraints {
                if !constraint.allows(class) {
                    return false;
                }
            }

            true
        });
    });

    classes
}

fn validate_classes(mut classes: Classes) -> Classes {
    classes.values_mut().for_each(|class_group| {
        let mut seen = HashSet::new();

        class_group.retain(|class| {
            let mut layout = Vec::new();

            for meeting in &class.meetings {
                layout.push((meeting.start_time, meeting.end_time, meeting.days));

                if meeting.start_time.is_none() {
                    return false;
                }
                if meeting.end_time.is_none() {
                    return false;
                }
            }

            if !seen.insert(layout) {
                return false;
            }

            true
        });
    });

    classes
}

fn random_schedule(classes: &Classes) -> Option<Schedule> {
    let mut classes = classes.clone();
    let mut order = classes.iter()
        .map(|(include, class_group)| {
            let class_count = class_group.len();
            (class_count, include.clone())
        })
        .collect::<Vec<(usize, Include)>>();
    order.sort_by_key(|(amount, _)| *amount);
    let mut schedule = Vec::new();

    for (_, include) in order {
        let choices = classes.remove(&include).unwrap();
        let choice = choices.choose(&mut thread_rng())?.clone();

        let constraints = choice.meetings.iter()
            .map(|meeting| Constraint::BlockTimes {
                start: meeting.start_time.unwrap(),
                end: meeting.end_time.unwrap(),
                days: meeting.days
            })
            .collect::<Vec<Constraint>>();

        classes.values_mut().for_each(|class_group|{
            class_group.retain(|class| {
                for constraint in &constraints {
                    if !constraint.allows(class) {
                        return false;
                    }
                }

                true
            })
        });

        schedule.push(choice);
    }

    Some(Schedule {
        classes: schedule
    })
}

fn bruteforce_schedules(classes: Classes, schedule: Vec<Class>) -> Vec<Schedule> { // Returns leafs found
    let choice = classes.iter()
        .min_by_key(|(_, class_group)| {
            class_group.len()
        });

    let mut schedules = Vec::new();

    if let Some((include, choices)) = choice {
        for choice in choices {
            let mut classes = classes.clone();
            classes.remove(include);

            let constraints = choice.meetings.iter()
                .map(|meeting| Constraint::BlockTimes {
                    start: meeting.start_time.unwrap(),
                    end: meeting.end_time.unwrap(),
                    days: meeting.days
                })
                .collect::<Vec<Constraint>>();

            classes.values_mut().for_each(|class_group|{
                class_group.retain(|class| {
                    for constraint in &constraints {
                        if !constraint.allows(class) {
                            return false;
                        }
                    }

                    true
                })
            });

            let mut schedule = schedule.clone();
            schedule.push(choice.clone());

            if classes.is_empty() {
                // Leaf
                schedules.push(Schedule {
                    classes: schedule
                });
            } else if classes.values().map(|it| it.len()).min() == Some(0) {
                // Bad combo
                continue;
            } else {
                schedules.extend(bruteforce_schedules(classes, schedule));
            }
        }
    }

    schedules
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
enum Constraint {
    BlockTimes {
        start: Time,
        end: Time,
        days: Days,
    },
    BlockDays {
        days: Days,
    },
    StartAfter {
        time: Time,
        days: Days,
    },
    EndBefore {
        time: Time,
        days: Days,
    },
    Campus {
        name: String,
    }
}

impl Constraint {
    pub fn allows(&self, class: &Class) -> bool {
        match self {
            Constraint::BlockDays { days } => {
                for meeting in &class.meetings {
                    if meeting.days & *days != Days::never() {
                        return false;
                    }
                }
            }
            Constraint::StartAfter { time, days } => {
                for meeting in &class.meetings {
                    if meeting.days & *days != Days::never() {
                        if let Some(ref start_time) = meeting.start_time {
                            if start_time < time {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                }
            }
            Constraint::EndBefore { time, days } => {
                for meeting in &class.meetings {
                    if meeting.days & *days != Days::never() {
                        if let Some(ref end_time) = meeting.end_time {
                            if end_time > time {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                }
            }
            Constraint::BlockTimes { start, end, days } => {
                for meeting in &class.meetings {
                    if meeting.days & *days != Days::never() {
                        if let Some((ref start_time, ref end_time)) = meeting.start_time.zip(meeting.end_time) {
                            if (start..=end).contains(&start_time) || (start..=end).contains(&end_time) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                }
            }
            Constraint::Campus { name } => {
                if &class.campus != name {
                    return false;
                }
            }
        }

        true
    }
}

#[derive(Clone, Debug)]
struct Priorities {
    time_between_classes: f64,
    similar_start_time: f64,
    similar_end_time: f64,
    free_block: f64,
    free_day: f64,
    day_length: f64
}

impl Priorities {
    pub fn score(&self, schedule: &Schedule) -> (f64, Priorities) {
        const VEC_HACK: Vec<(Time, Time)> = Vec::new();
        let mut week = [VEC_HACK; 7];

        for class in &schedule.classes {
            for meeting in &class.meetings {
                if meeting.days.sunday {
                    week[0].push((meeting.start_time.unwrap(), meeting.end_time.unwrap()));
                }
                if meeting.days.monday {
                    week[1].push((meeting.start_time.unwrap(), meeting.end_time.unwrap()));
                }
                if meeting.days.tuesday {
                    week[2].push((meeting.start_time.unwrap(), meeting.end_time.unwrap()));
                }
                if meeting.days.wednesday {
                    week[3].push((meeting.start_time.unwrap(), meeting.end_time.unwrap()));
                }
                if meeting.days.thursday {
                    week[4].push((meeting.start_time.unwrap(), meeting.end_time.unwrap()));
                }
                if meeting.days.friday {
                    week[5].push((meeting.start_time.unwrap(), meeting.end_time.unwrap()));
                }
                if meeting.days.saturday {
                    week[6].push((meeting.start_time.unwrap(), meeting.end_time.unwrap()));
                }
            }
        }

        let mut time_between = Vec::new();
        let mut start_times = Vec::new();
        let mut end_times = Vec::new();
        let mut free_blocks = Vec::new();
        let mut free_days = 0;

        for day in &mut week {
            day.sort();

            let mut day_time_between = Vec::new();
            for ((_, end), (start, _)) in day.iter().tuple_windows() {
                let end = end.hour as f64 * 60.0 + end.min as f64;
                let start = start.hour as f64 * 60.0 + start.min as f64;
                day_time_between.push(start - end);
            }
            if !day_time_between.is_empty() {
                let average_time_between = day_time_between.iter().sum::<f64>() / day_time_between.len() as f64;
                time_between.push(average_time_between);

                let day_free_blocks = day_time_between.iter().fold(0.0, |acc, free| {
                    acc + *free * *free * *free * *free
                });
                let day_free_blocks = (day_free_blocks / day_time_between.len() as f64).sqrt().sqrt();
                free_blocks.push(day_free_blocks);
            }


            if let Some(((start, _), (_, end))) = day.first().zip(day.last()) {
                start_times.push(start.hour as f64 * 60.0 + start.min as f64);
                end_times.push(end.hour as f64 * 60.0 + end.min as f64);
            } else {
                free_days += 1;
            }
        }

        let time_between = time_between.iter().sum1::<f64>().map(|sum| sum / time_between.len() as f64).unwrap_or_default();
        let start_time_average = start_times.iter().sum::<f64>() / start_times.len() as f64;
        let start_time = (start_times.iter().fold(0.0, |acc, start| {
            acc + (start - start_time_average) * (start - start_time_average)
        }) / start_times.len() as f64).sqrt();
        let end_time_average = end_times.iter().sum::<f64>() / end_times.len() as f64;
        let end_time = (end_times.iter().fold(0.0, |acc, start| {
            acc + (start - end_time_average) * (start - end_time_average)
        }) / end_times.len() as f64).sqrt();
        let free_blocks = free_blocks.iter().sum1::<f64>().map(|sum| sum / free_blocks.len() as f64).unwrap_or_default();
        let day_length_average = end_time_average - start_time_average;
        let free_days = (free_days as f64 - 2.0) * 30.0;

        (
            (
                time_between * self.time_between_classes
                    - start_time * self.similar_start_time
                    - end_time * self.similar_end_time
                    + free_blocks * self.free_block
                    - day_length_average * self.day_length
                    + free_days * self.free_day
            ) / (
                self.time_between_classes +
                    self.similar_end_time +
                    self.free_block +
                    self.similar_start_time +
                    self.day_length +
                    self.free_day
            ),
            Priorities {
                time_between_classes: time_between,
                similar_start_time: -start_time,
                similar_end_time: -end_time,
                free_block: free_blocks,
                free_day: free_days,
                day_length: -day_length_average
            }
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
enum Include {
    Class {
        crn: Crn,
    },
    Course {
        subject: String,
        course_type: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
struct Schedule {
    classes: Vec<Class>
}

impl Schedule {
    pub fn draw(&self) {
        let mut data: BTreeMap<u8, [String; 8]> = BTreeMap::new();

        for class in &self.classes {
            for meeting in &class.meetings {
                let start_time = meeting.start_time.unwrap();
                let starting_time_id = start_time.hour * 2 + (start_time.min + 10) / 30;
                let end_time = meeting.end_time.unwrap();
                let end_time_id = end_time.hour * 2 + (end_time.min + 10) / 30;

                for time_id in starting_time_id..end_time_id {
                    let row = data.entry(time_id).or_default();

                    if meeting.days.sunday {
                        write!(&mut row[1], "{}, {} ", class.subject_course, class.crn).unwrap();
                    }
                    if meeting.days.monday {
                        write!(&mut row[2], "{}, {} ", class.subject_course, class.crn).unwrap();
                    }
                    if meeting.days.tuesday {
                        write!(&mut row[3], "{}, {} ", class.subject_course, class.crn).unwrap();
                    }
                    if meeting.days.wednesday {
                        write!(&mut row[4], "{}, {} ", class.subject_course, class.crn).unwrap();
                    }
                    if meeting.days.thursday {
                        write!(&mut row[5], "{}, {} ", class.subject_course, class.crn).unwrap();
                    }
                    if meeting.days.friday {
                        write!(&mut row[6], "{}, {} ", class.subject_course, class.crn).unwrap();
                    }
                    if meeting.days.saturday {
                        write!(&mut row[7], "{}, {} ", class.subject_course, class.crn).unwrap();
                    }
                }
            }
        }

        if !data.is_empty() {
            let min = *data.keys().min().unwrap();
            let max = *data.keys().max().unwrap();

            for i in min..=max {
                let hour = i / 2;
                let min = i % 2 * 30;
                data.entry(i).or_insert(["".to_owned(), " ".to_owned(), " ".to_owned(), " ".to_owned(), " ".to_owned(), " ".to_owned(), " ".to_owned(), " ".to_owned()])[0] = format!("{:02}:{:02}", hour, min);
            }

            let display = data.into_values().table().title(&["Time", "Sunday", "Monday", "Tuesday", "Wednesday", "thursday", "Friday", "Saturday"]).display().unwrap();
            println!("{}", display);
        } else {
            println!("No classes");
        }
    }
}
