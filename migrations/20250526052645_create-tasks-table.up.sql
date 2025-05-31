CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- table: users
CREATE TABLE IF NOT EXISTS users (
    id UUID NOT NULL DEFAULT uuid_generate_v4(),
    family_name VARCHAR(100) NOT NULL,
    given_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL,
    hashed_password VARCHAR(255) NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    last_login_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_users PRIMARY KEY (id)
);

CREATE UNIQUE index if NOT EXISTS idx_users_email ON users (email);

-- table: login_failure_histories
CREATE TABLE IF NOT EXISTS login_failure_histories (
    user_id UUID NOT NULL,
    number_of_attempts INTEGER NOT NULL,
    first_attempted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_login_failure_histories PRIMARY KEY (user_id),
    CONSTRAINT fk_login_failure_histories_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

-- table: todo_statuses
CREATE TABLE IF NOT EXISTS todo_statuses (
    code INTEGER NOT NULL,
    name VARCHAR(50) NOT NULL,
    CONSTRAINT pk_todo_statuses PRIMARY KEY (code)
);

INSERT INTO todo_statuses (code, name) VALUES
    (1, '未着手'),
    (2, '進行中'),
    (3, '完了'),
    (4, '中止'),
    (5, '保留');

-- table: todos
CREATE TABLE IF NOT EXISTS todos (
    id uuid NOT NULL DEFAULT uuid_generate_v4(),
    user_id uuid NOT NULL,
    title VARCHAR(100) NOT NULL,
    description VARCHAR(400),
    todo_status_code INTEGER NOT NULL DEFAULT 1,
    completed_at TIMESTAMP WITH TIME ZONE,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT pk_todos PRIMARY KEY (id),
    CONSTRAINT fk_todos_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_todos_todo_status FOREIGN KEY (todo_status_code) REFERENCES todo_statuses (code) ON DELETE RESTRICT
);
