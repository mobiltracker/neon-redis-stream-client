"use strict";
const streamFFI = require("./index.node");

type Message = { key: string } & Object;

class RedisStreamClient {
  private clientRef: any;
  private connectionString: string;
  private groupName: string;
  private consumerPrefix: string;
  private streamKey: string;

  constructor(
    connectionString: string,
    streamKey: string,
    groupName: string,
    consumerPrefix: string
  ) {
    this.connectionString = connectionString;
    this.streamKey = streamKey;
    this.groupName = groupName;
    this.consumerPrefix = consumerPrefix;
    this.clientRef = null;
  }

  connect() {
    // //"redis://127.0.0.1",
    // "test-stream",
    // "test-group",
    // "test-consumer"
    const client = streamFFI.connect(
      this.connectionString,
      this.streamKey,
      this.groupName,
      this.consumerPrefix
    );

    this.clientRef = client;
  }

  async readNext() {
    const msg: Message = await streamFFI.readNext(this.clientRef);
    return msg;
  }

  async ackMessageId(msgId: string) {
    return await streamFFI.ackMessage(this.clientRef, msgId);
  }
}

module.exports = { RedisStreamClient };
