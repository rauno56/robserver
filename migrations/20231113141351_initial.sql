-- Add migration script here

create schema data;

create table data.entity (
	id numeric not null,
	created_at timestamptz not null default now(),
	last_seen_at timestamptz not null default now(),
	vhost text not null,
	exchange text not null,
	count integer not null default 1,
	payload jsonb not null,
	primary key (id, vhost, exchange)
);

-- create or replace rule "data.entity_insert" AS ON
-- insert to data.entity
--      where exists (select 1 from data.entity where (new.id, new.vhost, new.exchange) = (id, vhost, exchange))
--     do instead
-- update data.entity
--     set
--       count = count + 1,
--       last_seen_at = now()
--     where (new.id, new.vhost, new.exchange) = (id, vhost, exchange);
