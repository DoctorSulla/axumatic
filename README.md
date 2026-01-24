## axumatic
When generating new Rust projects I found myself spending a lot of
time adding similar crates and writing boilerplate so have created this repository as
a template for web applications to reduce the amount of time I spend rewriting the same thing
over and over again. This is still very much a work in progress but I intend to
make improvements to it over time as my Rust knowledge improves and I discover the pain points of my initial attempt.


# Project Structure
The project is structured as a monorepo with the SvelteKit frontend stored in /frontend and the Rust backend in /src. Each project can be built separately but in production it is recommended you build into a single binary.


If you want to use a different frontend it should be pretty easy to swap out as long as you utilise something which can output static HTML.

## Key Files
- routes.rs - This file contains protected and unprotected routes. Unprotected routes can be accessed by anyone while protected routes require a valid session.
- default_route_handlers.rs - Contains all of the endpoint logic for the routes which are included by default.
- custom_route_handlers.rs - This is where you can add additional endpoints containing your application logic.
- user.rs - Contains logic for fetching users by various ids.
- auth.rs - Contains logic around cookies, sessions, codes, and registrations.
- middleware.rs - Contains the middleware which validates the user has a valid session for protected routes.
- utilities.rs - Contains various utility functions which might be used throughout the app.

# Development
## Pre-requisities
- You will need a PostgreSql instance. There is an included Docker file for creating a test database but you could also install Postgre locally or use a managed service.
- The Svelte frontend is built using PNPM so you will need this or equivalent if you want to build or run the frontend.
- Set up your environment variables (details in below section)
- Set up your test-config.toml (details in below section)


To run the frontend and the backend simply run cargo run which will start the application on the port specified in your config.


# Environment Variables
The following environment variables are used:
- AXUMATIC_PG_PASSWORD - This is where the password for PostgreSql is stored
- AXUMATIC_SMTP_PASSWORD - This is where the SMTP password is stored
- AXUMATIC_ENVIRONMENT - This can be PROD or TEST and will determine whether to use config.toml or test-config.toml

# Configuration
Various options in the server can be controlled using the config.toml (or test-config.toml for development). The options are detailed below:


## database
username - The username for connecting to the database
connection_url - The url for connecting to the database
pool_size = The pool size for the database

## email
server_url - The SMTP server url
username = The username to connect to the SMTP server
pool_size - The maximum email pool size
send_emails = Whether or not emails are actually sent. This should be true in production and can be true or false in test depending on your requirements.

## server
request_timeout - How long it will take a request to timeout in seconds.
port = - The port which the server will run on
max_unsuccessful_login_attempts - The maximum number of unsuccessful logon attempts before an account is locked.
session_length_in_days - The length a session will be valid for in days.
google_client_id - The Google client ID if you are using OAuth

# Testing
To run the tests run cargo test --features test-utils. You will need a running PostgreSql instance to run the integration tests.

# Database
For the database I am using PostgreSql.

# Email
For email I am planning on using Amazon SES but as it is just an SMTP server, any provider should be able to be used without too much difficulty.


# Users and Auth
Users are stored in the database with a hashed and salted password. I have also written but not tested most of the code required to integrate with Google as an identity provider. Sessions are created at login, stored in a separate table and managed with a session cookie which is authenticated by a middleware layer.




# Known issues
- Error handling is a bit consistent and reflective of the fact I was learning more about Rust error handling as I was going.
- Some of the errors almost certainly leak out more of the internal implementation than they should.
- Structure of the project could be better. Some stuff is in the wrong place and Default Route Handlers is getting a bit long.
- There is a lot of logic which is not covered by tests. There's a lot more cases to cover doing auth than I originally thought.
