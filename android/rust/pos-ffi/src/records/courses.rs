//! Records of the courses (engine module `courses`): the setting, the held courses and the
//! result of "Envoyer la suite". Mirrors the engine types (PD56): same field names, same serde
//! form, `deny_unknown_fields`.

use super::common::SlipOutcome;

/// `pos_engine::commands::courses::CategoryCourse`: a category and the course its products take.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct CategoryCourse {
    pub category_id: i64,
    /// 1 to 3; an unmapped category is "Direct" (0).
    pub course: i32,
}

/// `pos_engine::commands::courses::CourseSettings`: the courses setting (off by default).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct CourseSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub category_courses: Vec<CategoryCourse>,
}

/// `pos_engine::commands::courses::HeldCourseView`: a course waiting for "Envoyer la suite".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct HeldCourse {
    pub course: i32,
    pub lines: i64,
    pub quantity: i64,
}

/// `pos_engine::commands::courses::NextCourse`: what "Envoyer la suite" did.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct NextCourse {
    /// The course released; `None` when nothing was held and the call printed again what an
    /// earlier slip had not delivered.
    pub course: Option<i32>,
    /// The slip of the P that registered the unsent lines first.
    pub signed: Option<SlipOutcome>,
    /// The release slip.
    pub slip: SlipOutcome,
}
