use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::courses::{CourseSettings, HeldCourseView, NextCourse};
use tauri::State;

use super::Managed;

/// The courses setting (switch and category courses).
#[tauri::command]
pub fn get_course_settings(state: State<'_, Managed>) -> Result<CourseSettings, String> {
    pos_engine::commands::courses::do_get_course_settings(state.inner())
}

/// Store the courses setting; managers only.
#[tauri::command]
pub fn set_course_settings(
    state: State<'_, Managed>,
    input: CourseSettings,
) -> Result<CourseSettings, String> {
    pos_engine::commands::courses::do_set_course_settings(state.inner(), input)
}

/// Change a line's course while no kitchen slip carried it.
#[tauri::command]
pub fn set_line_course(state: State<'_, Managed>, line_id: i64, course: i32) -> Result<(), String> {
    pos_engine::commands::courses::do_set_line_course(state.inner(), line_id, course)
}

/// The courses of an order that wait for "Envoyer la suite".
#[tauri::command]
pub fn list_held_courses(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<Vec<HeldCourseView>, String> {
    pos_engine::commands::courses::do_list_held_courses(state.inner(), order_id)
}

/// "Envoyer la suite": release and print the next held course.
#[tauri::command]
pub async fn send_next_course(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<NextCourse, String> {
    pos_engine::commands::courses::do_send_next_course(state.inner(), order_id).await
}

/// Print a course slip again, marked DUPLICATA.
#[tauri::command]
pub async fn reprint_course_slip(
    state: State<'_, Managed>,
    order_id: i64,
    course: i32,
) -> Result<SlipOutcome, String> {
    pos_engine::commands::courses::do_reprint_course_slip(state.inner(), order_id, course).await
}
