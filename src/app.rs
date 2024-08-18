// Furtherance - Track your time without being tracked
// Copyright (C) 2024  Ricky Kresslein <rk@unobserved.io>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::BTreeMap;

use crate::models::task_to_edit::TaskToEdit;
use crate::style;
use crate::{
    database::*,
    models::{fur_settings::FurSettings, fur_task::FurTask, fur_task_group::FurTaskGroup},
};
use chrono::Duration;
use chrono::{offset::LocalResult, DateTime, Datelike, Local, NaiveDate, NaiveTime};
use iced::widget::Row;
use iced::Color;
use iced::{
    alignment, font, keyboard,
    multi_window::Application,
    widget::{
        button, column, horizontal_space, pick_list, row, text, text_input, theme, vertical_space,
        Button, Column, Container, Scrollable,
    },
    window, Alignment, Command, Element, Font, Length, Renderer, Settings, Size, Theme,
};
use iced_aw::{
    core::icons::{bootstrap, BOOTSTRAP_FONT_BYTES},
    date_picker::{self, Date},
    modal,
    time_picker::{self, Period},
    Card, Modal, TimePicker,
};
use regex::Regex;
use tokio::time;

#[derive(Debug, Clone, PartialEq)]
pub enum FurView {
    Shortcuts,
    Timer,
    History,
    Report,
    Settings,
}

#[derive(Debug)]
pub enum FurAlert {
    TaskNameEmpty,
}

#[derive(Debug)]
pub enum FurInspectorView {
    EditTask,
    EditGroup,
}

#[derive(Debug, Clone)]
pub enum EditTextProperty {
    Name,
    Tags,
    Project,
    Rate,
}

pub struct Furtherance {
    current_view: FurView,
    displayed_alert: Option<FurAlert>,
    displayed_task_start_time: time_picker::Time,
    group_to_edit: Option<(FurTaskGroup, FurTaskGroup)>,
    inspector_view: Option<FurInspectorView>,
    show_timer_start_picker: bool,
    task_history: BTreeMap<chrono::NaiveDate, Vec<FurTaskGroup>>,
    task_input: String,
    timer_is_running: bool,
    timer_start_time: DateTime<Local>,
    timer_stop_time: DateTime<Local>,
    timer_text: String,
    task_to_edit: Option<TaskToEdit>,
}

#[derive(Debug, Clone)]
pub enum Message {
    AlertClose,
    EditGroup(FurTaskGroup),
    EditTaskTextChanged(String, EditTextProperty),
    FontLoaded(Result<(), font::Error>),
    CancelCurrentTaskStartTime,
    ChooseCurrentTaskStartTime,
    NavigateTo(FurView),
    RepeatLastTaskPressed(String),
    StartStopPressed,
    StopwatchTick,
    SubmitCurrentTaskStartTime(time_picker::Time),
    TaskInputChanged(String),
}

impl Application for Furtherance {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Flags = ();
    type Theme = Theme;

    fn new(_: Self::Flags) -> (Self, Command<Self::Message>) {
        // Load or create database
        let _ = db_init();
        // Update old furtherance databases with new properties
        let _ = db_upgrade_old();

        let furtherance = Furtherance {
            current_view: FurView::Timer,
            displayed_alert: None,
            displayed_task_start_time: time_picker::Time::now_hm(true),
            group_to_edit: None,
            inspector_view: None,
            show_timer_start_picker: false,
            task_history: get_task_history(),
            task_input: "".to_string(),
            timer_is_running: false,
            timer_start_time: Local::now(),
            timer_stop_time: Local::now(),
            timer_text: "0:00:00".to_string(),
            task_to_edit: None,
        };

        (
            furtherance,
            font::load(BOOTSTRAP_FONT_BYTES).map(Message::FontLoaded),
        )
    }

    fn title(&self, _window_id: window::Id) -> String {
        "Furtherance".to_owned()
    }

    fn theme(&self, _window_id: window::Id) -> Theme {
        match dark_light::detect() {
            dark_light::Mode::Light | dark_light::Mode::Default => Theme::Light,
            dark_light::Mode::Dark => Theme::Dark,
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::AlertClose => {
                self.displayed_alert = None;
                Command::none()
            }
            Message::EditGroup(task_group) => {
                self.group_to_edit = Some((task_group.clone(), task_group));
                self.inspector_view = Some(FurInspectorView::EditGroup);
                Command::none()
            }
            Message::EditTaskTextChanged(new_value, property) => {
                // TODO: if new_val does not include @ or $ since none of them can
                match self.inspector_view {
                    Some(FurInspectorView::EditTask) => {
                        if let Some(task_to_edit) = self.task_to_edit.as_mut() {
                            match property {
                                EditTextProperty::Name => task_to_edit.new_name = new_value, // TODO: Cannot include #, @, $
                                EditTextProperty::Project => task_to_edit.new_project = new_value, // TODO: Cannot include #, @, $
                                EditTextProperty::Tags => task_to_edit.new_tags = new_value, // TODO: Make sure first char is #. Cannot include @/$
                                EditTextProperty::Rate => {
                                    if new_value.is_empty() {
                                        task_to_edit.new_rate = String::new();
                                    } else if new_value.parse::<f32>().is_ok() {
                                        task_to_edit.new_rate = new_value;
                                    }
                                }
                            }
                        }
                    }
                    Some(FurInspectorView::EditGroup) => {
                        // TODO: CHange to group
                        if let Some(task_to_edit) = self.task_to_edit.as_mut() {
                            match property {
                                EditTextProperty::Name => task_to_edit.new_name = new_value, // TODO: Cannot include #, @, $
                                EditTextProperty::Project => task_to_edit.new_project = new_value, // TODO: Cannot include #, @, $
                                EditTextProperty::Tags => task_to_edit.new_tags = new_value, // TODO: Make sure first char is #. Cannot include @/$
                                EditTextProperty::Rate => {
                                    if new_value.is_empty() {
                                        task_to_edit.new_rate = String::new();
                                    } else if new_value.parse::<f32>().is_ok() {
                                        task_to_edit.new_rate = new_value;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                Command::none()
            }
            Message::CancelCurrentTaskStartTime => {
                self.show_timer_start_picker = false;
                Command::none()
            }
            Message::ChooseCurrentTaskStartTime => {
                self.show_timer_start_picker = true;
                Command::none()
            }
            Message::FontLoaded(_) => Command::none(),
            Message::NavigateTo(destination) => {
                if self.current_view != destination {
                    self.inspector_view = None;
                    self.current_view = destination;
                }
                Command::none()
            }
            Message::RepeatLastTaskPressed(last_task_input) => {
                self.task_input = last_task_input;
                self.current_view = FurView::Timer;
                Command::perform(async { Message::StartStopPressed }, |msg| msg)
            }
            Message::StartStopPressed => {
                if self.timer_is_running {
                    // Stop & reset timer
                    self.timer_stop_time = Local::now();
                    self.timer_is_running = false;

                    let (name, project, tags, rate) = split_task_input(&self.task_input);
                    db_write_task(FurTask {
                        id: 1, // Not used
                        name,
                        start_time: self.timer_start_time,
                        stop_time: self.timer_stop_time,
                        tags,
                        project,
                        rate,
                    })
                    .expect("Couldn't write task to database.");

                    self.task_input = "".to_string();
                    self.task_history = get_task_history();
                    self.timer_text = "0:00:00".to_string();
                    Command::none()
                } else {
                    // TODO: This should not be necessary - logic is in task_input text input
                    let (name, _, _, _) = split_task_input(&self.task_input);
                    if name.is_empty() {
                        self.displayed_alert = Some(FurAlert::TaskNameEmpty);
                        Command::none()
                    } else {
                        self.timer_start_time = Local::now();
                        self.timer_is_running = true;
                        Command::perform(get_timer_duration(), |_| Message::StopwatchTick)
                    }
                }
            }
            Message::StopwatchTick => {
                if self.timer_is_running {
                    let duration = Local::now().signed_duration_since(self.timer_start_time);
                    let hours = duration.num_hours();
                    let minutes = duration.num_minutes() % 60;
                    let seconds = duration.num_seconds() % 60;
                    self.timer_text = format!("{}:{:02}:{:02}", hours, minutes, seconds);

                    Command::perform(get_timer_duration(), |_| Message::StopwatchTick)
                } else {
                    Command::none()
                }
            }
            Message::SubmitCurrentTaskStartTime(new_time) => {
                match convert_iced_time_to_chrono_local(new_time) {
                    LocalResult::Single(local_time) => {
                        if local_time <= Local::now() {
                            self.displayed_task_start_time = new_time;
                            self.timer_start_time = local_time;
                            self.show_timer_start_picker = false;
                        }
                    }
                    _ => {
                        self.show_timer_start_picker = false;
                        eprintln!("Error converting chosen time to local time.");
                    }
                }
                Command::none()
            }
            Message::TaskInputChanged(new_value) => {
                // Handle all possible task input checks here rather than on start/stop press
                let new_value_trimmed = new_value.trim_start();
                // Doesn't start with @
                if new_value_trimmed.chars().next() != Some('@')
                    // Doesn't start with #
                    && new_value_trimmed.chars().next() != Some('#')
                    // Doesn't start with $
                    && new_value_trimmed.chars().next() != Some('$')
                    // No more than 1 @
                    && new_value_trimmed.chars().filter(|&c| c == '@').count() < 2
                    // No more than 1 $
                    && new_value_trimmed.chars().filter(|&c| c == '$').count() < 2
                {
                    // Check if there is a $ and the subsequent part is a parseable f32
                    if let Some(dollar_index) = new_value_trimmed.find('$') {
                        let after_dollar = &new_value_trimmed[dollar_index + 1..];
                        if after_dollar.is_empty() {
                            // Allow typing the $ in the first place
                            self.task_input = new_value_trimmed.to_string();
                        } else {
                            // Find the parseable number right after the $
                            let end_index = after_dollar.find(' ').unwrap_or(after_dollar.len());
                            let number_str = &after_dollar[..end_index];

                            if number_str.parse::<f32>().is_ok() {
                                let remaining_str = &after_dollar[end_index..].trim_start();
                                if remaining_str.is_empty() {
                                    // Allow a number to be typed after the $
                                    self.task_input = new_value_trimmed.to_string();
                                } else {
                                    // Only allow a space, @, or # to be typed after the $ amount
                                    if remaining_str.starts_with('@')
                                        || remaining_str.starts_with('#')
                                    {
                                        self.task_input = new_value_trimmed.to_string();
                                    }
                                }
                            }
                        }
                    } else {
                        // If there is no $, no other checks are necessary
                        self.task_input = new_value_trimmed.to_string();
                    }
                }
                Command::none()
            }
        }
    }

    fn view(&self, _window_id: window::Id) -> Element<Message> {
        // MARK: SIDEBAR
        let sidebar = Container::new(
            column![
                nav_button("Shortcuts", FurView::Shortcuts),
                nav_button("Timer", FurView::Timer),
                nav_button("History", FurView::History),
                nav_button("Report", FurView::Report),
                vertical_space().height(Length::Fill),
                // TODO: if timer is running and nav is not timer, show timer
                nav_button("Settings", FurView::Settings)
            ]
            .spacing(12)
            .padding(20)
            .width(175)
            .align_items(Alignment::Start),
        )
        .style(style::gray_background);

        // MARK: Shortcuts
        let shortcuts_view = column![Scrollable::new(column![])];

        // MARK: TIMER
        let timer_view = column![
            row![
                button(bootstrap::icon_to_text(bootstrap::Bootstrap::ArrowRepeat))
                    .on_press_maybe(get_last_task_input(&self))
                    .style(theme::Button::Text),
                horizontal_space().width(Length::Fill),
                text(format!("Recorded today: {}", get_todays_total_time(&self)))
            ],
            vertical_space().height(Length::Fill),
            text(&self.timer_text).size(80),
            column![
                row![
                    text_input("Task name @Project #tags $rate", &self.task_input)
                        .on_input(Message::TaskInputChanged)
                        .size(20),
                    button(row![
                        horizontal_space().width(Length::Fixed(5.0)),
                        if self.timer_is_running {
                            bootstrap::icon_to_text(bootstrap::Bootstrap::StopFill).size(20)
                        } else {
                            bootstrap::icon_to_text(bootstrap::Bootstrap::PlayFill).size(20)
                        },
                        horizontal_space().width(Length::Fixed(5.0)),
                    ])
                    .on_press(Message::StartStopPressed)
                ]
                .spacing(10),
                if self.timer_is_running {
                    row![TimePicker::new(
                        self.show_timer_start_picker,
                        self.displayed_task_start_time,
                        Button::new(text(format!(
                            "Started at {}",
                            self.displayed_task_start_time.to_string()
                        )))
                        .on_press(Message::ChooseCurrentTaskStartTime),
                        Message::CancelCurrentTaskStartTime,
                        Message::SubmitCurrentTaskStartTime,
                    )
                    .use_24h(),]
                    .align_items(Alignment::Center)
                    .spacing(10)
                } else {
                    row![button("").style(theme::Button::Text)] // Button to match height
                },
            ]
            .align_items(Alignment::Center)
            .spacing(15),
            vertical_space().height(Length::Fill),
        ]
        .align_items(Alignment::Center)
        .padding(20);

        // MARK: HISTORY
        let mut all_history_rows: Column<'_, Message, Theme, Renderer> =
            Column::new().spacing(8).padding(20);
        for (date, task_groups) in self.task_history.iter().rev() {
            let total_time = task_groups
                .iter()
                .map(|group| group.total_time)
                .sum::<i64>();
            all_history_rows = all_history_rows.push(history_title_row(date, total_time));
            for task_group in task_groups {
                all_history_rows = all_history_rows.push(history_group_row(task_group))
            }
        }
        let history_view = column![Scrollable::new(all_history_rows)
            .width(Length::FillPortion(3)) // TODO: Adjust?
            .height(Length::Fill)];

        // MARK: REPORT
        let report_view = column![Scrollable::new(column![])];

        // MARK: SETTINGS
        let settings_view = column![Scrollable::new(column![])];

        // MARK: INSPECTOR
        let inspector: Container<'_, Message, Theme, Renderer> =
            Container::new(match &self.inspector_view {
                Some(FurInspectorView::EditTask) => match &self.task_to_edit {
                    Some(task_to_edit) => column![
                        text_input(&task_to_edit.name, &task_to_edit.new_name)
                            .on_input(|s| Message::EditTaskTextChanged(s, EditTextProperty::Name)),
                        text_input(&task_to_edit.project, &task_to_edit.new_project).on_input(
                            |s| Message::EditTaskTextChanged(s, EditTextProperty::Project)
                        ),
                        text_input(&task_to_edit.tags, &task_to_edit.new_tags)
                            .on_input(|s| Message::EditTaskTextChanged(s, EditTextProperty::Tags)),
                        text_input(&task_to_edit.rate.to_string(), &task_to_edit.new_rate)
                            .on_input(|s| {
                                Message::EditTaskTextChanged(s, EditTextProperty::Rate)
                            }),
                    ]
                    .spacing(12)
                    .padding(20)
                    .width(250)
                    .align_items(Alignment::Start),
                    None => column![],
                },
                Some(FurInspectorView::EditGroup) => {
                    match &self.group_to_edit {
                        Some(group_to_edit) => {
                            if group_to_edit.0.tasks.len() == 1 {
                                // Edit a single task
                                column![
                                    text_input(&group_to_edit.0.name, &group_to_edit.1.name)
                                        .on_input(|s| Message::EditTaskTextChanged(
                                            s,
                                            EditTextProperty::Name
                                        )),
                                    text_input(&group_to_edit.0.project, &group_to_edit.1.project)
                                        .on_input(|s| Message::EditTaskTextChanged(
                                            s,
                                            EditTextProperty::Project
                                        )),
                                    text_input(&group_to_edit.0.tags, &group_to_edit.1.tags)
                                        .on_input(|s| Message::EditTaskTextChanged(
                                            s,
                                            EditTextProperty::Tags
                                        )),
                                    text_input(
                                        &group_to_edit.0.rate.to_string(),
                                        &group_to_edit.1.rate.to_string()
                                    )
                                    .on_input(|s| {
                                        Message::EditTaskTextChanged(s, EditTextProperty::Rate)
                                    }),
                                ]
                                .spacing(12)
                                .padding(20)
                                .width(250)
                                .align_items(Alignment::Start)
                            } else {
                                // Edit a task group
                                column![text(&group_to_edit.0.name)]
                                    .spacing(12)
                                    .padding(20)
                                    .width(175)
                                    .align_items(Alignment::Start)
                            }
                        }
                        None => column![text("Nothing selected.")]
                            .spacing(12)
                            .padding(20)
                            .width(175)
                            .align_items(Alignment::Start),
                    }
                }
                None => column![],
                _ => column![text("Empty.")]
                    .spacing(12)
                    .padding(20)
                    .width(175)
                    .align_items(Alignment::Start),
            });

        let content = row![
            sidebar,
            // Main view
            match self.current_view {
                FurView::Shortcuts => shortcuts_view,
                FurView::Timer => timer_view,
                FurView::History => history_view,
                FurView::Report => report_view,
                FurView::Settings => settings_view,
            },
            inspector,
        ];

        let overlay: Option<Card<'_, Message, Theme, Renderer>> = if self.displayed_alert.is_some()
        {
            let alert_text: &str;
            let alert_description: &str;
            let close_button_text: &str;

            match self.displayed_alert.as_ref().unwrap() {
                FurAlert::TaskNameEmpty => {
                    alert_text = "Empty Task Name";
                    alert_description = "The task must have a name.";
                    close_button_text = "OK";
                }
            }

            let buttons: Row<'_, Message, Theme, Renderer> = row![button(
                text(close_button_text)
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .width(Length::Fill)
            )
            .on_press(Message::AlertClose)]
            .spacing(10)
            .padding(5)
            .width(Length::Fill);

            Some(
                Card::new(text(alert_text), text(alert_description))
                    .foot(buttons)
                    .max_width(300.0)
                    .on_close(Message::AlertClose),
            )
        } else {
            None
        };

        modal(content, overlay)
            .backdrop(Message::AlertClose)
            .on_esc(Message::AlertClose)
            .into()
    }
}

fn nav_button<'a>(text: &'a str, destination: FurView) -> Button<'a, Message> {
    button(text)
        .on_press(Message::NavigateTo(destination))
        .style(theme::Button::Text)
}

fn history_group_row<'a>(task_group: &FurTaskGroup) -> Button<'a, Message> {
    let total_time_str = seconds_to_formatted_duration(task_group.total_time);
    let mut task_details_column: Column<'_, Message, Theme, Renderer> =
        column![text(&task_group.name),];
    if !task_group.project.is_empty() {
        task_details_column = task_details_column.push(text(format!("@{}", task_group.project)));
    }
    if !task_group.tags.is_empty() {
        task_details_column = task_details_column.push(text(format!("#{}", task_group.tags)));
    }

    let mut task_row: Row<'_, Message, Theme, Renderer> =
        row![].align_items(Alignment::Center).spacing(5);
    if task_group.tasks.len() > 1 {
        task_row = task_row.push(
            Container::new(text(task_group.tasks.len()))
                .align_x(alignment::Horizontal::Center)
                .width(30)
                .style(style::group_count_circle),
        );
    }
    task_row = task_row.push(task_details_column);
    task_row = task_row.push(horizontal_space().width(Length::Fill));
    task_row = task_row.push(text(total_time_str));
    task_row = task_row.push(
        button(bootstrap::icon_to_text(bootstrap::Bootstrap::ArrowRepeat))
            .on_press(Message::RepeatLastTaskPressed(task_input_builder(
                task_group,
            )))
            .style(theme::Button::Text),
    );

    button(
        Container::new(task_row)
            .padding([10, 15, 10, 15])
            .width(Length::Fill)
            .style(style::task_row),
    )
    .on_press(Message::EditGroup(task_group.clone()))
    .style(theme::Button::Text)
}

// fn get_task_group_with_id(state: &Furtherance) -> Option<&FurTaskGroup> {
//     for value in state.task_history.values() {
//         if let Some(group_to_edit) = value.iter().find(|v| v.id == state.group_id_to_edit) {
//             return Some(group_to_edit);
//         }
//     }
//     None
// }

// fn get_mutable_task_group_with_id(state: &mut Furtherance) -> Option<&mut FurTaskGroup> {
//     for value in map.values_mut() {
//         if value.id == target_id {
//             return Some(value);
//         }
//     }
//     None
// }

fn history_title_row<'a>(date: &NaiveDate, total_time: i64) -> Row<'a, Message> {
    let total_time_str = seconds_to_formatted_duration(total_time);
    row![
        text(format_history_date(date)).font(font::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
        horizontal_space().width(Length::Fill),
        text(total_time_str).font(font::Font {
            weight: iced::font::Weight::Bold,
            ..Default::default()
        }),
    ]
}

fn format_history_date(date: &NaiveDate) -> String {
    let today = Local::now().date_naive();
    let yesterday = today - Duration::days(1);
    let current_year = today.year();

    if date == &today {
        "Today".to_string()
    } else if date == &yesterday {
        "Yesterday".to_string()
    } else if date.year() == current_year {
        date.format("%b %d").to_string()
    } else {
        date.format("%b %d, %Y").to_string()
    }
}

fn get_task_history() -> BTreeMap<chrono::NaiveDate, Vec<FurTaskGroup>> {
    let mut grouped_tasks_by_date: BTreeMap<chrono::NaiveDate, Vec<FurTaskGroup>> = BTreeMap::new();

    //TODO : Change limit based on user limit or max limit. Also should limit by days not items.
    if let Ok(all_tasks) = db_retrieve_all(SortBy::StopTime, SortOrder::Descending) {
        let tasks_by_date = group_tasks_by_date(all_tasks);

        for (date, tasks) in tasks_by_date {
            let mut all_groups: Vec<FurTaskGroup> = vec![];
            for task in tasks {
                if let Some(matching_group) = all_groups.iter_mut().find(|x| x.is_equal_to(&task)) {
                    matching_group.add(task);
                } else {
                    all_groups.push(FurTaskGroup::new_from(task));
                }
            }
            grouped_tasks_by_date.insert(date, all_groups);
        }
    }
    grouped_tasks_by_date
}

fn group_tasks_by_date(tasks: Vec<FurTask>) -> BTreeMap<chrono::NaiveDate, Vec<FurTask>> {
    let mut grouped_tasks: BTreeMap<chrono::NaiveDate, Vec<FurTask>> = BTreeMap::new();

    for task in tasks {
        let date = task.start_time.date_naive(); // Extract the date part
        grouped_tasks
            .entry(date)
            .or_insert_with(Vec::new)
            .push(task);
    }

    grouped_tasks
}

fn convert_iced_time_to_chrono_local(iced_time: time_picker::Time) -> LocalResult<DateTime<Local>> {
    let (hour, minute, _) = match iced_time {
        time_picker::Time::Hm {
            hour,
            minute,
            period,
        } => (hour, minute, period),
        _ => (1, 1, Period::H24),
    };

    if let Some(time) = NaiveTime::from_hms_opt(hour, minute, 0) {
        Local::now().with_time(time)
    } else {
        LocalResult::None
    }
}

fn hex_to_color(hex: &str) -> Color {
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap();

    Color::from_rgb8(r, g, b)
}

async fn get_timer_duration() {
    time::sleep(time::Duration::from_secs(1)).await;
}

pub fn split_task_input(input: &str) -> (String, String, String, f32) {
    let re_name = Regex::new(r"^[^@#$]+").unwrap();
    let re_project = Regex::new(r"@([^#\$]+)").unwrap();
    let re_tags = Regex::new(r"#([^@#$]+)").unwrap();
    let re_rate = Regex::new(r"\$([^@#$]+)").unwrap();

    let name = re_name
        .find(input)
        .map_or("", |m| m.as_str().trim())
        .to_string();

    let project = re_project
        .captures(input)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .unwrap_or(String::new());

    let separated_tags: Vec<String> = re_tags
        .captures_iter(input)
        .map(|cap| cap.get(1).unwrap().as_str().trim().to_string())
        .collect();
    let tags = separated_tags.join(" #");

    let rate_string = re_rate
        .captures(input)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .unwrap_or("0.0".to_string());
    let rate: f32 = rate_string.parse().unwrap_or(0.0);

    (name, project, tags, rate)
}

fn get_last_task_input(state: &Furtherance) -> Option<Message> {
    let today = Local::now().date_naive();
    if let Some(groups) = state.task_history.get(&today) {
        if let Some(last_task) = groups.first() {
            let task_input_builder = task_input_builder(last_task);
            Some(Message::RepeatLastTaskPressed(task_input_builder))
        } else {
            None
        }
    } else {
        None
    }
}

// TODO: Use task.to_string instead
fn task_input_builder(task_group: &FurTaskGroup) -> String {
    let mut task_input_builder = task_group.name.to_string();

    if !task_group.project.is_empty() {
        task_input_builder += &format!(" @{}", task_group.project);
    }

    if !task_group.tags.is_empty() {
        task_input_builder += &format!(" #{}", task_group.tags);
    }

    if task_group.rate != 0.0 {
        task_input_builder += &format!(" ${}", task_group.rate);
    }

    task_input_builder
}

fn get_todays_total_time(state: &Furtherance) -> String {
    let today = Local::now().date_naive();
    let total_seconds: i64 = if let Some(groups) = state.task_history.get(&today) {
        groups.iter().map(|group| group.total_time).sum()
    } else {
        0
    };
    seconds_to_formatted_duration(total_seconds)
}

fn seconds_to_formatted_duration(total_seconds: i64) -> String {
    seconds_to_hms(total_seconds)
    // TODO: If don't show seconds:
    // seconds_to_hm(total_seconds)
}

fn seconds_to_hms(total_seconds: i64) -> String {
    let h = total_seconds / 3600;
    let m = total_seconds % 3600 / 60;
    let s = total_seconds % 60;
    format!("{}:{:02}:{:02}", h, m, s)
}

fn seconds_to_hm(total_seconds: i64) -> String {
    let h = total_seconds / 3600;
    let m = total_seconds % 3600 / 60;
    format!("{}:{:02}", h, m)
}
