import { connect } from "https://deno.land/x/amqp/mod.ts";

const connection = await connect();
const channel = await connection.openChannel();

const EX = Deno.args[0]?.split(',') ?? [
  'amq.fanout',
  'amq.headers',
  'amq.topic',
];

let x = 50_000;
console.log('publishing', x, 'messages to', EX);

const randomInt = (max = 20) => (Math.random() * max) >> 0;
const randomObject = () => {
  return Object.fromEntries(
    (new Array(3))
      .fill(null)
      .map(() => {
        return [`prop${randomInt(4)}`, randomInt(100)];
      })
  );
};

// for (const ex of EX) {
//     await channel.declareExchange(ex);
// }

while (x > 0) {
  if (x % 1000 === 0) {
    console.log('to publish', x);
  }
  for (const exchange of EX) {
    const payload = JSON.stringify({ foo: "bar", ...randomObject() });
    x--;
    await channel.publish(
      { exchange, routingKey: 'dasds' },
      { contentType: "application/json" },
      new TextEncoder().encode(payload),
    );
    if (x <= 0) {
      break;
    }
  }
}
console.log('to publish', x);

await connection.close();
