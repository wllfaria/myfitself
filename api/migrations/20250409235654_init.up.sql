CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE SEX_TYPE AS ENUM (
    'male',
    'female'
);

CREATE TYPE GOAL_TYPE AS ENUM (
    'maintain',
    'lose',
    'gain'
);

CREATE TYPE ACTIVITY_LEVEL_TYPE AS ENUM (
    'sedentary',
    'light',
    'moderate',
    'active',
    'very_active'
);

CREATE OR REPLACE FUNCTION set_updated_at ()
    RETURNS TRIGGER
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS users (
    id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
    clerk_id varchar(255) NOT NULL UNIQUE,
    username varchar(100) UNIQUE,
    email varchar(255) NOT NULL UNIQUE,
    has_image bool NOT NULL,
    calorie_goal int NULL,
    needs_setup bool NOT NULL DEFAULT TRUE,
    image_url text,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_profile (
    id uuid DEFAULT uuid_generate_v4 () PRIMARY KEY,
    user_id uuid NOT NULL,
    birthdate date NOT NULL,
    sex SEX_TYPE NOT NULL,
    weight_kg numeric NOT NULL,
    height_cm numeric NOT NULL,
    activity_level ACTIVITY_LEVEL_TYPE NOT NULL,
    goal GOAL_TYPE NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE IF NOT EXISTS wweia_categories (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4 (),
    code int NOT NULL UNIQUE,
    name varchar(255) NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS nutrients (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4 (),
    name varchar(255) NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS units (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4 (),
    name varchar(255) NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS food_sources (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4 (),
    name varchar(255) NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS foods (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4 (),
    name text NOT NULL,
    source_id uuid NOT NULL,
    external_id int NOT NULL,
    fndds_code int UNIQUE,
    wweia_category uuid,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_source FOREIGN KEY (source_id) REFERENCES food_sources (id),
    CONSTRAINT fk_wweia_category FOREIGN KEY (wweia_category) REFERENCES wweia_categories (id),
    CONSTRAINT unique_source_external UNIQUE (source_id, external_id)
);

CREATE TABLE IF NOT EXISTS food_nutrients (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4 (),
    food_id uuid NOT NULL,
    nutrient_id uuid NOT NULL,
    value float4 NOT NULL,
    unit_id uuid NOT NULL,
    source_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_food FOREIGN KEY (food_id) REFERENCES foods (id),
    CONSTRAINT fk_nutrient FOREIGN KEY (nutrient_id) REFERENCES nutrients (id),
    CONSTRAINT fk_unit FOREIGN KEY (unit_id) REFERENCES units (id),
    CONSTRAINT fk_source FOREIGN KEY (source_id) REFERENCES food_sources (id),
    CONSTRAINT uq_food_nutrient UNIQUE (food_id, nutrient_id, source_id)
);

CREATE TABLE IF NOT EXISTS aggregation_metadata (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4 (),
    last_run timestamptz NOT NULL DEFAULT NOW(),
    created_at timestamptz NOT NULL DEFAULT NOW(),
    updated_at timestamptz NOT NULL DEFAULT NOW()
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
DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN
    SELECT
        table_schema,
        table_name
    FROM
        information_schema.columns
    WHERE
        column_name = 'updated_at'
        AND table_schema = 'public' LOOP
            EXECUTE format('CREATE TRIGGER trg_set_updated_at
             BEFORE UPDATE ON %I.%I
             FOR EACH ROW
             EXECUTE FUNCTION set_updated_at();', tbl.table_schema, tbl.table_name);
        END LOOP;
END;
$$;

