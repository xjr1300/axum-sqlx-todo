use garde::Validate as _;
use time::{Date, OffsetDateTime};

use crate::models::primitives::{Description, DisplayOrder, Id};
use crate::models::user::User;
use crate::{
    DomainError, DomainErrorKind, DomainResult, impl_int_primitive, impl_string_primitive,
};

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, garde::Validate)]
pub struct TodoStatusCode(#[garde(range(min=1, max=i16::MAX))] pub i16);
impl_int_primitive!(TodoStatusCode, i16);

/// Todo状態名
#[derive(Debug, Clone, garde::Validate)]
pub struct TodoStatusName(#[garde(length(chars, min = 1, max = 50))] pub String);
impl_string_primitive!(TodoStatusName);

/// Todo
///
/// # ドメインルール
///
/// - 作成日時は更新日時と同じか、更新日時よりも前でなくてはならない。
/// - 完了したTodoは更新できない。
///   - したがって、完了時に更新日時を更新するため、完了日時は更新日時と同でなくてはならない。
/// - アーカイブされたTodoは、更新できない。
#[derive(Debug, Clone)]
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
    pub due_date: Option<Date>,
    /// 完了日時
    pub completed_at: Option<OffsetDateTime>,
    /// アーカイブ済み
    pub archived: bool,
    /// 作成日時
    pub created_at: OffsetDateTime,
    /// 更新日時
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

    fn validate(&self) -> DomainResult<()> {
        // 作成日時は更新日時と同じか、更新日時よりも前でなくてはならない。
        if self.created_at > self.updated_at {
            return Err(DomainError {
                kind: DomainErrorKind::Validation,
                messages: vec!["created_at must be less than or equal to updated_at".into()],
                source: anyhow::anyhow!("created_at must be less than or equal to updated_at"),
            });
        }

        // Todoが完了している場合
        if let Some(completed_at) = self.completed_at {
            // 完了日時は作成日時と同じか、作成日時よりも後でなくてはならない。
            if completed_at < self.created_at {
                return Err(DomainError {
                    kind: DomainErrorKind::Validation,
                    messages: vec![
                        "completed_at must be greater than or equal to created_at".into(),
                    ],
                    source: anyhow::anyhow!(
                        "completed_at must be greater than or equal to created_at"
                    ),
                });
            }
            // 完了日時は更新日時と等しくなくてはならない。
            if completed_at != self.updated_at {
                return Err(DomainError {
                    kind: DomainErrorKind::Validation,
                    messages: vec!["completed_at must be equal to updated_at".into()],
                    source: anyhow::anyhow!("completed_at must be equal to updated_at"),
                });
            }
        }

        Ok(())
    }
}

/// Todo状態
#[derive(Debug, Clone)]
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
    pub created_at: OffsetDateTime,
    /// Todo状態の更新日時
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
                code: RoleCode(1),
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
            code: TodoStatusCode(1),
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
    #[case(datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 00:00:00 UTC), None, true)]
    // 作成日時が更新日時と等しい
    #[case(datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 00:00:01 UTC), None, true)]
    // 作成日時が更新日時と等しい
    #[case(datetime!(2025-01-01 00:00:01 UTC), datetime!(2025-01-01 00:00:00 UTC), None, false)]
    // 完了日時が作成日時と更新日時と等しい
    #[case(datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 00:00:00 UTC), Some(datetime!(2025-01-01 00:00:00 UTC)), true)]
    // 完了日時が作成日時よりも後で、更新日時と等しい
    #[case(datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 01:00:00 UTC), Some(datetime!(2025-01-01 01:00:00 UTC)), true)]
    // 完了日時が作成日時よりも後で、更新日時と異なる
    #[case(datetime!(2025-01-01 00:00:00 UTC), datetime!(2025-01-01 01:00:00 UTC), Some(datetime!(2025-01-01 01:00:01 UTC)), false)]
    // 完了日時が作成日時よりも前
    #[case(datetime!(2025-01-01 00:00:01 UTC), datetime!(2025-01-01 00:00:00 UTC), Some(datetime!(2025-01-01 00:00:00 UTC)), false)]
    fn todo_new_date_time_related(
        #[case] created_at: OffsetDateTime,
        #[case] updated_at: OffsetDateTime,
        #[case] completed_at: Option<OffsetDateTime>,
        #[case] expected: bool,
    ) {
        let id = Uuid::new_v4();
        let user = create_user();
        let todo_id = TodoId::from(id);
        let title = TodoTitle::new("Test Title".to_string()).unwrap();
        let description = Some(TodoDescription::new("Test Description".to_string()).unwrap());
        let status = TodoStatus {
            code: TodoStatusCode(1),
            name: TodoStatusName("未着手".to_string()),
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
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}
