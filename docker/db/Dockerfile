# PostgreSQL 18 with Apache AGE graph extension and native UUIDv7 support
#
# This image provides:
# - PostgreSQL 18 with native uuidv7() function
# - Apache AGE 1.7.0 graph extension (Cypher queries over relational data)
# - Initialization scripts to create the AGE extension on first run
#
# Based on the official Apache AGE Docker image which bundles AGE
# into the PostgreSQL 18 base image.

FROM apache/age:release_PG18_1.7.0

USER root

# 1. Create the AGE extension and set up search path
RUN cat > /docker-entrypoint-initdb.d/01-create-age-extension.sql <<'SQL'
-- Enable Apache AGE graph extension
CREATE EXTENSION IF NOT EXISTS age;
LOAD 'age';

-- Add ag_catalog to search path so Cypher functions resolve.
-- public comes first so unqualified CREATE TABLE lands in public schema.
ALTER DATABASE storyteller_development SET search_path = public, ag_catalog, "$user";
SQL

# 2. Create compatibility alias for uuid_generate_v7()
# PG18 provides native uuidv7(). This alias lets code use uuid_generate_v7()
# for compatibility with libraries that expect the uuid-ossp naming convention.
RUN echo "CREATE OR REPLACE FUNCTION uuid_generate_v7() RETURNS uuid AS \$\$ SELECT uuidv7(); \$\$ LANGUAGE SQL VOLATILE PARALLEL SAFE;" > /docker-entrypoint-initdb.d/02-uuid_v7_alias.sql

# 3. Verification script — runs on first init to confirm everything works
RUN cat > /docker-entrypoint-initdb.d/99-verify.sql <<'SQL'
-- Verify UUIDv7
SELECT uuidv7() AS test_native_uuidv7;
SELECT uuid_generate_v7() AS test_compat_uuidv7;

-- Verify AGE is installed
SELECT extname, extversion FROM pg_extension WHERE extname = 'age';

-- Verify ag_catalog is accessible
SELECT * FROM ag_catalog.ag_graph;

SELECT 'PostgreSQL 18 + AGE: ready' AS status;
SQL

USER postgres

LABEL description="PostgreSQL 18 with Apache AGE graph extension and native UUID v7 support"
LABEL maintainer="tasker-systems"
LABEL base-image="apache/age:release_PG18_1.7.0"
LABEL extensions="age"
LABEL pg-version="18"

ENV POSTGRES_DB=storyteller_development
ENV POSTGRES_USER=storyteller
ENV POSTGRES_PASSWORD=storyteller
