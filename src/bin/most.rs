use fxhash::FxHashMap as HashMap;
use itertools::Itertools;
use schedual::ClassBank;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let data = tokio::fs::read_to_string("spring2023/data.json")
        .await
        .unwrap();
    let classes: ClassBank = serde_json::from_str(&data)?;
    let mut counters = HashMap::default();

    'mainloop: for (_, class) in classes {
        for meeting in class.meetings {
            if meeting.start_time.is_none() || meeting.end_time.is_none() {
                continue 'mainloop;
            }
        }

        *counters.entry(class.subject_course).or_insert(0) += 1;
    }

    let mut sorted_counters = counters.into_iter().collect_vec();
    sorted_counters.sort_by(|(_, count_a), (_, count_b)| u32::cmp(count_a, count_b).reverse());

    for (subject, count) in sorted_counters.iter().take(15) {
        println!("{}: {}", subject, count);
    }

    Ok(())
}
