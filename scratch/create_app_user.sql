DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'bos_app_user') THEN
        CREATE ROLE bos_app_user WITH LOGIN PASSWORD 'AppUser@123';
    END IF;
END
$$;
GRANT CONNECT ON DATABASE test_bos_auth_db TO bos_app_user;
GRANT USAGE ON SCHEMA public TO bos_app_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO bos_app_user;
