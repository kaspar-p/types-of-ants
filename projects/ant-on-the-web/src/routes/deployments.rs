use ant_library::routes::Routes;

use crate::state::ApiRoutes;

pub fn routes() -> ApiRoutes {
    Routes::new()
}
