"use strict";

const { connect, hello } = require("./index.node");

function main() {
  try {
    console.log(hello());

    const client = connect(
      "redis://127.0.0.1",
      "test-stream",
      "test-group",
      "test-consumer"
    );
    console.log(client);
  } catch (err) {
    console.log(err);
  }
}

main();
//module.exports = compress;
