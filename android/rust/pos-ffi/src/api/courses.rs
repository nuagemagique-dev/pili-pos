//! The courses facade (engine module courses, v1.2): six commands as async methods of
//! [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/courses.rs`): the same parameter
//! names and the single engine call. Kitchen printing only, never fiscal. Reading, changing a
//! line's course, sending the next course and reprinting are open to any logged-in operator;
//! the setting is for managers.

use pos_engine::commands::courses;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::common::SlipOutcome;
use crate::records::courses::{CourseSettings, HeldCourse, NextCourse};

#[uniffi::export]
impl PosEngine {
    /// The courses setting (switch and category courses).
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn get_course_settings(&self) -> Result<CourseSettings, PosError> {
        let settings = self
            .blocking(|st| courses::do_get_course_settings(st))
            .await?;
        convert::out(&settings)
    }

    /// Store the courses setting; managers only.
    pub async fn set_course_settings(
        &self,
        input: CourseSettings,
    ) -> Result<CourseSettings, PosError> {
        let input: courses::CourseSettings = convert::inp(&input)?;
        let stored = self
            .blocking(move |st| courses::do_set_course_settings(st, input))
            .await?;
        convert::out(&stored)
    }

    /// Change a line's course while no kitchen slip carried it.
    pub async fn set_line_course(&self, line_id: i64, course: i32) -> Result<(), PosError> {
        self.blocking(move |st| courses::do_set_line_course(st, line_id, course))
            .await
    }

    /// The courses of an order that wait for "Envoyer la suite".
    pub async fn list_held_courses(&self, order_id: i64) -> Result<Vec<HeldCourse>, PosError> {
        let held = self
            .blocking(move |st| courses::do_list_held_courses(st, order_id))
            .await?;
        convert::out(&held)
    }

    /// "Envoyer la suite": release and print the next held course.
    pub async fn send_next_course(&self, order_id: i64) -> Result<NextCourse, PosError> {
        let next = self
            .spawn(move |e| async move { courses::do_send_next_course(&e, order_id).await })
            .await?;
        convert::out(&next)
    }

    /// Print a course slip again, marked DUPLICATA.
    pub async fn reprint_course_slip(
        &self,
        order_id: i64,
        course: i32,
    ) -> Result<SlipOutcome, PosError> {
        let outcome = self
            .spawn(
                move |e| async move { courses::do_reprint_course_slip(&e, order_id, course).await },
            )
            .await?;
        convert::out(&outcome)
    }
}
