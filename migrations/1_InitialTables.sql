        CREATE TABLE IF NOT EXISTS users(
                email VARCHAR(320) unique,
                email_verified boolean DEFAULT false,
                username VARCHAR(50) unique,
                hashed_password VARCHAR(255),
                login_attempts INTEGER DEFAULT 0,
                auth_level VARCHAR(30) DEFAULT 'user',
                registration_ts BIGINT,
                identity_provider VARCHAR(30),
                sub VARCHAR(256),
                PRIMARY KEY(email)
        );

        CREATE TABLE IF NOT EXISTS sessions(
            session_key VARCHAR(255),
            email VARCHAR(320) references users(email) ON DELETE CASCADE,
            expiry INTEGER,
            PRIMARY KEY(session_key)
        );

       CREATE TABLE IF NOT EXISTS codes(
            id SERIAL PRIMARY KEY,
            code_type VARCHAR(20),
            email VARCHAR(320) references users(email) ON DELETE CASCADE,
            code VARCHAR(30),
            created_ts BIGINT,
            expiry_ts BIGINT,
            used BOOLEAN default false
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_email ON sessions(email);
        CREATE INDEX IF NOT EXISTS idx_codes_email ON codes(email);
