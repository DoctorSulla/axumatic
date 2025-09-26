## axumatic
When generating new Rust projects I found myself spending a lot of
time adding similar crates and writing boilerplate so have created this repository as
a template for web applications to reduce the amount of time I spend rewriting the same thing
over and over again. This is still very much a work in progress but I intend to
make improvements to it over time as my Rust knowledge improves and I discover the pain points of my initial attempt.


# Database
For the database I am using PostgreSql. I am planning on using Neon for production instances and a Docker container for testing but any provider should slot in fairly easily.


# Email
For email I am planning on using Amazon SES but as it is just an SMTP server, any provider should be able to be used without too much difficulty.


# Users and Auth
Users are stored in the database with a hashed and salted password. I have also written but not tested most of the code required to integrate with Google as an identity provider. Sessions are created at login, stored in a separate table and managed with a session cookie which is authenticated by a middleware layer.

# Environment Variables
The following environment variables are used:
- AXUMATIC_PG_PASSWORD - This is where the password for PostgreSql is stored
- AXUMATIC_SMTP_PASSWORD - This is where the SMTP password is stored
- AXUMATIC_ENVIRONMENT - This can be PROD or TEST and will determine whether to use config.toml or test-config.toml
