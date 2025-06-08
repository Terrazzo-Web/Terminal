use std::sync::Arc;

use crate::state::make_state;

make_state!(base_path, Arc<str>);
make_state!(file_path, Arc<str>);
