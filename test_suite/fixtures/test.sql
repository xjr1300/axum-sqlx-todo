INSERT INTO
    users (
        id,
        family_name,
        given_name,
        email,
        hashed_password,
        role_code,
        active,
        last_login_at,
        created_at,
        updated_at
    )
VALUES
    -- password: Adminst0r@tor
    (
        '3c369de2-a382-4d8a-aef9-bc8cb3ecd211',
        'システム',
        '管理者',
        'admin@example.com',
        '$argon2id$v=19$m=12288,t=3,p=1$AYZfaw8GI5rNOTIY2P7qVw$dYNKx8I5Oow+A789NJJAN+p3M4EMaIZvfRJqT2mzIvM',
        1,
        TRUE,
        NULL,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    ),
    -- password: ab12AB#$
    (
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '山田',
        '太郎',
        'taro@example.com',
        '$argon2id$v=19$m=12288,t=3,p=1$kjiB0W7JNYG0rThbIKAntA$LVVIH23hay0J3FT8IEpCVPSk3v0ZBpUY5cikThJ5aSE',
        2,
        TRUE,
        NULL,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    ),
    -- password: aaBB33@@
    (
        'dcae7076-8c5a-4d4c-8894-bcaca68131c6',
        '佐藤',
        '花子',
        'hanako@example.com',
        '$argon2id$v=19$m=12288,t=3,p=1$9p8xMyCJoYFu2AoqL1POaw$s1wF5X5z/hMNMVUmZVN8R268pPKNRjCPiBXIqUvtfvE',
        2,
        TRUE,
        NULL,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    ),
    -- password: p@ssw0rD
    (
        '431389f9-6ee7-44f1-b4b5-a420ee78d005',
        '鈴木',
        '次郎',
        'jiro@example.com',
        '$argon2id$v=19$m=12288,t=3,p=1$mMi/k4pgCICmFMAwhB9Dww$zI9tWKfCgXNp6QGVi9JkraRzy+aN7a/bC8NK5q0+b+Q',
        2,
        FALSE,
        NULL,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    );

INSERT INTO
    todos (
        id,
        user_id,
        title,
        description,
        todo_status_code,
        due_date,
        completed_at,
        archived,
        created_at,
        updated_at
    )
VALUES
    -- 山田太郎、未着手、未アーカイブ
    (
        'ee0f5a08-87c3-48d9-81b0-3f3e7bd8c175',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        'レポート提出',
        '月次レポートを作成して提出',
        1,
        '2025-06-12',
        NULL,
        FALSE,
        '2025-06-08 06:30:00+09',
        '2025-06-08 07:00:00+09'
    ),
    -- 山田太郎、進行中、未アーカイブ
    (
        '4da95cdb-6898-4739-b2be-62ceaa174baf',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        'チームミーティング',
        'プロジェクトの進捗確認',
        2,
        '2025-06-12',
        NULL,
        FALSE,
        '2025-06-03 09:30:00+09',
        '2025-06-10 14:00:00+09'
    ),
    -- 山田太郎、完了済み、未アーカイブ
    (
        'a0c1b2d3-4e5f-6789-abcd-ef0123456789',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '会議資料の準備',
        '次回の会議用資料を作成',
        3,
        '2025-06-14',
        '2025-06-05 15:00:00+09',
        FALSE,
        '2025-06-01 08:00:00+09',
        '2025-06-05 15:00:00+09'
    ),
    -- 山田太郎、キャンセル、未アーカイブ
    (
        'b1c2d3e4-5f6a-7890-abcd-ef0123456789',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '健康診断の予約',
        '来週の健康診断を予約する',
        4,
        '2025-06-15',
        NULL,
        FALSE,
        '2025-05-20 08:30:00+09',
        '2025-06-01 10:00:00+09'
    ),
    -- 山田太郎、保留、未アーカイブ
    (
        'a61301fa-bb2a-490b-84aa-7dae6c4e086a',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '書籍を返却',
        '図書館の本を返す',
        5,
        '2025-06-15',
        NULL,
        FALSE,
        '2025-06-03 09:30:00+09',
        '2025-06-04 16:30:00+09'
    ),
    -- 山田太郎、進行中、未アーカイブ
    (
        'fefdc219-085b-496b-bbf6-72dc40814ab7',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '本の購入',
        '書籍を購入する',
        2,
        '2025-06-16',
        NULL,
        FALSE,
        '2025-06-13 12:00:00+09',
        '2025-06-13 12:00:00+09'
    ),
    -- 山田太郎、未着手、未アーカイブ
    (
        '136327d4-62be-4f7c-ba72-0bf5a019d36c',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        'プレゼンテーションの準備',
        '次回のプレゼン用資料を作成',
        1,
        '2025-06-17',
        NULL,
        FALSE,
        '2025-06-15 10:00:00+09',
        '2025-06-15 10:00:00+09'
    ),
    -- 山田太郎、完了済み、未アーカイブ
    (
        'd4e5f6a7-8901-2345-abcd-ef0123456789',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        'メールの返信',
        '重要なメールに返信する',
        3,
        '2025-06-18',
        '2025-06-08 11:00:00+09',
        FALSE,
        '2025-06-01 08:00:00+09',
        '2025-06-08 11:00:00+09'
    ),
    -- 山田太郎、未着手、未アーカイブ
    (
        'e5f6a7b8-9012-3456-abcd-ef0123456789',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        'プロジェクトの計画',
        '新しいプロジェクトの計画を立てる',
        1,
        '2025-06-19',
        NULL,
        FALSE,
        '2025-06-19 10:00:00+09',
        '2025-06-19 10:00:00+09'
    ),
    -- 山田太郎、未着手、未アーカイブ
    (
        '91c6d97f-5ef8-4776-be93-03a2738759dd',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '車の洗車',
        '車を洗車する',
        1,
        '2025-06-19',
        NULL,
        FALSE,
        '2025-06-18 10:00:00+09',
        '2025-06-18 12:00:00+09'
    ),
    -- 山田太郎、キャンセル、アーカイブ済
    (
        '94904cc3-fff5-44c5-a290-0a6cd54902cd',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '旅行の準備',
        '必要な荷物をまとめる',
        4,
        NULL,
        NULL,
        TRUE,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    ),
    -- 山田太郎、未着手、未アーカイブ
    (
        'b2c3d4e5-6789-0123-abcd-ef0123456789',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '新しいプロジェクトの提案',
        'クライアントに新しいプロジェクトを提案する',
        1,
        NULL,
        NULL,
        FALSE,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    ),
    -- 山田太郎、未着手、未アーカイブ
    (
        'c3d4e5f6-7890-1234-abcd-ef0123456789',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        '健康診断の結果確認',
        '健康診断の結果を確認する',
        1,
        NULL,
        NULL,
        FALSE,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    ),
    -- 山田太郎、完了済み、アーカイブ済み
    (
        '6459a7ba-5b05-412d-8a39-64a7740f4b7a',
        '47125c09-1dea-42b2-a14e-357e59acf3dc',
        'プロジェクトのレビュー',
        'プロジェクトの進捗をレビューする',
        3,
        '2025-06-20',
        '2025-06-15 10:00:00+09',
        TRUE,
        '2025-06-01 08:00:00+09',
        '2025-06-15 10:00:00+09'
    ),
    -- 佐藤花子
    (
        '653acf81-a2e6-43cb-b4b4-9cdb822c740e',
        'dcae7076-8c5a-4d4c-8894-bcaca68131c6',
        '掃除する',
        '部屋を掃除する',
        5,
        '2025-06-11',
        NULL,
        FALSE,
        '2025-05-29 10:00:00+09',
        '2025-05-29 10:00:00+09'
    ),
    -- 佐藤花子
    (
        '8d2555a7-2751-4d35-91e2-5de94df379c1',
        'dcae7076-8c5a-4d4c-8894-bcaca68131c6',
        '企画書作成',
        '来期の企画書を作る',
        2,
        '2025-06-13',
        NULL,
        FALSE,
        '2025-06-01 08:00:00+09',
        '2025-06-01 08:00:00+09'
    ),
    -- 佐藤花子
    (
        '527aef27-2fb8-4bb1-8697-eb12a5649029',
        'dcae7076-8c5a-4d4c-8894-bcaca68131c6',
        '英語の勉強',
        '毎日30分勉強する',
        2,
        '2025-06-30',
        NULL,
        FALSE,
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    ),
    -- 佐藤花子、完了済み
    (
        '7e4c5d0e-3213-4063-abfc-ba833add774b',
        'dcae7076-8c5a-4d4c-8894-bcaca68131c6',
        '買い物リスト作成',
        '週末の買い物リストを作成する',
        3,
        '2025-06-20',
        '2025-06-16 14:00:00+09',
        FALSE,
        '2025-06-15 09:00:00+09',
        '2025-06-16 14:00:00+09'
    ),
    -- 鈴木次郎（ロック中）
    (
        'e74b9ebc-4c81-46dc-9823-e2d743346cb8',
        '431389f9-6ee7-44f1-b4b5-a420ee78d005',
        'フィードバック対応',
        'レビューの修正対応',
        3,
        '2025-06-09',
        '2025-06-09 10:00:00+09',
        FALSE,
        '2025-05-29 15:00:00+09',
        '2025-06-09 10:00:00+09'
    ),
    -- 鈴木次郎（ロック中）
    (
        'e78d38b6-5d62-4793-aeb6-b10b7d146a0b',
        '431389f9-6ee7-44f1-b4b5-a420ee78d005',
        '歯医者の予約',
        '定期検診の予約を入れる',
        1,
        '2025-06-18',
        NULL,
        FALSE,
        '2025-06-17 09:00:00+09',
        '2025-06-17 09:00:00+09'
    );
