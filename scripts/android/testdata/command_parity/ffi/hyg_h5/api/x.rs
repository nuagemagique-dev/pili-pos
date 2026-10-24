use std::sync::{LazyLock, OnceCell, OnceLock};

static COUNTER: u8 = 0;
thread_local! { static T: u8 = 0; }
lazy_static! { static ref L: u8 = 0; }
static CELL: OnceLock<i64> = OnceLock::new();
static CELL2: OnceCell<i64> = OnceCell::new();
static CELL3: LazyLock<i64> = LazyLock::new(|| 1);
