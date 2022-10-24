use std::collections::HashMap;
use std::fs;
use eframe::{Frame, Storage};
use egui::{Context, Id, Pos2, Rect, Rgba, RichText, Rounding, Sense, Stroke, Ui, Vec2};
use egui_extras::{Size, StripBuilder};
use serde::{Serialize, Deserialize};
use schedual::{ClassBank, Crn, Days, solver, Time};
use schedual::solver::{Constraint, Include, Priorities, Schedule};

// TODO better input validation

fn main() -> anyhow::Result<()> {
    let data = fs::read_to_string("spring2023/data.json").unwrap();
    let classes: ClassBank = serde_json::from_str(&data)?;

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Scheduler",
        native_options,
        Box::new(|cc| Box::new(ScheduleApp::new(classes, cc))),
    );

    Ok(())
}

#[derive(Default)]
struct ScheduleApp {
    raw_classes: ClassBank,
    sorted_schedules: Vec<((f64, Priorities), Schedule)>,
    persistent: PersistentData,

    create_class_window: Option<CreateClassWindow>,
    create_constraint_window: Option<CreateConstraintWindow>,
    displayed_schedules: Vec<DisplayedSchedule>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct PersistentData {
    constraints: Vec<Constraint>,
    includes: Vec<Include>,
    priorities: Priorities,
    // todo filters?
}

// TODO better way than String?
struct CreateClassWindow(Include, String);
struct CreateConstraintWindow(Constraint, String, String);
struct DisplayedSchedule(((f64, Priorities), Schedule));

impl ScheduleApp {
    fn new(raw_classes: ClassBank, cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let persistent = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        Self {
            raw_classes,
            persistent,
            ..Default::default()
        }
    }

    // TODO run in parallel
    fn generate_schedules(&mut self) {
        let classes = &self.raw_classes;
        let includes = &self.persistent.includes;
        let constraints = &self.persistent.constraints;
        let priorities = &self.persistent.priorities;

        // Filter classes
        let classes = solver::include_classes(classes, includes, HashMap::new());
        let classes = solver::filter_classes(classes, constraints);
        let classes = solver::validate_classes(classes);

        // Bruteforce schedules
        let schedules = solver::bruteforce_schedules(classes, Vec::new()).into_iter().collect::<Vec<_>>();

        // Score schedules
        let mut scored_schedules = Vec::new();
        for schedule in schedules {
            scored_schedules.push((priorities.score(&schedule), schedule));
        }
        scored_schedules.sort_by(|((a, _), _), ((b, _), _)| {
            f64::total_cmp(a, b).reverse()
        });

        self.sorted_schedules = scored_schedules;
    }
}

impl eframe::App for ScheduleApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Schedule solver");
            ui.collapsing("Classes", |ui| {
                if ui.button("Add Class").clicked() {
                    self.create_class_window = Some(CreateClassWindow(Include::Course { subject: "".to_string(), course_type: None }, String::new()));
                }

                let mut remove = None;
                for (idx, include) in self.persistent.includes.iter().enumerate() {
                    match include {
                        Include::Class { crn } => {
                            let subject = if let Some(class) = self.raw_classes.get(crn) {
                                &class.subject_course
                            } else {
                                "Unknown Class"
                            };
                            ui.label(format!("{}: {}", subject, crn));
                        }
                        Include::Course { subject, course_type } => {
                            ui.label(format!("{} {}", subject, course_type.as_ref().map(|it| it.as_str()).unwrap_or("")));
                        }
                    }

                    if ui.button("Remove").clicked() {
                        remove = Some(idx);
                    }
                }

                if let Some(idx) = remove {
                    self.persistent.includes.remove(idx);
                }
            });
            ui.collapsing("Constraints", |ui| {
                if ui.button("Add Constraint").clicked() {
                    self.create_constraint_window = Some(CreateConstraintWindow(Constraint::Campus { name: "Boca Raton".to_string() }, String::new(), String::new()));
                }

                let mut remove = None;
                for (idx, include) in self.persistent.constraints.iter().enumerate() {
                    // TODO improve
                    ui.label(format!("{:?}", include));

                    if ui.button("Remove").clicked() {
                        remove = Some(idx);
                    }
                }

                if let Some(idx) = remove {
                    self.persistent.constraints.remove(idx);
                }
            });
            ui.collapsing("Priorities", |ui| {
                let priorities = &mut self.persistent.priorities;
                ui.label("Time between classes");
                ui.add(egui::Slider::new(&mut priorities.time_between_classes, -5.0..=5.0));
                ui.label("Similar Start Times");
                ui.add(egui::Slider::new(&mut priorities.similar_start_time, -5.0..=5.0));
                ui.label("Similar End Times");
                ui.add(egui::Slider::new(&mut priorities.similar_end_time, -5.0..=5.0));
                ui.label("Free Time Blocks");
                ui.add(egui::Slider::new(&mut priorities.free_block, -5.0..=5.0));
                ui.label("Days Off");
                ui.add(egui::Slider::new(&mut priorities.free_day, -5.0..=5.0));
                ui.label("Day Length");
                ui.add(egui::Slider::new(&mut priorities.day_length, -5.0..=5.0));
            });
            if ui.button("Generate schedules").clicked() {
                self.generate_schedules();
            }
            ui.label(format!("{} solutions found", self.sorted_schedules.len()));
            for scored_schedule in &self.sorted_schedules {
                if ui.link(format!("Schedule: {:.2}", scored_schedule.0.0)).clicked() {
                    self.displayed_schedules.push(DisplayedSchedule(scored_schedule.to_owned()));
                }
            }
        });

        if let Some(mut window) = self.create_class_window.take() {
            egui::Window::new("Create class").show(ctx, |ui| {
                if ui.button("Crn/Subject").clicked() {
                    window.1 = String::new();

                    match &window.0 {
                        Include::Class { .. } => {
                            window.0 = Include::Course {
                                subject: "".to_string(),
                                course_type: None
                            }
                        }
                        Include::Course { .. } => {
                            window.0 = Include::Class {
                                crn: 0
                            }
                        }
                    }
                }

                match &mut window.0 {
                    Include::Class { crn } => {
                        ui.label("Crn: ");
                        ui.text_edit_singleline(&mut window.1);
                        if let Ok(new_crn) = window.1.parse::<Crn>() {
                            *crn = new_crn;
                        } else {
                            ui.label(RichText::new(format!("Could not parse `{}`", window.1)).color(Rgba::RED));
                        }
                    }
                    Include::Course { subject, course_type } => {
                        ui.label("Subject: ");
                        ui.text_edit_singleline(subject);
                        *subject = subject.to_uppercase()
                        // TODO course_type
                    }
                }

                if ui.button("Add").clicked() {
                    self.persistent.includes.push(window.0);
                } else {
                    self.create_class_window = Some(window);
                }
            });
        }

        if let Some(mut window) = self.create_constraint_window.take() {
            egui::Window::new("Create class").show(ctx, |ui| {
                if ui.radio(matches!(window.0, Constraint::Campus { .. }), "Campus").clicked() {
                    window.0 = Constraint::Campus {
                        name: "".to_string()
                    };
                    window.1 = String::new();
                    window.2 = String::new();
                }
                if ui.radio(matches!(window.0, Constraint::StartAfter { .. }), "Start After").clicked() {
                    window.0 = Constraint::StartAfter {
                        time: Time::new(0, 0),
                        days: Days::everyday()
                    };
                    window.1 = String::new();
                    window.2 = String::new();
                }
                if ui.radio(matches!(window.0, Constraint::EndBefore { .. }), "End Before").clicked() {
                    window.0 = Constraint::EndBefore {
                        time: Time::new(0, 0),
                        days: Days::everyday()
                    };
                    window.1 = String::new();
                    window.2 = String::new();
                }
                if ui.radio(matches!(window.0, Constraint::BlockDays { .. }), "Block Days").clicked() {
                    window.0 = Constraint::BlockDays {
                        days: Days::never()
                    };
                    window.1 = String::new();
                    window.2 = String::new();
                }
                if ui.radio(matches!(window.0, Constraint::BlockTimes { .. }), "Block Times").clicked() {
                    window.0 = Constraint::BlockTimes {
                        start: Time::new(0, 0),
                        end: Time::new(0, 0),
                        days: Days::everyday()
                    };
                    window.1 = String::new();
                    window.2 = String::new();
                }

                match &mut window.0 {
                    Constraint::BlockTimes { start, end, days } => {
                        ui.label("Block From: ");
                        time_selector(ui, start, &mut window.1);
                        ui.label("Block Until: ");
                        time_selector(ui, end, &mut window.2);
                        ui.label("Days: ");
                        day_selector(ui, days);
                    }
                    Constraint::BlockDays { days } => {
                        ui.label("Block Days: ");
                        day_selector(ui, days);
                    }
                    Constraint::StartAfter { time, days } => {
                        ui.label("Start After: ");
                        time_selector(ui, time, &mut window.2);
                        ui.label("Days: ");
                        day_selector(ui, days);
                    }
                    Constraint::EndBefore { time, days } => {
                        ui.label("End Before: ");
                        time_selector(ui, time, &mut window.2);
                        ui.label("Days: ");
                        day_selector(ui, days);
                    }
                    Constraint::Campus { name } => {
                        ui.label("Campus: ");
                        ui.text_edit_singleline(name);
                    }
                }

                if ui.button("Add").clicked() {
                    self.persistent.constraints.push(window.0);
                } else {
                    self.create_constraint_window = Some(window);
                }
            });
        }
        self.displayed_schedules.dedup_by_key(|it| it.0.1.clone()); // Grr
        for DisplayedSchedule(((score, breakdown), schedule)) in &self.displayed_schedules {
            egui::Window::new("Schedule").id(Id::new(schedule)).show(ctx, |ui| {
                ui.label(format!("Score: {:.2}, Breakdown: {:?}", score, breakdown));

                let (res, painter) = ui.allocate_painter(Vec2::new(300.0, 200.0), Sense::hover());
                painter.rect(painter.clip_rect(), Rounding::default(), Rgba::BLUE, Stroke::new(5.0, Rgba::RED));
            });
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.persistent);
    }
}

fn day_selector(ui: &mut Ui, days: &mut Days) {
    ui.checkbox(&mut days.sunday, "Sunday");
    ui.checkbox(&mut days.monday, "Monday");
    ui.checkbox(&mut days.tuesday, "Tuesday");
    ui.checkbox(&mut days.wednesday, "Wednesday");
    ui.checkbox(&mut days.thursday, "Thursday");
    ui.checkbox(&mut days.friday, "Friday");
    ui.checkbox(&mut days.saturday, "Saturday");
}

fn time_selector(ui: &mut Ui, time: &mut Time, buffer: &mut String) {
    if buffer.is_empty() {
        buffer.push_str("00:00");
    }

    ui.text_edit_singleline(buffer);

    match buffer.parse() {
        Ok(time_new) => {
          *time = time_new;
        },
        Err(error) => {
            ui.label(RichText::new(format!("{}", error)).color(Rgba::RED));
        },
    }
}
