-- Add down migration script here
drop table if EXISTS topics_junc_destinations CASCADE;
drop table if EXISTS destinations CASCADE;
DROP TYPE IF EXISTS destination_kind;
drop table if EXISTS topics CASCADE;
