use crate::{Class, ClassBank, Crn, Days, Schedule, SmallClass, Time};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

//type Classes = HashMap<Include, Vec<Class>>;
type Classes<'a> = HashMap<&'a Include, Vec<&'a Class>>;
type ClassesMapped = Vec<SmallClass>;

pub fn include_classes<'a>(
    classes: &'a ClassBank,
    includes: &'a [Include],
    filters: HashMap<String, Box<dyn Fn(&Class) -> bool>>,
) -> Classes<'a> {
    let mut filtered_classes: Classes = HashMap::new();

    classes
        .iter()
        .filter_map(|(_, class)| {
            for include in includes {
                match include {
                    Include::Class { crn, .. } => {
                        if &class.crn == crn {
                            return Some((include, class));
                        }
                    }
                    Include::Course {
                        subject,
                        course_type,
                        ..
                    } => {
                        if &class.subject_course == subject
                            && filters
                                .get(subject)
                                .map(|filter| (filter)(class))
                                .unwrap_or(true)
                        {
                            if let Some(course_type) = course_type {
                                if &class.schedule_type == course_type {
                                    return Some((include, class));
                                }
                            } else {
                                return Some((include, class));
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

pub fn filter_classes<'a>(mut classes: Classes<'a>, constraints: &[Constraint]) -> Classes<'a> {
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

pub fn validate_classes(mut classes: Classes) -> Classes {
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

pub fn map_classes(classes: Classes) -> Vec<ClassesMapped> {
    classes
        .into_iter()
        .map(|(_include, group)| {
            group
                .into_iter()
                .map(|it| SmallClass {
                    crn: it.crn,
                    schedule: it.schedule.to_owned(),
                })
                .collect_vec()
        })
        .collect_vec()
}

pub fn bruteforce_schedules<'a, F: FnMut(&[Crn])>(
    data: &'a [ClassesMapped],
    classes: &mut Vec<Crn>,
    schedule: &mut Vec<&'a Schedule>,
    callback: &mut F,
) {
    for choice in data.first().iter().flat_map(|it| it.iter()) {
        if !choice.schedule.overlaps(&schedule) {
            classes.push(choice.crn);
            schedule.push(&choice.schedule);

            if data.len() <= 1 {
                // Leaf
                (callback)(&classes)
            } else {
                bruteforce_schedules(&data[1..], classes, schedule, callback);
            }

            classes.pop();
            schedule.pop();
        }
    }
}

pub fn unmap_classes<'a>(bank: &'a ClassBank, classes: &[Crn]) -> Vec<&'a Class> {
    classes
        .into_iter()
        .map(|crn| bank.get(crn).expect("Got bad crn"))
        .collect_vec()
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Constraint {
    BlockTimes { start: Time, end: Time, days: Days },
    BlockDays { days: Days },
    StartAfter { time: Time, days: Days },
    EndBefore { time: Time, days: Days },
    Campus { name: String },
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
                        if let Some((ref start_time, ref end_time)) =
                            meeting.start_time.zip(meeting.end_time)
                        {
                            // todo instead of comparing both ways, just do it wiht the longer class?
                            if (start..=end).contains(&start_time)
                                || (start..=end).contains(&end_time)
                                || (start_time..=end_time).contains(&start)
                                || (start_time..=end_time).contains(&end)
                            {
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

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Priorities {
    pub time_between_classes: f64,
    pub similar_start_time: f64,
    pub similar_end_time: f64,
    pub free_block: f64,
    pub free_day: f64,
    pub day_length: f64,
}

impl Priorities {
    pub fn score(&self, schedule: &Vec<&Class>) -> (f64, Priorities) {
        const VEC_HACK: Vec<(Time, Time)> = Vec::new();
        let mut week = [VEC_HACK; 7];

        for class in schedule {
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

        let mut time_between = [None; 7];
        let mut start_times = [None; 7];
        let mut end_times = [None; 7];
        let mut free_blocks = [None; 7];
        let mut free_days = 0;

        for (id, day) in week.iter_mut().enumerate() {
            day.sort();

            let mut day_time_between = Vec::new();
            for ((_, end), (start, _)) in day.iter().tuple_windows() {
                let end = end.hour as f64 * 60.0 + end.min as f64;
                let start = start.hour as f64 * 60.0 + start.min as f64;
                day_time_between.push(start - end);
            }
            if !day_time_between.is_empty() {
                time_between[id] = day_time_between.iter().copied().min_by(f64::total_cmp);
                free_blocks[id] = day_time_between.iter().copied().max_by(f64::total_cmp);
            }

            if let Some(((start, _), (_, end))) = day.first().zip(day.last()) {
                start_times[id] = Some(start.hour as f64 * 60.0 + start.min as f64);
                end_times[id] = Some(end.hour as f64 * 60.0 + end.min as f64);
            } else {
                free_days += 1;
            }
        }

        let time_between = time_between
            .iter()
            .flatten()
            .sum1::<f64>()
            .map(|sum| sum / time_between.iter().flatten().count() as f64)
            .unwrap_or_default();
        let start_time_average =
            start_times.iter().flatten().sum::<f64>() / start_times.iter().flatten().count() as f64;
        let start_time = (start_times.iter().flatten().fold(0.0, |acc, start| {
            acc + (start - start_time_average) * (start - start_time_average)
        }) / start_times.iter().flatten().count() as f64)
            .sqrt();
        let end_time_average =
            end_times.iter().flatten().sum::<f64>() / end_times.iter().flatten().count() as f64;
        let end_time = (end_times.iter().flatten().fold(0.0, |acc, start| {
            acc + (start - end_time_average) * (start - end_time_average)
        }) / end_times.iter().flatten().count() as f64)
            .sqrt();
        let free_blocks = free_blocks
            .iter()
            .flatten()
            .sum1::<f64>()
            .map(|sum| sum / free_blocks.iter().flatten().count() as f64)
            .unwrap_or_default();
        let day_length_average = end_time_average - start_time_average;
        let free_days = (free_days as f64 - 2.0) * 50.0;

        (
            (time_between * self.time_between_classes
                - start_time * self.similar_start_time
                - end_time * self.similar_end_time
                + free_blocks * self.free_block
                - day_length_average * self.day_length
                + free_days * self.free_day),
            Priorities {
                time_between_classes: time_between,
                similar_start_time: -start_time,
                similar_end_time: -end_time,
                free_block: free_blocks,
                free_day: free_days,
                day_length: -day_length_average,
            },
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum Include {
    Class {
        crn: Crn,
    },
    Course {
        subject: String,
        course_type: Option<String>,
    },
}
