use itertools::Itertools;
use schedual::{Class, CreditHours, CrossList, Days, Enrollment, Faculty, Schedule, Session};
use serde_json::Value;
use std::collections::BTreeMap;

#[tokio::main]
async fn main() {
    let data = tokio::fs::read_to_string("spring2023/raw_data.json")
        .await
        .unwrap();
    let json: Value = serde_json::from_str(&data).unwrap();

    let mut classes = BTreeMap::new();
    for class in json.get("data").unwrap().as_array().unwrap() {
        let credit_hours = CreditHours {
            credit_hour_high: class.get("creditHourHigh").and_then(|val| val.as_u64()),
            credit_hour_low: class.get("creditHourLow").and_then(|val| val.as_u64()),
            credit_hours: class.get("creditHours").and_then(|val| val.as_u64()),
        };

        let cross_list = class
            .get("crossList")
            .and_then(|val| val.as_u64())
            .map(|cross_list| CrossList {
                cross_list,
                cross_list_available: class
                    .get("crossListAvailable")
                    .and_then(|val| val.as_i64())
                    .unwrap(),
                cross_list_capacity: class
                    .get("crossListCapacity")
                    .and_then(|val| val.as_u64())
                    .unwrap(),
                cross_list_count: class
                    .get("crossListCount")
                    .and_then(|val| val.as_u64())
                    .unwrap(),
            });

        let enrollment = Enrollment {
            count: class
                .get("enrollment")
                .and_then(|val| val.as_u64())
                .unwrap(),
            capacity: class
                .get("maximumEnrollment")
                .and_then(|val| val.as_u64())
                .unwrap(),
            available: class
                .get("seatsAvailable")
                .and_then(|val| val.as_i64())
                .unwrap(),
        };

        let wait_list = Enrollment {
            count: class.get("waitCount").and_then(|val| val.as_u64()).unwrap(),
            capacity: class
                .get("waitCapacity")
                .and_then(|val| val.as_u64())
                .unwrap(),
            available: class
                .get("waitAvailable")
                .and_then(|val| val.as_i64())
                .unwrap(),
        };

        let mut faculty = Vec::new();
        for faculty1 in class.get("faculty").and_then(|val| val.as_array()).unwrap() {
            faculty.push(Faculty {
                name: faculty1
                    .get("displayName")
                    .and_then(|val| val.as_str())
                    .unwrap()
                    .to_owned(),
                email: faculty1
                    .get("emailAddress")
                    .and_then(|val| val.as_str())
                    .map(|val| val.to_owned()),
                primary: faculty1
                    .get("primaryIndicator")
                    .and_then(|val| val.as_bool())
                    .unwrap(),
            })
        }

        let mut meetings = Vec::new();
        for session in class
            .get("meetingsFaculty")
            .and_then(|val| val.as_array())
            .unwrap()
        {
            let session = session.get("meetingTime").unwrap();

            meetings.push(Session {
                start_time: session
                    .get("beginTime")
                    .and_then(|val| val.as_str())
                    .and_then(|val| val.parse().ok()),
                end_time: session
                    .get("endTime")
                    .and_then(|val| val.as_str())
                    .and_then(|val| val.parse().ok()),
                start_date: session
                    .get("startDate")
                    .and_then(|val| val.as_str())
                    .unwrap()
                    .to_owned(),
                end_date: session
                    .get("endDate")
                    .and_then(|val| val.as_str())
                    .unwrap()
                    .to_owned(),
                days: Days {
                    monday: session.get("monday").and_then(|val| val.as_bool()).unwrap(),
                    tuesday: session
                        .get("tuesday")
                        .and_then(|val| val.as_bool())
                        .unwrap(),
                    wednesday: session
                        .get("wednesday")
                        .and_then(|val| val.as_bool())
                        .unwrap(),
                    thursday: session
                        .get("thursday")
                        .and_then(|val| val.as_bool())
                        .unwrap(),
                    friday: session.get("friday").and_then(|val| val.as_bool()).unwrap(),
                    saturday: session
                        .get("saturday")
                        .and_then(|val| val.as_bool())
                        .unwrap(),
                    sunday: session.get("sunday").and_then(|val| val.as_bool()).unwrap(),
                },
                building_code: session
                    .get("building")
                    .and_then(|val| val.as_str())
                    .map(|val| val.to_owned()),
                building_name: session
                    .get("buildingDescription")
                    .and_then(|val| val.as_str())
                    .map(|val| val.to_owned()),
                room: session
                    .get("room")
                    .and_then(|val| val.as_str())
                    .and_then(|val| val.parse::<u64>().ok()),
                meeting_type: session
                    .get("meetingTypeDescription")
                    .and_then(|val| val.as_str())
                    .unwrap()
                    .to_owned(),
            })
        }

        let schedule = Schedule::generate(
            &meetings
                .iter()
                .flat_map(|it| {
                    if let (Some(s), Some(e)) = (it.start_time, it.end_time) {
                        Some((it.days, s, e))
                    } else {
                        None
                    }
                })
                .collect_vec(),
        );

        let class = Class {
            campus: class
                .get("campusDescription")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            crn: class
                .get("courseReferenceNumber")
                .and_then(|val| val.as_str())
                .and_then(|val| val.parse::<u64>().ok())
                .unwrap(),
            course_number: class
                .get("courseNumber")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            name: class
                .get("courseTitle")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            credit_hours,
            cross_list,
            enrollment,
            wait_list,
            faculty,
            instructional_method: class
                .get("instructionalMethodDescription")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            meetings,
            open: class
                .get("openSection")
                .and_then(|val| val.as_bool())
                .unwrap(),
            part_of_term: class
                .get("partOfTermDescription")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            schedule_type: class
                .get("scheduleTypeDescription")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            sequence_number: class
                .get("sequenceNumber")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            special_approval: class
                .get("specialApprovalDescription")
                .and_then(|val| val.as_str())
                .map(|val| val.to_owned()),
            subject_course: class
                .get("subjectCourse")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            subject_description: class
                .get("subjectDescription")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            term: class
                .get("termDesc")
                .and_then(|val| val.as_str())
                .unwrap()
                .to_owned(),
            schedule,
        };

        classes.insert(class.crn, class);
    }

    println!("classes: {}", classes.len());

    let data = serde_json::to_string_pretty(&classes).unwrap();
    tokio::fs::write("spring2023/data.json", data)
        .await
        .unwrap();
}
