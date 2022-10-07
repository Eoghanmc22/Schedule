use std::time::Duration;
use clap::Parser;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use serde::Serialize;

#[tokio::main]
async fn main() {
    let args: Args = Args::parse();

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open("../../fall2022/raw_data.json")
        .await
        .expect("open");

    println!("Using endpoint: {}", args.endpoint);
    println!("Using term: {}", args.term);
    println!("Using cookies: {}", args.cookies);

    let mut data = Vec::new();

    let client = Client::new();
    loop {
        let response = client.get(&args.endpoint)
            .header("Cookie", &args.cookies)
            .query(&[("txt_term", &args.term)])
            .query(&[("pageOffset", &data.len().to_string())])
            .query(&[("pageMaxSize", "1000")])
            .query(&[("sortColumn", "subjectDescription")])
            .query(&[("sortDirection", "asc")])
            .send()
            .await.expect("get");
        assert_eq!(response.status(), StatusCode::OK);
        //println!("{:?}", response);

        let json: Value = response.json().await.expect("json");
        //println!("{:?}", json);

        let success =
            json.get("success")
                .and_then(|success| success.as_bool())
                .unwrap_or_default();
        assert!(success);

        let new_data =
            json.get("data")
                .and_then(|data| data.as_array())
                .expect("class data");

        if new_data.len() == 0 {
            println!("Hit 0 len");
            break;
        }

        println!("Pulled {}", new_data.len());

        data.extend_from_slice(&new_data);

        let sections_fetched_count =
            json.get("sectionsFetchedCount")
                .and_then(|sections| sections.as_u64())
                .expect("Sections");

        if data.len() >= sections_fetched_count as usize {
            break;
        }

        tokio::time::sleep(Duration::from_secs(7)).await;
    }

    let structure = BasicStructure {
        data: Value::Array(data)
    };

    let string = serde_json::to_string_pretty(&structure).expect("to string");
    file.write_all(string.as_bytes()).await.expect("write");
}

#[derive(Parser, Clone, Debug)]
struct Args {
    #[clap(short = 'e')]
    endpoint: String,
    #[clap(short = 't')]
    term: String,
    #[clap(short = 'c')]
    cookies: String,
}

#[derive(Serialize)]
struct BasicStructure {
    data: Value
}
