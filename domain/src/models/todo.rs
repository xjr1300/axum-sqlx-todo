use garde::Validate as _;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::{Date, OffsetDateTime};

use utils::serde::{
    deserialize_option_date, deserialize_option_offset_datetime, serialize_option_date,
    serialize_option_offset_datetime,
};

use crate::models::primitives::{Description, DisplayOrder, Id};
use crate::models::user::User;
use crate::{DomainError, DomainErrorKind, DomainResult, domain_error, impl_string_primitive};

/// Todo ID
pub type TodoId = Id<Todo>;

// Todoタイトル
#[derive(Debug, Clone, garde::Validate)]
pub struct TodoTitle(#[garde(length(chars, min = 1, max = 100))] pub String);
impl_string_primitive!(TodoTitle);

/// Todo説明
#[derive(Debug, Clone, garde::Validate)]
pub struct TodoDescription(#[garde(length(chars, min = 1, max = 400))] pub String);
impl_string_primitive!(TodoDescription);

/// Todo状態コード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(i16)]
pub enum TodoStatusCode {
    /// 未着手
    NotStarted = 1,
    // 進行中
    InProgress = 2,
    // 完了
    Completed = 3,
    // キャンセル
    Cancelled = 4,
    // 保留
    OnHold = 5,
}

impl TryFrom<i16> for TodoStatusCode {
    type Error = DomainError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(TodoStatusCode::NotStarted),
            2 => Ok(TodoStatusCode::InProgress),
            3 => Ok(TodoStatusCode::Completed),
            4 => Ok(TodoStatusCode::Cancelled),
            5 => Ok(TodoStatusCode::OnHold),
            _ => Err(domain_error(
                DomainErrorKind::Validation,
                "Invalid todo status code",
            )),
        }
    }
}

/// Todo状態名
#[derive(Debug, Clone, garde::Validate)]
pub struct TodoStatusName(#[garde(length(chars, min = 1, max = 50))] pub String);
impl_string_primitive!(TodoStatusName);

/// Todo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    /// ID
    pub id: TodoId,
    /// ユーザー
    pub user: User,
    /// タイトル
    pub title: TodoTitle,
    /// 説明
    pub description: Option<TodoDescription>,
    /// 状態
    pub status: TodoStatus,
    /// 完了予定日
    #[serde(serialize_with = "serialize_option_date")]
    #[serde(deserialize_with = "deserialize_option_date")]
    pub due_date: Option<Date>,
    /// 完了日時
    #[serde(serialize_with = "serialize_option_offset_datetime")]
    #[serde(deserialize_with = "deserialize_option_offset_datetime")]
    pub completed_at: Option<OffsetDateTime>,
    /// アーカイブ済み
    pub archived: bool,
    /// 作成日時
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// 更新日時
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl Todo {
    /// Todoを新規作成する。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TodoId,
        user: User,
        title: TodoTitle,
        description: Option<TodoDescription>,
        status: TodoStatus,
        due_date: Option<Date>,
        completed_at: Option<OffsetDateTime>,
        archived: bool,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> DomainResult<Self> {
        let todo = Self {
            id,
            user,
            title,
            description,
            status,
            due_date,
            completed_at,
            archived,
            created_at,
            updated_at,
        };
        todo.validate()?;
        Ok(todo)
    }

    /// # ドメインルール
    ///
    /// - 作成日時は更新日時と同じか、更新日時よりも前でなくてはならない。
    /// - 完了予定日は、作成日時よりも後でなくてはならない。
    /// - 完了している（完了日時が登録されている）場合、状態が完了でなければならない。
    /// - 完了している（完了日時が登録されている）場合、完了日時は作成日時よりも後でなければならない。
    /// - 完了している（完了日時が登録されている）場合、完了日時と更新日時が一致しなければならない。
    ///   - 完了後は、更新できないため。
    /// - 完了したTodoは更新できない。
    /// - アーカイブされたTodoは、更新できない。
    fn validate(&self) -> DomainResult<()> {
        // 作成日時は更新日時と同じか、更新日時よりも前でなくてはならない。
        if self.created_at > self.updated_at {
            return Err(domain_error(
                DomainErrorKind::Validation,
                "created_at must be less than or equal to updated_at",
            ));
        }

        // 完了予定日が登録されている場合
        if let Some(due_date) = self.due_date {
            // 完了予定日は作成日時よりも後でなくてはならない。
            if due_date < self.created_at.date() {
                return Err(domain_error(
                    DomainErrorKind::Validation,
                    "due_date must be greater than created_at",
                ));
            }
        }

        // 完了している（完了日時地が登録されている）場合
        if let Some(completed_at) = self.completed_at {
            // 状態が完了でなければならない。
            if self.status.code != TodoStatusCode::Completed {
                return Err(domain_error(
                    DomainErrorKind::Validation,
                    "status must be completed when completed_at is set",
                ));
            }
            // 完了日時は作成日時よりも後でなくてはならない。
            if completed_at < self.created_at {
                return Err(domain_error(
                    DomainErrorKind::Validation,
                    "completed_at must be greater than or equal to created_at",
                ));
            }
            // 完了日時と更新日時が一致しなくてはならない。
            if completed_at != self.updated_at {
                return Err(domain_error(
                    DomainErrorKind::Validation,
                    "completed_at must be equal to updated_at",
                ));
            }
        }

        Ok(())
    }
}

/// Todo状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoStatus {
    /// Todo状態コード
    pub code: TodoStatusCode,
    /// Todo状態名
    pub name: TodoStatusName,
    /// Todo状態の説明
    pub description: Option<Description>,
    /// Todo状態の順序
    pub display_order: DisplayOrder,
    /// Todo状態の作成日時
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Todo状態の更新日時
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Role, RoleCode, RoleName,
        user::{Email, FamilyName, GivenName, UserId},
    };
    use time::{Duration, macros::datetime};
    use uuid::Uuid;

    fn create_user() -> User {
        User {
            id: UserId::default(),
            family_name: FamilyName::new(String::from("Doe")).unwrap(),
            given_name: GivenName::new(String::from("John")).unwrap(),
            email: Email::new(String::from("doe@example.com")).unwrap(),
            role: Role {
                code: RoleCode::Admin,
                name: RoleName("管理者".to_string()),
                description: None,
                display_order: DisplayOrder(1),
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            },
            active: true,
            last_login_at: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn todo_new() {
        let now = OffsetDateTime::now_utc();
        let id = Uuid::new_v4();
        let user = create_user();
        let todo_id = TodoId::from(id);
        let title = TodoTitle::new("Test Title".to_string()).unwrap();
        let description = Some(TodoDescription::new("Test Description".to_string()).unwrap());
        let status = TodoStatus {
            code: TodoStatusCode::NotStarted,
            name: TodoStatusName("未着手".to_string()),
            description: None,
            display_order: DisplayOrder(1),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };
        let due_date = Some(now.date() + Duration::days(7));
        let completed_at = None;
        let created_at = now;
        let updated_at = now;

        let todo = Todo::new(
            todo_id,
            user,
            title,
            description,
            status,
            due_date,
            completed_at,
            false,
            created_at,
            updated_at,
        )
        .unwrap();

        assert_eq!(todo.id.0, id);
        assert_eq!(todo.title.0, "Test Title");
        assert_eq!(todo.description.unwrap().0, "Test Description");
        assert_eq!(todo.completed_at, None);
        assert_eq!(todo.created_at, created_at);
        assert_eq!(todo.updated_at, updated_at);
    }

    #[rstest::rstest]
    // 作成日時が更新日時と等しい
    #[case(TodoStatusCode::NotStarted, None, datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 00:00:00 UTC),  true)]
    // 作成日時が更新日時と等しい
    #[case(TodoStatusCode::InProgress, None, datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 00:00:01 UTC),  true)]
    // 作成日時が更新日時と等しい
    #[case(TodoStatusCode::Cancelled, None, datetime!(2025-01-01 00:00:01 UTC), datetime!(2025-01-01 00:00:00 UTC), false)]
    // 完了日時が作成日時と更新日時と等しい
    #[case(TodoStatusCode::Completed, Some(datetime!(2025-01-01 00:00:00 UTC)),datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 00:00:00 UTC), true)]
    // 完了日時が作成日時よりも後で、更新日時と等しい
    #[case(TodoStatusCode::Completed, Some(datetime!(2025-01-01 01:00:00 UTC)), datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 01:00:00 UTC), true)]
    // 完了日時が作成日時よりも後で、更新日時と異なる
    #[case(TodoStatusCode::Completed, Some(datetime!(2025-01-01 01:00:01 UTC)), datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 01:00:00 UTC), false)]
    // 完了日時が作成日時よりも前
    #[case(TodoStatusCode::Completed, Some(datetime!(2025-01-01 00:00:00 UTC)), datetime!(2025-01-01 00:00:01 UTC), datetime!(2025-01-01 00:00:00 UTC), false)]
    fn todo_new_date_time_related(
        #[case] todo_status_code: TodoStatusCode,
        #[case] completed_at: Option<OffsetDateTime>,
        #[case] created_at: OffsetDateTime,
        #[case] updated_at: OffsetDateTime,
        #[case] expected: bool,
    ) {
        let id = Uuid::new_v4();
        let user = create_user();
        let todo_id = TodoId::from(id);
        let title = TodoTitle::new("Test Title".to_string()).unwrap();
        let description = Some(TodoDescription::new("Test Description".to_string()).unwrap());
        let status = TodoStatus {
            code: todo_status_code,
            name: TodoStatusName("any".to_string()),
            description: None,
            display_order: DisplayOrder(1),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };
        let due_date = Some(created_at.date() + Duration::days(7));

        let result = Todo::new(
            todo_id,
            user,
            title,
            description,
            status,
            due_date,
            completed_at,
            false,
            created_at,
            updated_at,
        );
        if expected {
            assert!(result.is_ok(), "{}", result.err().unwrap());
        } else {
            assert!(result.is_err());
        }
    }
}
