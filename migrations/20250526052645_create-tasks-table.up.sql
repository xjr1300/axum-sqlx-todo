CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- table: roles
CREATE TABLE IF NOT EXISTS roles (
    code SMALLINT NOT NULL,
    name VARCHAR(50) NOT NULL,
    description VARCHAR(255),
    display_order SMALLINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_roles PRIMARY KEY (code)
);
-- DO NOT CHANGE RECORDS BELOW
-- If you need to change the records, you must fix role code constants.
INSERT INTO roles (code, name, description, display_order) VALUES
    (1, '管理者', 'システム全体の管理を行う役割', 1),
    (2, 'ユーザー', '通常のユーザーとしての役割', 2);

-- table: users
CREATE TABLE IF NOT EXISTS users (
    id UUID NOT NULL DEFAULT uuid_generate_v4(),
    family_name VARCHAR(100) NOT NULL,
    given_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL,
    hashed_password VARCHAR(255) NOT NULL,
    role_code SMALLINT NOT NULL DEFAULT 2,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    last_login_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_users PRIMARY KEY (id),
    CONSTRAINT fk_users_role FOREIGN KEY (role_code) REFERENCES roles (code) ON DELETE RESTRICT
);

CREATE UNIQUE index if NOT EXISTS idx_users_email ON users (email);

-- table: user_tokens
-- This table is inserted row when user logs in.
-- This table should not be used to verify access/refresh tokens validity.
-- This table aims to delete access/refresh tokens from redis when the user logs out.
-- So, if user logs out, delete the record from this table.
CREATE TABLE IF NOT EXISTS user_tokens (
    id UUID NOT NULL DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    token_key VARCHAR(255) NOT NULL,
    expired_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_user_tokens PRIMARY KEY (id),
    CONSTRAINT fk_user_tokens_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

-- table: login_failed_histories
CREATE TABLE IF NOT EXISTS login_failed_histories (
    user_id UUID NOT NULL,
    attempted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    number_of_attempts INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_login_failed_histories PRIMARY KEY (user_id),
    CONSTRAINT fk_login_failed_histories_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

-- table: todo_statuses
-- If you need to change the records, you must fix todo status constants.
CREATE TABLE IF NOT EXISTS todo_statuses (
    code SMALLINT NOT NULL,
    name VARCHAR(50) NOT NULL,
    description VARCHAR(255),
    display_order SMALLINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_todo_statuses PRIMARY KEY (code)
);
INSERT INTO todo_statuses (code, name, description, display_order) VALUES
    (1, '未着手', 'タスクがまだ開始されていない状態', 1),
    (2, '進行中', 'タスクが現在進行中の状態', 2),
    (3, '完了', 'タスクが完了した状態', 3),
    (4, '中止', 'タスクが中止された状態', 4),
    (5, '保留', 'タスクが一時的に保留されている状態', 5);

-- table: todos
CREATE TABLE IF NOT EXISTS todos (
    id UUID NOT NULL DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    title VARCHAR(100) NOT NULL,
    description VARCHAR(400),
    todo_status_code SMALLINT NOT NULL DEFAULT 1,
    due_date DATE,
    completed_at TIMESTAMP WITH TIME ZONE,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_todos PRIMARY KEY (id),
    CONSTRAINT fk_todos_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_todos_todo_status FOREIGN KEY (todo_status_code) REFERENCES todo_statuses (code) ON DELETE RESTRICT
);
