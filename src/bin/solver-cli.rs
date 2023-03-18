use cli_table::Table;
use fxhash::FxHashMap as HashMap;
use schedual::solver::{Constraint, Include, Priorities};
use schedual::{solver, Class, ClassBank, Days, Schedule, Time};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use tokio::time::Instant;

fn main() -> anyhow::Result<()> {
    let constraints = &[
        Constraint::StartAfter {
            time: Time::new(10, 00),
            days: Days::everyday(),
        },
        // Constraint::EndBefore {
        //     time: Time::new(15, 50),
        //     days: Days::everyday(),
        // },
        Constraint::BlockDays {
            days: Days {
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: true,
                sunday: true,
            },
        },
        Constraint::Campus {
            name: "Boca Raton".to_owned(),
        },
    ];
    let includes = &[
        // Research
        Include::Class { crn: 15308 },
        // Physics
        Include::Course {
            subject: "PHY2048".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "PHY2048L".to_owned(),
            course_type: None,
        },
        // Differential Equations
        // Include::Course {
        //     subject: "MAP2302".to_owned(),
        //     course_type: None,
        // },
        // Calc 3
        Include::Course {
            subject: "MAC2313".to_owned(),
            course_type: None,
        },
        // Intro to Fiction
        Include::Course {
            subject: "LIT2010".to_owned(),
            course_type: None,
        },
        // Include::Class { crn: 15308 },
        // Include::Class { crn: 11436 },
        // Include::Course {
        //     subject: "LIT2010".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "BSC1011".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "BSC1011L".to_owned(),
        //     course_type: None,
        // },
        //
        // Include::Course {
        //     subject: "ACG2021".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "MAD2104".to_owned(),
        //     course_type: None,
        // },
        // Include::Class { crn: 10751 },
        // Include::Course {
        //     subject: "CHM2045".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "CHM2045L".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "PHY2048L".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "MAC2313".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "MAS2103".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "MAP2302".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "CDA3203".to_owned(),
        //     course_type: None,
        // },
    ];

    let priorities = Priorities {
        time_between_classes: 0.4,
        similar_start_time: 0.5,
        similar_end_time: 0.1,
        free_block: 0.0,
        free_day: 2.0,
        day_length: 0.5,
    };
    // let priorities = Priorities::default();
    let mut filters: HashMap<String, Box<dyn Fn(&Class) -> bool>> = HashMap::default();

    //let data = tokio::fs::read_to_string("spring2023bak2/data.json").await.unwrap();
    let data = fs::read_to_string("fall2023/data.json").unwrap();
    let bank: ClassBank = serde_json::from_str(&data)?;

    let start = Instant::now();

    let classes = solver::include_classes(&bank, includes, filters);
    let classes = solver::filter_classes(classes, constraints);
    let filtered = classes.clone();
    let classes = solver::validate_classes(classes);
    let classes = solver::map_classes(classes);

    println!(
        "Total combindnations: {}",
        classes.iter().fold(1, |last, it| last * it.len() as u64)
    );

    let mut soloutions = Vec::new();

    solver::bruteforce_schedules(
        &classes,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut |soloution, times| {
            let mut schedule = Schedule::flatten(times);
            schedule.sort();

            soloutions.push((
                priorities.score(&schedule.data()),
                solver::unmap_classes(&bank, soloution),
            ));
        },
    );

    soloutions.sort_by(|((a, _), _), ((b, _), _)| f64::total_cmp(a, b).reverse());

    for (score, schedule) in soloutions.iter().take(3) {
        println!();
        println!();
        println!("Score: {:?}", score);
        println!(
            "Credits: {}",
            schedule
                .iter()
                .flat_map(|class| class
                    .credit_hours
                    .credit_hours
                    .or(class.credit_hours.credit_hour_low))
                .sum::<u64>()
        );
        let alts = solver::find_alts(&filtered, schedule);
        draw(alts);
    }

    println!(
        "{} solutions found in {:.4}ms",
        soloutions.len(),
        start.elapsed().as_secs_f64() * 1000.
    );
    Ok(())
}

pub fn draw(schedule: Vec<(&Class, Vec<&Class>)>) {
    let mut data: BTreeMap<u8, [String; 8]> = BTreeMap::new();

    for class in schedule {
        for meeting in &class.0.meetings {
            let start_time = meeting.start_time.unwrap();
            let starting_time_id = start_time.hour * 2 + (start_time.min + 10) / 30;
            let end_time = meeting.end_time.unwrap();
            let end_time_id = end_time.hour * 2 + (end_time.min + 10) / 30;

            let mut crns = String::new();
            for alt in &class.1 {
                write!(&mut crns, "{} ", alt.crn).unwrap();
            }

            for time_id in starting_time_id..end_time_id {
                let row = data.entry(time_id).or_default();

                if meeting.days.sunday {
                    write!(&mut row[1], "{}, {} ", class.0.subject_course, crns).unwrap();
                }
                if meeting.days.monday {
                    write!(&mut row[2], "{}, {} ", class.0.subject_course, crns).unwrap();
                }
                if meeting.days.tuesday {
                    write!(&mut row[3], "{}, {} ", class.0.subject_course, crns).unwrap();
                }
                if meeting.days.wednesday {
                    write!(&mut row[4], "{}, {} ", class.0.subject_course, crns).unwrap();
                }
                if meeting.days.thursday {
                    write!(&mut row[5], "{}, {} ", class.0.subject_course, crns).unwrap();
                }
                if meeting.days.friday {
                    write!(&mut row[6], "{}, {} ", class.0.subject_course, crns).unwrap();
                }
                if meeting.days.saturday {
                    write!(&mut row[7], "{}, {} ", class.0.subject_course, crns).unwrap();
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
            data.entry(i).or_insert([
                "".to_owned(),
                " ".to_owned(),
                " ".to_owned(),
                " ".to_owned(),
                " ".to_owned(),
                " ".to_owned(),
                " ".to_owned(),
                " ".to_owned(),
            ])[0] = format!("{:02}:{:02}", hour, min);
        }

        let display = data
            .into_values()
            .table()
            .title(&[
                "Time",
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
            ])
            .display()
            .unwrap();
        println!("{}", display);
    } else {
        println!("No classes");
    }
}
