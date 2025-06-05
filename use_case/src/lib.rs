pub mod user;

use domain::models::User;

#[derive(Debug, Clone)]
pub enum RequestUser {
    AuthUser(User),
    Anonymous,
}
