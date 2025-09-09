CREATE TABLE
    global_api_config (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        api_config jsonb NOT NULL,
        CONSTRAINT global_api_config_pkey PRIMARY KEY (id)
    );

-- 用户账号
CREATE TABLE
    user_account (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        phone text NOT NULL,
        CONSTRAINT user_account_pkey PRIMARY KEY (id)
    );

-- 待输入确认码的用户会话
CREATE TABLE
    user_confirm (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        account uuid NOT NULL,
        login_hash text,
        confirm_code text,
        CONSTRAINT user_confirm_pkey PRIMARY KEY (id),
        CONSTRAINT user_confirm_account_fkey FOREIGN KEY (account) REFERENCES user_account (id)
    );

-- 会话
CREATE TABLE
    user_scraper (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        --
        confirm uuid NOT NULL,
        api_config uuid NOT NULL,
        --
        frozen_session jsonb NOT NULL,
        in_use boolean NOT NULL DEFAULT false,
        --
        CONSTRAINT user_scraper_pkey PRIMARY KEY (id),
        CONSTRAINT user_scraper_confirm_fkey FOREIGN KEY (confirm) REFERENCES user_confirm (id),
        CONSTRAINT user_scraper_global_api_config_fkey FOREIGN KEY (api_config) REFERENCES global_api_config (id)
    );

-- 用户聊天列表
CREATE TABLE
    user_chat (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        user_scraper uuid NOT NULL,
        packed_chat jsonb NOT NULL,
        CONSTRAINT user_chat_pkey PRIMARY KEY (id),
        CONSTRAINT user_chat_user_scraper_fkey FOREIGN KEY (user_scraper) REFERENCES user_scraper (id)
    );

/* 对端数据库频道信息

可配合user_scraper访问  

也可以脱离user_scraper  
从新的user_scraper搜索频道用户名获取access_hash加入
 */
CREATE TABLE
    peer_channel (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        user_scraper uuid,
        name text NOT NULL,
        full_info jsonb,
        CONSTRAINT peer_channel_pkey PRIMARY KEY (id),
        CONSTRAINT peer_channel_user_scraper_fkey FOREIGN KEY (user_scraper) REFERENCES user_scraper (id)
    );

/* 对端数据库中的用户信息

可配合user_scraper访问  

也可以脱离user_scraper  
从新的user_scraper搜索用户名获取access_hash加入
 */
CREATE TABLE
    peer_people (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        user_scraper uuid,
        name text NOT NULL,
        full_info jsonb,
        CONSTRAINT peer_people_pkey PRIMARY KEY (id),
        CONSTRAINT peer_people_user_scraper_fkey FOREIGN KEY (user_scraper) REFERENCES user_scraper (id)
    );

-- 用户和群组的关系
CREATE TABLE
    peer_participant (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        channel uuid NOT NULL,
        people uuid NOT NULL,
        CONSTRAINT peer_participant_pkey PRIMARY KEY (id),
        CONSTRAINT peer_participant_channel_fkey FOREIGN KEY (channel) REFERENCES peer_channel (id),
        CONSTRAINT peer_participant_people_fkey FOREIGN KEY (people) REFERENCES peer_people (id)
    );

/*
对端数据库聊天记录
人或频道聊天记录
 */
CREATE TABLE
    peer_history (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        -- 来源
        user_scraper uuid NOT NULL,
        -- 以下两个字段二选一
        people uuid,
        channel uuid,
        --
        message jsonb NOT NULL, -- 参考`grammers_tl_types::enums::messages::Messages`
        --
        CONSTRAINT peer_history_pkey PRIMARY KEY (id),
        CONSTRAINT peer_history_user_scraper_fkey FOREIGN KEY (user_scraper) REFERENCES user_scraper (id),
        CONSTRAINT peer_history_people_fkey FOREIGN KEY (people) REFERENCES peer_people (id),
        CONSTRAINT peer_history_channel_fkey FOREIGN KEY (channel) REFERENCES peer_channel (id),
        CHECK (
            (
                people IS NOT NULL
                and message IS NULL
            )
            OR (
                people IS NULL
                and message IS NOT NULL
            )
        )
    );

-- 对端数据库文件下载任务
CREATE TABLE
    peer_media (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        --
        user_scraper uuid NOT NULL,
        history uuid NOT NULL,
        --
        message_media jsonb NOT NULL, -- Message的Media字段，参考`grammers_tl_types::enums::MessageMedia`
        file_type jsonb, -- 参考`grammers_tl_types::enums::storage::FileType`
        --
        --
        CONSTRAINT peer_media_pkey PRIMARY KEY (id),
        CONSTRAINT peer_media_user_scraper_fkey FOREIGN KEY (user_scraper) REFERENCES user_scraper (id),
        CONSTRAINT peer_media_history_fkey FOREIGN KEY (history) REFERENCES peer_history (id)
    );

-- 对端数据库文件内容
CREATE TABLE
    peer_file_part (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        --
        user_scraper uuid NOT NULL,
        --
        media uuid NOT NULL,
        part_offset int8 NOT NULL,
        part_len int8 NOT NULL,
        is_last_part boolean NOT NULL DEFAULT false, -- 下载完成
        next_part uuid,
        --
        data bytea,
        --
        CONSTRAINT peer_file_part_pkey PRIMARY KEY (id),
        CONSTRAINT peer_file_part_user_scraper_fkey FOREIGN KEY (user_scraper) REFERENCES user_scraper (id),
        CONSTRAINT peer_file_part_media_fkey FOREIGN KEY (media) REFERENCES peer_media (id),
        CONSTRAINT peer_file_part_next_part_fkey FOREIGN KEY (next_part) REFERENCES peer_file_part (id)
    );

/*
本质上有价值的频道数据库

可单独提供name，或提供channel表的引用
 */
CREATE TABLE
    esse_interest_channel (
        id uuid NOT NULL DEFAULT gen_random_uuid (),
        updated_at timestamptz NOT NULL DEFAULT now (),
        name text,
        channel uuid,
        CONSTRAINT esse_interest_channel_pkey PRIMARY KEY (id),
        CONSTRAINT esse_interest_channel_channel_fkey FOREIGN KEY (channel) REFERENCES peer_channel (id),
        CHECK (
            name IS NOT NULL
            OR channel IS NOT NULL
        )
    );