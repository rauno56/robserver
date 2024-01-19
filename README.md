# Robserver

An observer for a **R**abbitMQ server. Listens to specified exchanges(`direct` exchanges not supported), counts different *shapes* of JSON payloads and stores that information to a PostgreSQL database.

## Running

There must be an accessable RabbitMQ and PostgreSQL server. If you'd just like to test things out, run them in containers:

```bash
podman run --rm -d --name robserver-db -v `pwd`/migrations/:/docker-entrypoint-initdb.d/:z -e POSTGRES_DB=robserver -e POSTGRES_HOST_AUTH_METHOD=trust -p 5432:5432 postgres
podman run --rm -d --name robserver-mq -p 15672:15672 -p 5672:5672 rabbitmq:3.10.7-management
```

Then if you have `cargo` installed locally:

```bash
export ROBSERVER_PG_ADDR="postgres://postgres@127.0.0.1/robserver"
export ROBSERVER_AMQP_ADDR="amqp://guest:guest@127.0.0.1:5672/%2f"

cargo run
```

Or run it as a container(the image is under 100mb):

```bash
podman run --rm -it --name robserver --network host -e ROBSERVER_PG_ADDR="postgres://postgres@127.0.0.1/robserver" -e ROBSERVER_AMQP_ADDR="amqp://guest:guest@127.0.0.1:5672/%2f" ghcr.io/rauno56/robserver:latest
```

## Running migrations

Above example uses a feature build into postgres docker images to run migrations on startup. If you don't have that possibility, you must run the migrations before starting robserver service. If you have `cargo` installed locally:

```bash
export DATABASE_URL="postgres://postgres@127.0.0.1/robserver"

cargo sqlx database create
cargo sqlx migrate run
```

... if not, you can use a prebuild docker image:

```bash
podman run --rm -it --name robserver-migration --network host -e ROBSERVER_PG_ADDR="postgres://postgres@127.0.0.1/robserver" ghcr.io/rauno56/robserver:latest-migration
```

## Configuration

Configuration is done through environment variables

#### MQ

- `ROBSERVER_BUFFER_SIZE`: number of payloads held in the memory at once. If the payloads are really big, you might want to decrease that. Defaults to `10_000`.
- `ROBSERVER_AMQP_ADDR`: connection string for the RabbitMQ server. Defaults to `amqp://guest:guest@127.0.0.1:5672/%2f`.
- `ROBSERVER_AMQP_DEF_ADDR`: API definition endpoint to discover and auto-bind to all exchanges. Defaults to `http://guest:guest@127.0.0.1:15672/api/definitions`.
- `ROBSERVER_LISTEN_EX`: comma-separated list of exchanges to observe. Defaults to `amq.direct,amq.fanout,amq.headers,amq.topic`.
- `ROBSERVER_PREFETCH`: AMQP prefetch setting. Defaults to `100`.
- `ROBSERVER_QUEUE`: queue to create and bind exchanges to. Defaults to `robserver.messages`.
- `ROBSERVER_QUEUE_MAX_LENGTH`: when `robserver` spins up it will create non-durable autodeleted queue to consume the payloads from. This is `x-max-length` property of that queue. If the number of queued payloads gets to that level, any unconsumed payloads will be dropped to make room for new. Defaults to `100_000`.

#### DB

- `ROBSERVER_PG_ADDR`: connection string for the PostgreSQL server. Defaults to `postgres://postgres@127.0.0.1/robserver`.
- `ROBSERVER_MAX_QUERY_SIZE`: maximum number of payloads taken from the internal buffer to be processed and stored. Making it bigger than the buffer size has no effect. Defaults to `1000`.

## JSON payload shape

Observed payloads are grouped together and regarded as the same payload based on the keys. Values are never considered. To illustrate:

```
1. { a: 1, b: 1 }
2. { a: { b: 2 } } <-- different from (1) "b" is nested under "a", not sibling.
3. { b: 3, a: 3 } <-- same as (1) Order of the properties does not matter.
4. { a: 4, b: null } <-- same as (1) Values are ignored.
5. { a: 5, b: [{ c: 5 }] } <-- same as (1) Arrays are values and not traversed into.
6. { a: 6 } <-- different from all of the above. Lacks "b".
```

## Produced data

Robserver will create a table `entity` within a `data` schema with following columns:

- `id`: `numeric` - a numeric representation of the payload shape
- `created_at`: `timestamptz` - timestamp for when this shape of payload was first seen
- `last_seen_at`: `timestamptz` - timestamp for when this shape of payload was last seen
- `vhost`: `text` - vhost observed (`TODO` currently `/` is assumed)
- `exchange`: `text` - name of the exchange the payload shape was observed on
- `count`: `integer` - number of times the payload shape was observed for
- `payload`: `jsonb` - first occurrence of the payload
