package dev.nuagemagique.pilipos.feature.settings.courses

import uniffi.pos_ffi.Category
import uniffi.pos_ffi.CategoryCourse
import uniffi.pos_ffi.CourseSettings

/** The highest course: 3 "Desserts". */
private const val MAX_COURSE = 3

/** The courses a category can take: 0 "Direct" (the default) to 3 "Desserts". */
internal val COURSE_CHOICES: List<Int> = (0..MAX_COURSE).toList()

/** Everything the manager can do in the courses panel. */
internal data class CoursesCallbacks(
    val onToggle: () -> Unit = {},
    val onCourse: (categoryId: Long, course: Int) -> Unit = { _, _ -> },
    val onSave: () -> Unit = {},
)

/**
 * What the courses panel shows (`CoursesSettings.vue`). [stored] is what the engine holds (null until read);
 * [enabled] and [mapping] are the draft the manager edits (category id to course, an unmapped category is 0).
 */
internal data class CoursesUiState(
    val manager: Boolean = false,
    val categories: List<Category> = emptyList(),
    val stored: CourseSettings? = null,
    val enabled: Boolean = false,
    val mapping: Map<Long, Int> = emptyMap(),
    val saved: Boolean = false,
    val busy: Boolean = false,
    val error: String? = null,
) {
    /** The draft as the engine takes it: only the categories with a course above 0, in category order. */
    fun draft(): CourseSettings =
        CourseSettings(
            enabled = enabled,
            categoryCourses =
                categories
                    .map { CategoryCourse(it.id, mapping[it.id] ?: 0) }
                    .filter { it.course > 0 },
        )

    /** The draft differs from what the engine holds. */
    val dirty: Boolean get() = stored != null && draft() != stored
}

/** The mapping of [this]: category id to course. */
internal fun CourseSettings.toMapping(): Map<Long, Int> = categoryCourses.associate { it.categoryId to it.course }
