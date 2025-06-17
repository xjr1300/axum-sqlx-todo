pub mod lookup;
pub mod todo;
pub mod user;

use domain::models::User;

#[derive(Debug, Clone)]
pub struct AuthorizedUser(pub User);
