use cli_table::Table;
use schedual::solver::{Include, Priorities};
use schedual::{solver, Class, ClassBank, Schedule};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::fs;
use tokio::time::Instant;

fn main() -> anyhow::Result<()> {
    let constraints = &[
        /*Constraint::StartAfter {
            time: Time::new(9, 00),
            days: Days::everyday(),
        },
        Constraint::EndBefore {
            time: Time::new(15, 00),
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
            subject: "ENC1101".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "ENC1102".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "BSC1005L".to_owned(),
            course_type: None,
        },
        Include::Course {
            subject: "BSC2086L".to_owned(),
            course_type: None,
        },
        // Include::Course {
        //     subject: "BSC1010L".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "CHM2046L".to_owned(),
        //     course_type: None,
        // },
        // Include::Course {
        //     subject: "PHY2049L".to_owned(),
        //     course_type: None,
        // },
    ];
    /*let priorities = Priorities {
        time_between_classes: 3.0,
        similar_start_time: 4.0,
        similar_end_time: 1.0,
        free_block: 0.0,
        free_day: 3.0,
        day_length: 1.0
    };*/
    let priorities = Priorities::default();
    let mut filters: HashMap<String, Box<dyn Fn(&Class) -> bool>> = HashMap::new();
    /*filters.insert("ENC1102".to_owned(), Box::new(|class| {
        for meeting in &class.meetings {
            match meeting.building_code.as_deref() {
                //None | Some("AL") | Some("CU") => return false,
                _ => continue,
            }
        }

        true
    }));*/

    //let data = tokio::fs::read_to_string("spring2023bak2/data.json").await.unwrap();
    let data = fs::read_to_string("spring2023/data.json").unwrap();
    let bank: ClassBank = serde_json::from_str(&data)?;

    let start = Instant::now();

    let classes = solver::include_classes(&bank, includes, filters);
    let classes = solver::filter_classes(classes, constraints);
    let classes = solver::validate_classes(classes);
    let classes = solver::map_classes(classes);

    let mut soloutions = Vec::new();

    solver::bruteforce_schedules(
        &classes,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut |soloution| {
            // soloutions.push(solver::unmap_classes(&bank, soloution));
            soloutions.push(soloution.to_owned());
        },
    );

    // let mut scored_schedules = Vec::new();
    // for schedule in &soloutions {
    //     scored_schedules.push((priorities.score(&schedule), schedule));
    // }
    // scored_schedules.sort_by(|((a, _), _), ((b, _), _)| f64::total_cmp(a, b).reverse());

    /*for (score, schedule) in scored_schedules.iter().take(10) {
        println!();
        println!();
        println!("Score: {:?}", score);
        draw(schedule);
    }*/

    println!(
        "{} solutions found in {:.4}ms",
        soloutions.len(),
        start.elapsed().as_secs_f64() * 1000.
    );
    Ok(())
}

pub fn draw(schedule: &Vec<&Class>) {
    let mut data: BTreeMap<u8, [String; 8]> = BTreeMap::new();

    for class in schedule {
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
                "thursday",
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
