use std::fs;

use schedual::ClassBank;

fn main() -> anyhow::Result<()> {
    let data = fs::read_to_string("spring2023bak/data.json").unwrap();
    let classes: ClassBank = serde_json::from_str(&data)?;

    let mut bad_start = 0;
    let mut bad_end = 0;
    let mut good_start = 0;
    let mut good_end = 0;

    let resolution = 10;

    for class in classes.values() {
        for meeting in &class.meetings {
            if let Some(start_time) = meeting.start_time {
                if start_time.min % resolution != 0 {
                    println!(
                        "S {}, {:?}, {}",
                        class.crn,
                        start_time,
                        start_time.min % resolution
                    );

                    bad_start += 1;
                } else {
                    good_start += 1;
                }
            }

            if let Some(end_time) = meeting.end_time {
                if (end_time.min + 10) % resolution != 0 {
                    println!(
                        "E {}, {:?}, {}",
                        class.crn,
                        end_time,
                        (end_time.min + 10) % resolution
                    );

                    bad_end += 1;
                } else {
                    good_end += 1;
                }
            }
        }
    }

    println!("Start {bad_start}:{good_start}");
    println!("End {bad_end}:{good_end}");

    Ok(())
}
