alter table data.entity alter column payload drop not null;

alter table data.entity add column raw_payload text;
