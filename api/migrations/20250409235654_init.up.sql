CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE SEX_TYPE AS ENUM ('male', 'female');
CREATE TYPE GOAL_TYPE AS ENUM ('maintain', 'lose', 'gain');
CREATE TYPE ACTIVITY_LEVEL_TYPE AS ENUM ('sedentary', 'light', 'moderate', 'active', 'very_active');
CREATE TYPE FOOD_SOURCE_TYPE AS ENUM ('USDA');

CREATE TABLE IF NOT EXISTS users (
    id                  UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    clerk_id            VARCHAR(255) NOT NULL UNIQUE,
    username            VARCHAR(100) UNIQUE,
    email               VARCHAR(255) NOT NULL UNIQUE,
    has_image           BOOL NOT NULL,
    calorie_goal        INT NULL,
    needs_setup         BOOL NOT NULL DEFAULT TRUE,
    image_url           TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_profile (
    id                  UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id             UUID NOT NULL,
    birthdate           DATE NOT NULL,
    sex                 SEX_TYPE NOT NULL,
    weight_kg           NUMERIC NOT NULL,
    height_cm           NUMERIC NOT NULL,
    activity_level      ACTIVITY_LEVEL_TYPE NOT NULL,
    goal                GOAL_TYPE NOT NULL,
    CONSTRAINT fk_user FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS foods (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name                TEXT NOT NULL,
    source              FOOD_SOURCE_TYPE NOT NULL,
    external_id         INT NOT NULL,
    fndds_code          VARCHAR(24) UNIQUE,
    wweia_category      UUID,
    CONSTRAINT fk_wweia_category FOREIGN KEY(wweia_category) REFERENCES wweia_categories(id)
);

CREATE TABLE IF NOT EXISTS wweia_categories (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code                INT NOT NULL UNIQUE,
    description         VARCHAR(255) NOT NULL
);

-- CREATE TABLE servings (
--     id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
--     name                TEXT NOT NULL,
--     food_id             UUID NOT NULL,
--     gram_weight         NUMERIC NOT NULL,
--     is_default          BOOLEAN DEFAULT false,
--     created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
--     updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
--     CONSTRAINT fk_food FOREIGN KEY(food_id) REFERENCES foods(id)
-- );

-- CREATE TABLE IF NOT EXISTS daily_logs (
--     id                  UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
--     user_id             UUID NOT NULL,
--     date                DATE NOT NULL,
--     created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
--     updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
--     CONSTRAINT fk_user FOREIGN KEY(user_id) REFERENCES users(id),
--     UNIQUE(user_id, date) -- one per day per user
-- );

-- CREATE TABLE IF NOT EXISTS food_logs (
--     id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
--     daily_log_id        UUID NOT NULL,
--     food_id             UUID NOT NULL,
--     serving_id          UUID NOT NULL,
--     quantity            NUMERIC NOT NULL,
--     created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
--     updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
--     CONSTRAINT fk_daily_log FOREIGN KEY(daily_log_id) REFERENCES daily_logs(id),
--     CONSTRAINT fk_food FOREIGN KEY(food_id) REFERENCES foods(id),
--     CONSTRAINT fk_serving FOREIGN KEY(serving_id) REFERENCES servings(id)
-- );
