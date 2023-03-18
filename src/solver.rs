use crate::{Class, ClassBank, Crn, Days, Schedule, SmallClass, Time};
use fxhash::FxHashMap as HashMap;
use fxhash::FxHashSet as HashSet;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

//type Classes = HashMap<Include, Vec<Class>>;
type Classes<'a> = HashMap<&'a Include, Vec<&'a Class>>;
type ClassesMapped = Vec<SmallClass>;

pub fn include_classes<'a>(
    classes: &'a ClassBank,
    includes: &'a [Include],
    filters: HashMap<String, Box<dyn Fn(&Class) -> bool>>,
) -> Classes<'a> {
    let mut filtered_classes: Classes = HashMap::default();

    classes
        .iter()
        .filter_map(|(_, class)| {
            for include in includes {
                if include.matches(class)
                    && filters
                        .get(&class.subject_course)
                        .map(|filter| (filter)(class))
                        .unwrap_or(true)
                {
                    return Some((include, class));
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
        let mut seen = HashSet::default();

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

pub fn bruteforce_schedules<'a, F: FnMut(&[Crn], &[&'a Schedule])>(
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
                (callback)(&classes, &schedule);
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

pub fn find_alts<'a>(
    bank: &Classes<'a>,
    classes: &Vec<&'a Class>,
) -> Vec<(&'a Class, Vec<&'a Class>)> {
    classes
        .into_iter()
        .map(|class| {
            (
                &**class,
                bank.into_iter()
                    .filter(|(include, _)| include.matches(class))
                    .flat_map(|(_, classes)| classes.into_iter())
                    .filter(|it| {
                        it.subject_course == class.subject_course && it.schedule == class.schedule
                    })
                    .map(|it| &**it)
                    .collect(),
            )
        })
        .collect()
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
    pub similar_start_time: f64,
    pub similar_end_time: f64,
    pub time_between_classes: f64,
    pub free_block: f64,
    pub free_day: f64,
    pub day_length: f64,
}

impl Priorities {
    pub fn score(&self, schedule: &[&[(u16, u16)]; 7]) -> (f64, Priorities) {
        let mut start_time_avg = 0;
        let mut end_time_total = 0;
        let mut free_blocks_total = 0;
        let mut time_between_avg = (0, 0);
        let mut days = 0;
        let mut start_times = [None; 7];
        let mut end_times = [None; 7];

        for (idx, day) in schedule.iter().enumerate() {
            if let Some((first, last)) = Option::zip(day.first(), day.last()) {
                let start_time = first.0;
                let end_time = last.0 + last.1;

                start_time_avg += start_time;
                end_time_total += end_time;
                days += 1;

                start_times[idx] = Some(start_time);
                end_times[idx] = Some(end_time);

                let mut free_block = 0;
                for (class_a, class_b) in day.iter().tuple_windows() {
                    let time_between = class_b.0 - (class_a.0 + class_a.1);

                    free_block = free_block.max(time_between);

                    time_between_avg.0 += time_between;
                    time_between_avg.1 += 1;
                }
                if free_block != 0 {
                    free_blocks_total += free_block;
                }
            }
        }

        let start_time = u16::checked_div(start_time_avg, days).unwrap_or_default();
        let end_time = u16::checked_div(end_time_total, days).unwrap_or_default();
        let day_length = end_time - start_time;

        let similar_start_time = i32::checked_div(
            start_times
                .into_iter()
                .flatten()
                .map(|start| start as i32 - start_time as i32)
                .map(|score| score * score)
                .sum(),
            days as i32,
        )
        .unwrap_or_default();
        let similar_end_time = i32::checked_div(
            end_times
                .into_iter()
                .flatten()
                .map(|end| end as i32 - end_time as i32)
                .map(|score| score * score)
                .sum(),
            days as i32,
        )
        .unwrap_or_default();

        let time_between =
            u16::checked_div(time_between_avg.0, time_between_avg.1).unwrap_or_default();
        let free_blocks = u16::checked_div(free_blocks_total, days).unwrap_or_default();

        let free_days = 5 - days as i32;

        (
            (0.0 - similar_start_time as f64 * self.similar_start_time
                - similar_end_time as f64 * self.similar_end_time
                + time_between as f64 * self.time_between_classes
                + free_blocks as f64 * self.free_block
                + free_days as f64 * self.free_day
                - day_length as f64 * self.day_length),
            Priorities {
                similar_start_time: -(similar_start_time as f64),
                similar_end_time: -(similar_end_time as f64),
                time_between_classes: time_between as f64,
                free_block: free_blocks as f64,
                free_day: free_days as f64,
                day_length: -(day_length as f64),
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
    All,
}

impl Include {
    pub fn matches(&self, class: &Class) -> bool {
        match self {
            Include::Class { crn, .. } => {
                if &class.crn == crn {
                    return true;
                }
            }
            Include::Course {
                subject,
                course_type,
                ..
            } => {
                if &class.subject_course == subject {
                    if let Some(course_type) = course_type {
                        if &class.schedule_type == course_type {
                            return true;
                        }
                    } else {
                        return true;
                    }
                }
            }
            Include::All => return true,
        }

        false
    }
}
