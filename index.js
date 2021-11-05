"use strict";

const { connect, hello, readNext } = require("./index.node");

function main() {
  try {
    console.log(hello());

    // Client is a raw pointer to the redus stream client
    const client = connect(
      "redis://127.0.0.1",
      "test-stream",
      "test-group",
      "test-consumer"
    );

    readNext(client)
      .then((msg) => {
        console.log(msg);
      })
      .catch((err) => console.log(err));
  } catch (err) {
    console.log(err);
  }
}

main();
//module.exports = compress;
