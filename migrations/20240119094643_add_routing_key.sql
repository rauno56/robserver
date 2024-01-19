alter table data.entity add column routing_key text;

delete from data.entity where routing_key is null;
