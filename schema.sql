--
-- PostgreSQL database dump
--

SET statement_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SET check_function_bodies = false;
SET client_min_messages = warning;

--
-- Name: plpgsql; Type: EXTENSION; Schema: -; Owner: 
--

CREATE EXTENSION IF NOT EXISTS plpgsql WITH SCHEMA pg_catalog;


--
-- Name: EXTENSION plpgsql; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION plpgsql IS 'PL/pgSQL procedural language';


SET search_path = public, pg_catalog;

--
-- Name: diesel_manage_updated_at(regclass); Type: FUNCTION; Schema: public; Owner: disease
--

CREATE FUNCTION diesel_manage_updated_at(_tbl regclass) RETURNS void
    LANGUAGE plpgsql
    AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE diesel_set_updated_at()', _tbl);
END;
$$;


ALTER FUNCTION public.diesel_manage_updated_at(_tbl regclass) OWNER TO disease;

--
-- Name: diesel_set_updated_at(); Type: FUNCTION; Schema: public; Owner: disease
--

CREATE FUNCTION diesel_set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.diesel_set_updated_at() OWNER TO disease;

SET default_tablespace = '';

SET default_with_oids = false;

--
-- Name: __diesel_schema_migrations; Type: TABLE; Schema: public; Owner: disease; Tablespace: 
--

CREATE TABLE __diesel_schema_migrations (
    version character varying(50) NOT NULL,
    run_on timestamp without time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.__diesel_schema_migrations OWNER TO disease;

--
-- Name: engagements; Type: TABLE; Schema: public; Owner: disease; Tablespace: 
--

CREATE TABLE engagements (
    playeruid bigint NOT NULL,
    zombieuid bigint NOT NULL,
    "timestamp" bigint NOT NULL,
    active integer DEFAULT 0 NOT NULL,
    accepted integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.engagements OWNER TO disease;

--
-- Name: items; Type: TABLE; Schema: public; Owner: disease; Tablespace: 
--

CREATE TABLE items (
    itemuid bigint NOT NULL,
    owneruid bigint NOT NULL,
    itemtype integer NOT NULL,
    "timestamp" bigint NOT NULL,
    lat real NOT NULL,
    lon real NOT NULL
);


ALTER TABLE public.items OWNER TO disease;

--
-- Name: locations; Type: TABLE; Schema: public; Owner: disease; Tablespace: 
--

CREATE TABLE locations (
    uid bigint NOT NULL,
    "timestamp" bigint NOT NULL,
    lat real NOT NULL,
    lon real NOT NULL,
    health integer DEFAULT 0 NOT NULL,
    appstate integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.locations OWNER TO disease;

--
-- Name: player_engagements; Type: TABLE; Schema: public; Owner: disease; Tablespace: 
--

CREATE TABLE player_engagements (
    player1uid bigint NOT NULL,
    player2uid bigint NOT NULL,
    "timestamp" bigint NOT NULL,
    active integer DEFAULT 0 NOT NULL,
    state integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.player_engagements OWNER TO disease;

--
-- Name: player_info; Type: TABLE; Schema: public; Owner: disease; Tablespace: 
--

CREATE TABLE player_info (
    playeruid bigint NOT NULL,
    xp integer NOT NULL,
    health integer DEFAULT 100 NOT NULL
);


ALTER TABLE public.player_info OWNER TO disease;

--
-- Name: zombie_locations; Type: TABLE; Schema: public; Owner: disease; Tablespace: 
--

CREATE TABLE zombie_locations (
    uid bigint NOT NULL,
    "timestamp" bigint NOT NULL,
    lat real NOT NULL,
    lon real NOT NULL,
    health integer DEFAULT 0 NOT NULL,
    bearing real DEFAULT 0.0 NOT NULL
);


ALTER TABLE public.zombie_locations OWNER TO disease;

--
-- Name: __diesel_schema_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: disease; Tablespace: 
--

ALTER TABLE ONLY __diesel_schema_migrations
    ADD CONSTRAINT __diesel_schema_migrations_pkey PRIMARY KEY (version);


--
-- Name: engagements_pkey; Type: CONSTRAINT; Schema: public; Owner: disease; Tablespace: 
--

ALTER TABLE ONLY engagements
    ADD CONSTRAINT engagements_pkey PRIMARY KEY (playeruid, zombieuid, "timestamp");


--
-- Name: items_pkey; Type: CONSTRAINT; Schema: public; Owner: disease; Tablespace: 
--

ALTER TABLE ONLY items
    ADD CONSTRAINT items_pkey PRIMARY KEY (itemuid);


--
-- Name: locations_pkey; Type: CONSTRAINT; Schema: public; Owner: disease; Tablespace: 
--

ALTER TABLE ONLY locations
    ADD CONSTRAINT locations_pkey PRIMARY KEY (uid, "timestamp");


--
-- Name: player_engagements_pkey; Type: CONSTRAINT; Schema: public; Owner: disease; Tablespace: 
--

ALTER TABLE ONLY player_engagements
    ADD CONSTRAINT player_engagements_pkey PRIMARY KEY (player1uid, player2uid, "timestamp");


--
-- Name: player_info_pkey; Type: CONSTRAINT; Schema: public; Owner: disease; Tablespace: 
--

ALTER TABLE ONLY player_info
    ADD CONSTRAINT player_info_pkey PRIMARY KEY (playeruid);


--
-- Name: zombie_locations_pkey; Type: CONSTRAINT; Schema: public; Owner: disease; Tablespace: 
--

ALTER TABLE ONLY zombie_locations
    ADD CONSTRAINT zombie_locations_pkey PRIMARY KEY (uid, "timestamp");


--
-- Name: public; Type: ACL; Schema: -; Owner: postgres
--

REVOKE ALL ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON SCHEMA public FROM postgres;
GRANT ALL ON SCHEMA public TO postgres;
GRANT ALL ON SCHEMA public TO PUBLIC;


--
-- PostgreSQL database dump complete
--

